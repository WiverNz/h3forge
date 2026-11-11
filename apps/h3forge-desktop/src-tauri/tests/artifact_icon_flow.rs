use h3forge_desktop::session::AppState;

#[test]
fn artifact_icon_command_is_exposed_to_main_window() {
    let build = include_str!("../build.rs");
    let main = include_str!("../src/main.rs");
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/main.json")).unwrap();
    assert!(build.contains("\"get_artifact_icon\""));
    assert!(main.contains("commands::get_artifact_icon,"));
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
            .any(|v| v == "allow-get-artifact-icon")
    );
}

#[test]
fn selected_installation_axe_png_response_and_smoke_fixture() {
    let Some(root) = std::env::var_os("H3FORGE_ICON_INSTALLATION") else {
        eprintln!("skipping: set H3FORGE_ICON_INSTALLATION for local resource/session check");
        return;
    };
    let mut state = AppState::new();
    state.set_manual_installation(root.into()).unwrap();
    let icon = state.artifact_icon(7).unwrap();
    assert!(icon.resolved, "{:?}", icon.reason);
    let json = serde_json::to_value(&icon).unwrap();
    let png = json["pngBase64"].as_str().unwrap();
    // Base64 PNG signature and 44x44 IHDR prefix; same DTO used by IPC.
    assert!(png.starts_with("iVBORw0KGgoAAAANSUhEUgAAACwAAAAs"));
    assert_eq!(state.artifact_icon(7).unwrap().png_base64, icon.png_base64);
    let missing = state.artifact_icon(166).unwrap();
    assert!(!missing.resolved);
    assert!(missing.reason.is_some());
    eprintln!(
        "Selected installation: ID 7 resolved; PNG base64 length {}",
        png.len()
    );

    // A disposable native-UI fixture. Never write to the original save directory.
    if let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") {
        use h3forge_save::{
            BackpackEdit, BackpackItem, EquipmentEdit, EquipmentItem, HeroConfidence, HeroEdit,
            SaveDocument,
        };
        let document = SaveDocument::open(
            std::fs::read(std::path::PathBuf::from(root).join("SaAndVla.GM2")).unwrap(),
        )
        .unwrap();
        let hero = document
            .heroes
            .iter()
            .find(|h| {
                h.confidence == HeroConfidence::High
                    && h.equipment_gating.is_clear()
                    && h.equipment_items[3] == EquipmentItem::Empty
                    && h.backpack[0] == BackpackItem::Empty
            })
            .unwrap();
        let plan = document
            .plan_hero_edits(
                hero.id,
                &[
                    HeroEdit::SetEquipmentSlot {
                        slot: 3,
                        item: EquipmentEdit::Artifact(7),
                    },
                    HeroEdit::SetBackpackSlot {
                        slot: 0,
                        item: BackpackEdit::Artifact(7),
                    },
                ],
            )
            .unwrap();
        let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.local/fixtures/artifact-icon-smoke.GM2");
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        std::fs::write(&output, document.serialize_with_patches(&plan).unwrap()).unwrap();
        eprintln!("Smoke fixture hero {} at {:?}", hero.id, output);
    }
}
