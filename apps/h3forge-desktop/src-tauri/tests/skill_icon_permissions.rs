#[test]
fn skill_icon_command_is_exposed_to_the_main_window() {
    let build = include_str!("../build.rs");
    let main = include_str!("../src/main.rs");
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/main.json")).unwrap();
    assert!(build.contains("\"get_skill_icon\""));
    assert!(main.contains("commands::get_skill_icon,"));
    assert!(
        capability["windows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "main")
    );
    assert!(
        capability["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "allow-get-skill-icon")
    );
}
