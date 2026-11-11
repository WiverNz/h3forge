//! H3F-023: private-copy map/save enrichment, original artwork and preservation.
use h3forge_desktop::{player_details::describe, session::AppState};
use h3forge_save::SaveDocument;

#[test]
fn verified_player_facts_masks_artwork_and_safe_resource_roundtrip() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping private player details: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let dir = repo.join(format!(
        ".local/fixtures/player-details-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let source = std::fs::read(std::path::PathBuf::from(root).join("SaAndVla.GM2")).unwrap();
    let map_source =
        std::fs::read(repo.join("Maps/New/[HotA] The Mysterious Island - v1-5-j - ALLIES.h3m"))
            .unwrap();
    let save = dir.join("source.GM2");
    let map = dir.join("source.h3m");
    std::fs::write(&save, &source).unwrap();
    std::fs::write(&map, &map_source).unwrap();
    let document = SaveDocument::open(source.clone()).unwrap();
    let absent = describe(&document, None, 0);
    assert!(!absent.map_linked);
    assert!(absent.eligibility.is_none() && absent.team.is_none() && absent.allies.is_none());
    assert!(absent.obelisks.is_none() && absent.visited_obelisks.is_none());
    let mut state = AppState::new();
    state.set_manual_installation(repo.join("Data")).unwrap();
    let grant = state.issue_grant(save.clone());
    state.open_granted_save(&grant.grant_id).unwrap();
    state.set_manual_map(&map).unwrap();
    let players = state.list_players().unwrap();
    let expected_teams = [1, 2, 3, 3, 4, 2, 4, 5];
    let expected_visits = [0, 5, 15, 15, 17, 5, 17, 1];
    for (color, p) in players.iter().enumerate() {
        assert_eq!(
            p.details.eligibility,
            Some(if color == 0 || color == 7 {
                "Computer"
            } else {
                "Human or Computer"
            })
        );
        assert!(p.details.controller.is_none());
        assert_eq!(p.details.team, Some(expected_teams[color]));
        let allies: Vec<u8> = (0..8)
            .filter(|&c| c != color && expected_teams[c] == expected_teams[color])
            .map(|c| c as u8)
            .collect();
        assert_eq!(p.details.allies.as_ref().unwrap(), &allies);
        assert_eq!(p.details.enemies.as_ref().unwrap().len(), 7 - allies.len());
        assert_eq!(p.details.map_size, Some(252));
        assert_eq!(p.details.levels, Some(2));
        assert_eq!(p.details.obelisks.as_ref().unwrap().len(), 29);
        assert_eq!(p.details.visited_obelisks, Some(expected_visits[color]));
        assert_eq!(p.details.grail_placements, Some(1));
        assert!(p.details.puzzle_progress.is_none());
        assert!(p.details.reveal_map_reason.starts_with("Disabled:"));
        assert!(p.details.reveal_puzzle_reason.starts_with("Disabled:"));
        assert!(state.player_flag(color as u32).unwrap().resolved);
    }
    assert!(!state.player_flag(8).unwrap().resolved);
    for (color, faction, hero, name) in [(0, 3, 150, "Night King"), (7, 9, 165, "John Silver")] {
        let d = &players[color].details;
        assert_eq!(d.starting_town.as_ref().unwrap().faction, Some(faction));
        assert!(state.town_icon(faction, false, false).unwrap().resolved);
        assert_eq!(d.starting_hero.as_ref().unwrap().hero_type, hero);
        assert_eq!(
            d.starting_hero
                .as_ref()
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .lossy,
            name
        );
        assert!(state.player_starting_portrait(color).unwrap().resolved);
    }
    for color in 1..7 {
        assert!(
            players[color]
                .details
                .starting_town
                .as_ref()
                .unwrap()
                .faction
                .is_none()
        );
        assert!(players[color].details.starting_hero.is_none());
        assert!(!state.player_starting_portrait(color).unwrap().resolved);
    }
    let masks = document.obelisk_visit_masks().unwrap();
    assert_eq!(masks.len(), 29);
    for (a, b) in [(1, 5), (2, 3), (4, 6)] {
        assert!(
            masks
                .iter()
                .all(|m| (m & (1 << a) != 0) == (m & (1 << b) != 0))
        );
    }
    let mut malformed = document.payload().to_vec();
    malformed[4634945] = 49;
    assert!(
        SaveDocument::open(h3forge_save::compress_gzip(&malformed))
            .unwrap()
            .obelisk_visit_masks()
            .is_none()
    );
    malformed = document.payload().to_vec();
    malformed[4634946 + 29] = 1;
    assert!(
        SaveDocument::open(h3forge_save::compress_gzip(&malformed))
            .unwrap()
            .obelisk_visit_masks()
            .is_none()
    );
    assert!(state.pending_edits().unwrap().is_empty());
    assert_eq!(std::fs::read(&save).unwrap(), source);
    assert_eq!(std::fs::read(&map).unwrap(), map_source);
    // No reveal edit is offered. Existing resource semantics and rollback stay intact.
    state.apply_player_edit(4, 6, 1234567).unwrap();
    state.discard_player_edits(4).unwrap();
    assert!(state.pending_edits().unwrap().is_empty());
    state.apply_player_edit(6, 0, 0).unwrap();
    state.discard_all_edits().unwrap();
    assert_eq!(state.list_players().unwrap()[6].resources[0], 760);
    state.apply_player_edit(4, 6, 1234567).unwrap();
    let dest = dir.join("edited.GM2");
    state.save_save_as(&dest).unwrap();
    let reopened = SaveDocument::open(std::fs::read(&dest).unwrap()).unwrap();
    assert_eq!(reopened.players[4].resources[6], 1234567);
    let diff: Vec<_> = document
        .payload()
        .iter()
        .zip(reopened.payload())
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert_eq!(diff, [4635712, 4635713, 4635714]);
    assert_eq!(reopened.obelisk_visit_masks().unwrap(), masks);
    assert_eq!(std::fs::read(&map).unwrap(), map_source);
    println!(
        "H3F-023: 8 flags, 2 fixed town icons, 2 starting portraits; 29 obelisks; resource-only diff {diff:?}; exploration bytes unchanged"
    );
}
