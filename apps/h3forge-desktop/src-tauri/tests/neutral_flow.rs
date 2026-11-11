//! Copy-only map army persistence and revert regression (H3F-027).
use h3forge_desktop::{dto::ArmyEditDto, session::AppState};
use h3forge_save::SaveDocument;

fn edit(slot: u8, creature_id: Option<u32>, count: u32) -> ArmyEditDto {
    ArmyEditDto {
        slot,
        creature_id,
        count,
    }
}

#[test]
fn neutral_session_revert_discard_save_and_save_as() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping neutral session fixture: H3FORGE_FIXTURE_ROOT unset");
        return;
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../.local/fixtures/neutral-flow-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let original = std::fs::read(std::path::PathBuf::from(root).join("SaAndVla.GM2")).unwrap();
    let source = dir.join("source.GM2");
    std::fs::write(&source, &original).unwrap();
    let mut state = AppState::new();
    let grant = state.issue_grant(source.clone());
    state.open_granted_save(&grant.grant_id).unwrap();
    let rows = state.list_neutral_armies().unwrap();
    assert_eq!(rows.groups.len(), 338);
    let g = rows
        .groups
        .iter()
        .find(|g| g.position == [158, 35, 1])
        .unwrap();
    let monster = rows.groups.iter().find(|g| g.kind == "wandering").unwrap();
    assert!(
        state
            .apply_neutral_edit(monster.id, edit(0, None, 0))
            .is_err()
    );
    state
        .apply_neutral_edit(g.id, edit(0, Some(13), 1234))
        .unwrap();
    assert_eq!(state.pending_edits().unwrap().len(), 1);
    state.discard_neutral_edits(g.id).unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    state
        .apply_neutral_edit(g.id, edit(0, Some(13), 1234))
        .unwrap();
    state.discard_all_edits().unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    state
        .apply_neutral_edit(g.id, edit(0, Some(13), 1234))
        .unwrap();
    state.apply_neutral_edit(g.id, edit(6, None, 0)).unwrap();
    let destination = dir.join("edited.GM2");
    state.save_save_as(&destination).unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let reopened = SaveDocument::open(std::fs::read(&destination).unwrap()).unwrap();
    let armies = reopened.neutral_armies();
    let army = armies
        .groups
        .iter()
        .find(|a| a.id == g.id)
        .unwrap()
        .army
        .unwrap();
    assert_eq!((army[0].creature_id, army[0].count), (13, 1234));
    assert!(army[6].is_empty());
    state
        .apply_neutral_edit(g.id, edit(6, Some(0), 42))
        .unwrap();
    state.save().unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    let rows = state.list_neutral_armies().unwrap();
    assert_eq!(
        rows.groups
            .iter()
            .find(|a| a.id == g.id)
            .unwrap()
            .army
            .as_ref()
            .unwrap()[6]
            .count,
        42
    );
    assert!(
        dir.read_dir()
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().contains(".backup-"))
    );
    // The same persistence path now carries the single wandering stack.
    let monster = state
        .list_neutral_armies()
        .unwrap()
        .groups
        .into_iter()
        .find(|g| {
            g.kind == "wandering"
                && g.read_only_reason.is_none()
                && g.allowed_creatures
                    .iter()
                    .any(|&id| Some(id) != g.creature_id)
        })
        .unwrap();
    let target = *monster
        .allowed_creatures
        .iter()
        .find(|&&id| Some(id) != monster.creature_id)
        .unwrap();
    let rows = state
        .apply_neutral_edit(monster.id, edit(0, Some(target), 123))
        .unwrap();
    let pending = rows.groups.iter().find(|g| g.id == monster.id).unwrap();
    assert_eq!(pending.creature_id, Some(target));
    assert_eq!(pending.army.as_ref().unwrap().len(), 1);
    assert_eq!(pending.army.as_ref().unwrap()[0].count, 123);
    state.discard_neutral_edits(monster.id).unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    state
        .apply_neutral_edit(monster.id, edit(0, Some(target), 123))
        .unwrap();
    state.save().unwrap();
    let rows = state.list_neutral_armies().unwrap();
    let saved = rows.groups.iter().find(|g| g.id == monster.id).unwrap();
    assert_eq!(
        (saved.creature_id, saved.army.as_ref().unwrap()[0].count),
        (Some(target), 123)
    );
    assert!(state.pending_edits().unwrap().is_empty());
}
