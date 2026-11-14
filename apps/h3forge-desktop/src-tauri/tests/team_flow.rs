//! H3F-030 team moves through the application session on private copies:
//! pending state, option gating, Revert, Discard, Save As, Save and reopen.
use h3forge_desktop::session::AppState;
use h3forge_save::SaveDocument;

#[test]
fn team_moves_are_pending_checked_and_saved_as_single_bytes() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping team flow: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = repo.join(format!(".local/fixtures/team-flow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // (save, assignment offset, Blue's original one-based team)
    for (name, offset, blue) in [
        ("same_teams_restart", 475usize, 1u8),
        ("separate_team_map_2", 477, 2),
    ] {
        let original =
            std::fs::read(std::path::PathBuf::from(&root).join(format!("{name}.GM2"))).unwrap();
        let source = dir.join(format!("{name}.GM2"));
        std::fs::write(&source, &original).unwrap();
        let doc = SaveDocument::open(original.clone()).unwrap();
        let other = 3 - blue;

        let mut state = AppState::new();
        let grant = state.issue_grant(source.clone());
        state.open_granted_save(&grant.grant_id).unwrap();
        let players = state.list_players().unwrap();
        assert_eq!(players[1].team, Some(blue));
        assert_eq!(players[1].original_team, Some(blue));
        assert_eq!(players[1].team_state, "editable");
        assert_eq!(players[1].team_count, 2);
        // Red is alone only in the second sample and then cannot leave.
        assert_eq!(
            players[0].team_options,
            if blue == 1 { vec![1, 2] } else { vec![1] }
        );
        assert_eq!(players[3].team, None);
        assert_eq!(players[3].team_state, "absent");
        assert!(players[3].team_options.is_empty());

        // Refused requests leave no pending state.
        assert!(state.apply_team_edit(3, 1).is_err());
        assert!(state.apply_team_edit(1, 3).is_err());
        assert!(state.apply_team_edit(1, 0).is_err());
        assert!(state.apply_team_edit(8, 1).is_err());
        if blue == 2 {
            assert!(state.apply_team_edit(0, 2).is_err());
        }
        assert!(state.pending_edits().unwrap().is_empty());

        let players = state.apply_team_edit(1, other).unwrap();
        assert_eq!(players[1].team, Some(other));
        assert_eq!(players[1].original_team, Some(blue));
        assert!(players[1].team_dirty && !players[0].team_dirty);
        let pending = state.pending_edits().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(
            (pending[0].field.as_str(), pending[0].before.as_str()),
            ("Team", blue.to_string().as_str())
        );
        assert_eq!(pending[0].after, other.to_string());
        // Selecting the original team again clears the pending move.
        state.apply_team_edit(1, blue).unwrap();
        assert!(state.pending_edits().unwrap().is_empty());
        state.apply_team_edit(1, other).unwrap();
        state.discard_player_edits(1).unwrap();
        assert!(state.pending_edits().unwrap().is_empty());
        state.apply_team_edit(1, other).unwrap();
        state.discard_all_edits().unwrap();
        assert!(state.pending_edits().unwrap().is_empty());

        state.apply_team_edit(1, other).unwrap();
        let dest = dir.join(format!("{name}-teams.GM2"));
        state.save_save_as(&dest).unwrap();
        let next = SaveDocument::open_strict(std::fs::read(&dest).unwrap()).unwrap();
        let differing: Vec<usize> = (0..doc.payload().len())
            .filter(|&i| next.payload()[i] != doc.payload()[i])
            .collect();
        assert_eq!(differing, [offset + 1]);
        assert_eq!(next.payload()[offset + 1], other - 1);
        assert_eq!(next.players, doc.players);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(state.pending_edits().unwrap().is_empty());
        let players = state.list_players().unwrap();
        assert_eq!(players[1].team, Some(other));
        assert_eq!(players[1].original_team, Some(other));
        assert!(!players[1].team_dirty);

        // Direct Save of the disposable copy moves Blue back.
        state.apply_team_edit(1, blue).unwrap();
        state.save().unwrap();
        let back = SaveDocument::open_strict(std::fs::read(&dest).unwrap()).unwrap();
        assert_eq!(back.payload(), doc.payload());
        eprintln!("{name}: team gates, pending, Revert, Discard, Save As and Save passed");
    }
}
