//! Every command registered in the invoke handler must also be declared in
//! the build manifest and allowed for the main window. A command missing from
//! either list compiles and passes session-level tests but is refused at
//! runtime with "<command> not allowed" (the H3F-013 town commands shipped
//! that way once).

/// Command names registered in `generate_handler!`.
fn registered_commands() -> Vec<String> {
    include_str!("../src/main.rs")
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("h3forge_desktop::commands::")
                .map(|rest| rest.trim_end_matches(',').to_owned())
        })
        .collect()
}

#[test]
fn every_registered_command_is_declared_and_allowed() {
    let build = include_str!("../build.rs");
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/main.json")).expect("capability JSON");
    let permissions: Vec<&str> = capability["permissions"]
        .as_array()
        .expect("permissions array")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    let commands = registered_commands();
    assert!(commands.len() >= 29, "handler list was not parsed");
    for command in &commands {
        assert!(
            build.contains(&format!("\"{command}\"")),
            "{command} is missing from the build.rs app manifest"
        );
        let permission = format!("allow-{}", command.replace('_', "-"));
        assert!(
            permissions.contains(&permission.as_str()),
            "{permission} is missing from capabilities/main.json"
        );
    }
}

#[test]
fn town_commands_are_exposed_to_the_main_window() {
    let commands = registered_commands();
    for command in [
        "list_towns",
        "get_town",
        "apply_town_edit",
        "get_town_edit_state",
        "discard_town_edits",
    ] {
        assert!(commands.iter().any(|name| name == command), "{command}");
    }
}
