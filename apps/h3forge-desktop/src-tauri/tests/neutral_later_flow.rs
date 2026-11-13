//! Application-level loading and pending edits for the later SaAndVla save.
use h3forge_desktop::{dto::ArmyEditDto, session::AppState};

#[test]
fn later_creatures_load_edit_and_reopen() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping later creatures session: H3FORGE_FIXTURE_ROOT unset");
        return;
    };
    let original_path = std::path::PathBuf::from(root).join("SaAndVla.GM2");
    let original = std::fs::read(&original_path).unwrap();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../.local/fixtures/neutral-later-flow-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source.GM2");
    std::fs::write(&source, &original).unwrap();
    let mut state = AppState::new();
    let grant = state.issue_grant(source.clone());
    state.open_granted_save(&grant.grant_id).unwrap();
    let rows = state.list_neutral_armies().unwrap();
    assert_eq!(rows.unavailable_reason, None);
    assert_eq!(rows.groups.len(), 515);
    let group = rows.groups.iter().find(|g| g.kind == "wandering").unwrap();
    let slot = &group.army.as_ref().unwrap()[0];
    let edit = || ArmyEditDto {
        slot: 0,
        creature_id: Some(slot.creature_id),
        count: slot.count + 1,
    };
    state.apply_neutral_edit(group.id, edit()).unwrap();
    assert_eq!(state.pending_edits().unwrap().len(), 1);
    state.discard_neutral_edits(group.id).unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    state.apply_neutral_edit(group.id, edit()).unwrap();
    let destination = dir.join("edited.GM2");
    state.save_save_as(&destination).unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    let grant = state.issue_grant(destination);
    state.open_granted_save(&grant.grant_id).unwrap();
    let after = state.list_neutral_armies().unwrap();
    assert_eq!(after.groups.len(), 515);
    assert_eq!(
        after
            .groups
            .iter()
            .find(|g| g.id == group.id)
            .unwrap()
            .army
            .as_ref()
            .unwrap()[0]
            .count,
        slot.count + 1
    );
    assert_eq!(std::fs::read(source).unwrap(), original);
    assert_eq!(std::fs::read(original_path).unwrap(), original);
}
