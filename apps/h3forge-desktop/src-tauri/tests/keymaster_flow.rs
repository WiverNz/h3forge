//! Copy-only Keymaster workflow including colliding color bytes across players.
use h3forge_desktop::session::AppState;
use h3forge_save::SaveDocument;
#[test]
fn tents_pending_revert_discard_save_and_artwork() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping keymaster flow: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = repo.join(".local/fixtures/keymaster-flow");
    std::fs::create_dir_all(&dir).unwrap();
    let original_path = std::path::PathBuf::from(root).join("SaAndVla_before_visit_red_tent.GM2");
    let original = std::fs::read(&original_path).unwrap();
    let source = dir.join("source.GM2");
    std::fs::write(&source, &original).unwrap();
    let mut state = AppState::new();
    let grant = state.issue_grant(source.clone());
    state.open_granted_save(&grant.grant_id).unwrap();
    assert!(!state.list_players().unwrap()[7].tent_visits.unwrap()[2]);
    state.apply_tent_edit(7, 2, true).unwrap();
    assert_eq!(state.list_players().unwrap()[7].dirty_tents, [2]);
    assert_eq!(state.pending_edits().unwrap().len(), 1);
    state.apply_tent_edit(7, 2, false).unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    state.apply_tent_edit(7, 2, true).unwrap();
    state.apply_tent_edit(0, 2, false).unwrap();
    state.discard_player_edits(7).unwrap();
    assert_eq!(state.pending_edits().unwrap().len(), 1);
    assert!(!state.list_players().unwrap()[7].tent_visits.unwrap()[2]);
    state.discard_all_edits().unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    state.apply_tent_edit(7, 2, true).unwrap();
    state.apply_tent_edit(0, 2, false).unwrap();
    state.apply_player_edit(1, 0, 123).unwrap();
    assert!(state.apply_tent_edit(8, 2, true).is_err());
    let dest = dir.join("edited.GM2");
    state.save_save_as(&dest).unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    let saved = SaveDocument::open(std::fs::read(&dest).unwrap()).unwrap();
    assert_eq!(saved.keymaster_masks().unwrap()[2], 128);
    assert_eq!(saved.players[1].resources[0], 123);
    assert_eq!(std::fs::read(&source).unwrap(), original);
    state.apply_tent_edit(0, 2, true).unwrap();
    state.save().unwrap();
    assert_eq!(
        SaveDocument::open(std::fs::read(&dest).unwrap())
            .unwrap()
            .keymaster_masks()
            .unwrap()[2],
        129
    );
    state.apply_tent_edit(7, 2, false).unwrap();
    std::fs::write(&dest, &original).unwrap();
    assert!(state.save().is_err());
    assert_eq!(state.pending_edits().unwrap().len(), 1);
    state.set_manual_installation(repo.join("Data")).unwrap();
    for color in 0..8 {
        let icon = state.keymaster_icon(color).unwrap();
        assert!(icon.png_base64.is_some());
        assert_eq!(
            state.keymaster_icon(color).unwrap().png_base64,
            icon.png_base64
        );
    }
    assert!(state.keymaster_icon(8).unwrap().png_base64.is_none());
    assert_eq!(std::fs::read(original_path).unwrap(), original);
}
