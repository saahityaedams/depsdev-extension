use toml::Value;
use zed_extension_api::{
    self as zed,
    http_client::{HttpMethod, HttpRequest, RedirectPolicy},
    serde_json::{self, json},
    SlashCommand, SlashCommandOutput, SlashCommandOutputSection, Worktree,
};

struct DepsDevExtension;

impl zed::Extension for DepsDevExtension {
    fn new() -> Self {
        DepsDevExtension
    }

    fn complete_slash_command_argument(
        &self,
        command: SlashCommand,
        _args: Vec<String>,
    ) -> Result<Vec<zed_extension_api::SlashCommandArgumentCompletion>, String> {
        match command.name.as_str() {
            "depsdev-dump" => Ok(vec![]),
            command => Err(format!("unknown slash command: \"{command}\"")),
        }
    }

    fn run_slash_command(
        &self,
        command: SlashCommand,
        _args: Vec<String>,
        _worktree: Option<&Worktree>,
    ) -> Result<SlashCommandOutput, String> {
        match command.name.as_str() {
            "depsdev-dump" => {
                let deps_file = _worktree.unwrap().read_text_file("Cargo.toml").unwrap();
                let deps = get_rust_deps(deps_file);

                let requests: Vec<serde_json::Value> = deps
                    .into_iter()
                    .map(|(name, version)| {
                        json!({
                            "versionKey": {
                                "system": "CARGO",
                                "name": name,
                                "version": version
                            }
                        })
                    })
                    .collect();

                // Prepare the request
                let request = HttpRequest {
                    method: HttpMethod::Post,
                    url: "https://api.deps.dev/v3alpha/versionbatch".to_string(),
                    headers: vec![],
                    body: Some(
                        serde_json::to_vec(&json!({
                            "requests": json!(requests)
                        }))
                        .unwrap(),
                    ),
                    redirect_policy: RedirectPolicy::FollowAll,
                };

                // Make the HTTP request
                match zed::http_client::fetch(&request) {
                    Ok(response) => {
                        // Convert ASCII codes to a string
                        let json_string: String = response
                            .body
                            .iter()
                            .map(|&code| char::from(code as u8))
                            .collect();

                        // Parse the string as JSON
                        let json_value: Value = serde_json::from_str(&json_string).unwrap();
                        let formatted_json =
                            format!("{}", serde_json::to_string_pretty(&json_value).unwrap());
                        let formatted_json_len = formatted_json.len();
                        Ok(zed::SlashCommandOutput {
                            text: formatted_json,
                            sections: vec![SlashCommandOutputSection {
                                range: (0..formatted_json_len).into(),
                                label: "DepsDev Dump".to_string(),
                            }],
                        })
                    }
                    Err(e) => Ok(zed::SlashCommandOutput {
                        text: format!("API request failed. Error: {}.", e),
                        sections: vec![],
                    }),
                }
            }
            command => Err(format!("unknown slash command: \"{command}\"")),
        }
    }
}

fn get_rust_deps(file_contents: String) -> Vec<(String, String)> {
    let cargo_toml: Value = file_contents.parse().unwrap();
    let mut deps: Vec<(String, String)> = vec![];

    if let Some(dependencies) = cargo_toml.get("workspace").unwrap().get("dependencies") {
        if let Some(deps_table) = dependencies.as_table() {
            for (name, version) in deps_table {
                match version {
                    Value::String(_v) => {
                        deps.push((name.to_string(), "".to_string()));
                    }
                    Value::Table(t) => {
                        if let Some(v) = t.get("version") {
                            if let Some(version_str) = v.as_str() {
                                deps.push((name.to_string(), version_str.to_string()));
                            }
                        }
                    }
                    _ => {
                        deps.push((name.to_string(), "".to_string()));
                    }
                }
            }
        }
    } else {
        eprintln!("No dependencies found in Cargo.toml");
    }

    return deps;
}

zed::register_extension!(DepsDevExtension);
