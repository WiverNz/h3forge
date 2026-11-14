//! Native localized town names must not hide the preceding player table.
use h3forge_desktop::session::AppState;
use h3forge_save::{PlayerEdit, SaveDocument, TownConfidence};

#[test]
fn native_localized_towns_and_every_player_resource_roundtrip() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping native player fixtures: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = repo.join(format!(
        ".local/fixtures/native-players-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    for name in [
        "same_teams",
        "same_teams_restart",
        "separate_teams",
        "separate_team_map_2",
    ] {
        let original =
            std::fs::read(std::path::PathBuf::from(&root).join(format!("{name}.GM2"))).unwrap();
        let source = dir.join(format!("{name}.GM2"));
        std::fs::write(&source, &original).unwrap();
        let doc = SaveDocument::open(std::fs::read(&source).unwrap()).unwrap();
        assert_eq!(doc.towns.len(), 6);
        assert!(
            doc.towns
                .iter()
                .all(|t| t.confidence == TownConfidence::High)
        );
        assert!(
            doc.towns
                .windows(2)
                .all(|w| w[0].span.end == w[1].span.start)
        );
        assert_eq!(doc.players.len(), 8);
        assert!(doc.players.iter().all(|p| p.read_only_reason.is_none()));
        assert_eq!(
            doc.towns
                .iter()
                .map(|t| (t.x, t.y, t.z))
                .collect::<Vec<_>>(),
            [
                (6, 7, 0),
                (65, 7, 0),
                (64, 65, 0),
                (7, 65, 0),
                (10, 64, 1),
                (65, 64, 1)
            ]
        );
        // All 56 fields independently, including currently unused colors.
        // Verify the documented word boundary independently of returned patches.
        let table = doc.towns[0].span.start - 2 - 8 * 149;
        for color in 0..8 {
            for resource in 0..7 {
                let value = i64::from(doc.players[color].resources[resource]) + 17;
                let patches = doc
                    .plan_player_edits(
                        color,
                        &[PlayerEdit::SetResource {
                            resource: resource as u8,
                            value,
                        }],
                    )
                    .unwrap();
                let next = SaveDocument::open_strict(doc.serialize_with_patches(&patches).unwrap())
                    .unwrap();
                let at = table + color * 149 + 98 + resource * 4;
                assert_eq!(&next.payload()[..at], &doc.payload()[..at]);
                assert_eq!(&next.payload()[at + 4..], &doc.payload()[at + 4..]);
                assert_eq!(&next.payload()[at..at + 4], &(value as i32).to_le_bytes());
                assert_eq!(next.players[color].resources[resource], value as i32);
                for other in (0..8).filter(|&c| c != color) {
                    assert_eq!(next.players[other], doc.players[other]);
                }
            }
        }
        // Structural corruption still closes the player-table write gate.
        let mut ambiguous = doc.payload().to_vec();
        ambiguous[doc.towns[0].span.start - 2] = 5;
        let rejected = SaveDocument::open(h3forge_save::compress_gzip(&ambiguous)).unwrap();
        assert!(rejected.players.is_empty());
        assert!(
            rejected
                .plan_player_edits(
                    0,
                    &[PlayerEdit::SetResource {
                        resource: 6,
                        value: 9
                    }]
                )
                .is_err()
        );

        let mut state = AppState::new();
        let grant = state.issue_grant(source.clone());
        state.open_granted_save(&grant.grant_id).unwrap();
        assert_eq!(state.list_players().unwrap().len(), 8);
        state.apply_player_edit(0, 6, 12345).unwrap();
        state.apply_player_edit(1, 6, 23456).unwrap();
        state.discard_player_edits(0).unwrap();
        assert_eq!(state.pending_edits().unwrap().len(), 1);
        assert_eq!(
            state.list_players().unwrap()[0].resources,
            doc.players[0].resources
        );
        state.discard_all_edits().unwrap();
        assert!(state.pending_edits().unwrap().is_empty());
        state.apply_player_edit(1, 6, 23456).unwrap();
        let dest = dir.join(format!("{name}-resources.GM2"));
        state.save_save_as(&dest).unwrap();
        let saved = std::fs::read(&dest).unwrap();
        let next = SaveDocument::open_strict(saved.clone()).unwrap();
        let patches = doc
            .plan_player_edits(
                1,
                &[PlayerEdit::SetResource {
                    resource: 6,
                    value: 23456,
                }],
            )
            .unwrap();
        let mut expected = doc.payload().to_vec();
        h3forge_save::apply_patches(&mut expected, &patches).unwrap();
        assert_eq!(next.payload(), expected);
        assert!(state.pending_edits().unwrap().is_empty());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(
            dir.read_dir().unwrap().filter_map(Result::ok).any(|e| e
                .file_name()
                .to_string_lossy()
                .contains(".backup-")
                && std::fs::read(e.path()).unwrap() == original)
        );
        state.apply_player_edit(0, 0, 9876).unwrap();
        state.save().unwrap();
        let next = SaveDocument::open_strict(std::fs::read(&dest).unwrap()).unwrap();
        assert_eq!(next.players[0].resources[0], 9876);
        assert_eq!(next.players[1].resources[6], 23456);
        // External changes to the disposable destination still block Save.
        state.apply_player_edit(0, 0, 123).unwrap();
        std::fs::write(&dest, &saved).unwrap();
        assert!(state.save().is_err());
        assert_eq!(std::fs::read(&dest).unwrap(), saved);
        eprintln!(
            "{name}: 6 verified towns, 8 editable player slots, 56 isolated resource roundtrips, Revert/Discard/Save As/Save/backup/source guard passed"
        );
    }
}
