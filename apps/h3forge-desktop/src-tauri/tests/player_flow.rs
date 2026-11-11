//! Copy-only player edits through the same persistence path used by the UI.
use h3forge_desktop::session::AppState;
use h3forge_save::SaveDocument;

#[test]
fn player_session_revert_discard_save_as_save_and_source_guard() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping private player flow: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../.local/fixtures/player-flow-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let original = std::fs::read(std::path::PathBuf::from(root).join("SaAndVla.GM2")).unwrap();
    let source = dir.join("source.GM2");
    std::fs::write(&source, &original).unwrap();
    let mut state = AppState::new();
    let grant = state.issue_grant(source.clone());
    state.open_granted_save(&grant.grant_id).unwrap();
    let base = state.list_players().unwrap();
    assert_eq!(base.len(), 8);
    assert_eq!(base[1].status, "unknown");
    assert!(base[1].read_only_reason.is_none());
    state.apply_player_edit(1, 6, 1234567).unwrap();
    state.apply_player_edit(6, 0, 765).unwrap();
    assert_eq!(state.pending_edits().unwrap().len(), 2);
    assert_eq!(state.list_players().unwrap()[1].resources[6], 1234567);
    assert!(state.apply_player_edit(1, 6, -1).is_err());
    assert!(state.apply_player_edit(1, 6, 2147483648).is_err());
    assert!(state.apply_player_edit(8, 6, 1).is_err());
    assert_eq!(state.pending_edits().unwrap().len(), 2);
    state.discard_player_edits(1).unwrap();
    assert_eq!(
        state.list_players().unwrap()[1].resources,
        base[1].resources
    );
    assert_eq!(state.pending_edits().unwrap().len(), 1);
    state
        .apply_player_edit(6, 0, base[6].resources[0] as i64)
        .unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    state.apply_player_edit(0, 0, 0).unwrap();
    state.apply_player_edit(7, 6, 42).unwrap();
    state.discard_all_edits().unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    for (actual, expected) in state.list_players().unwrap().iter().zip(&base) {
        assert_eq!(actual.resources, expected.resources);
    }
    state.apply_player_edit(1, 6, 2147483647).unwrap();
    state.apply_player_edit(6, 0, 0).unwrap();
    let dest = dir.join("edited.GM2");
    state.save_save_as(&dest).unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert!(state.pending_edits().unwrap().is_empty());
    let output = std::fs::read(&dest).unwrap();
    let doc = SaveDocument::open(output.clone()).unwrap();
    assert_eq!(doc.players[1].resources[6], i32::MAX);
    assert_eq!(doc.players[6].resources[0], 0);
    let before = SaveDocument::open(original.clone()).unwrap();
    let changed: Vec<_> = before
        .payload()
        .iter()
        .zip(doc.payload())
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert_eq!(
        changed,
        [4635265, 4635266, 4635267, 4635268, 4635986, 4635987]
    );
    assert!(
        dir.read_dir().unwrap().filter_map(Result::ok).any(|e| e
            .file_name()
            .to_string_lossy()
            .contains(".backup-")
            && std::fs::read(e.path()).unwrap() == original)
    );
    state.apply_player_edit(1, 6, 1234567).unwrap();
    state.save().unwrap();
    assert_eq!(state.list_players().unwrap()[1].original[6], 1234567);
    assert!(state.pending_edits().unwrap().is_empty());
    assert!(
        dir.read_dir().unwrap().filter_map(Result::ok).any(|e| e
            .file_name()
            .to_string_lossy()
            .contains(".backup-")
            && std::fs::read(e.path()).unwrap() == output)
    );
    // Simulate an external writer only on our generated fixture.
    state.apply_player_edit(1, 6, 7654321).unwrap();
    std::fs::write(&dest, &original).unwrap();
    assert!(state.save().is_err());
    assert_eq!(state.pending_edits().unwrap().len(), 1);
    assert_eq!(std::fs::read(&dest).unwrap(), original);
}
