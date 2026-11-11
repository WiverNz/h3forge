//! End-to-end map link flow through the plain session state (H3F-021, no
//! Tauri runtime): sample save → bounded discovery → parsed map → hero
//! enrichment → recovered portraits. Opt-in: needs `H3FORGE_FIXTURE_ROOT`
//! (absolute path of the folder with `SaAndVla.GM2`) plus the repository's
//! private `Data/` and `Maps/` folders; skips with a message otherwise.
//! Originals are only read; copies go to `.local/fixtures` or the OS
//! temporary directory.

use std::path::{Path, PathBuf};

use h3forge_desktop::session::{AppState, PortraitSize};

const SAMPLE_MAP_FILE: &str = "[HotA] The Mysterious Island - v1-5-j - ALLIES.h3m";

fn repo(relative: &str) -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(relative)
        .canonicalize()
        .ok()
}

fn sample_save() -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from)?;
    let source = root.join("SaAndVla.GM2");
    source.is_file().then_some(source)
}

fn find_sample_map(maps: &Path) -> Option<PathBuf> {
    h3forge_map::list_map_files(maps, h3forge_map::ScanLimits::default())
        .into_iter()
        .find(|path| path.file_name().is_some_and(|name| name == SAMPLE_MAP_FILE))
}

struct Fixture {
    data: PathBuf,
    save: PathBuf,
    map: PathBuf,
    maps: PathBuf,
}

/// `tag` keeps the per-test save copies apart (tests run in parallel).
fn fixture(tag: &str) -> Option<Fixture> {
    let data = repo("Data")?;
    let maps = repo("Maps")?;
    let map = find_sample_map(&maps)?;
    let source = sample_save()?;
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.local/fixtures");
    std::fs::create_dir_all(&fixtures).ok()?;
    let save = fixtures.join(format!("SaAndVla-map-link-{tag}.GM2"));
    std::fs::copy(&source, &save).ok()?;
    Some(Fixture {
        data,
        save,
        map,
        maps,
    })
}

fn open(state: &mut AppState, fixture: &Fixture, save: &Path) {
    state
        .set_manual_installation(fixture.data.clone())
        .expect("installation validates");
    let grant = state.issue_grant(save.to_path_buf());
    state
        .open_granted_save(&grant.grant_id)
        .expect("sample opens");
}

#[test]
fn sample_save_finds_its_map_and_enriches_heroes() {
    let Some(fixture) = fixture("match") else {
        eprintln!("no private sample save/map/data; skipping map link flow test");
        return;
    };
    let mut state = AppState::new();
    open(&mut state, &fixture, &fixture.save);

    // Discovery ran on open, through the installation's Maps folder, and
    // matched by internal name: the file name never mentions "COOP".
    let link = state.map_link().expect("link available");
    assert_eq!(link.status, "matched", "{}", link.message);
    assert!(link.scanned_files > 200, "scanned {}", link.scanned_files);
    let map = link.map.expect("map linked");
    assert_eq!(map.file_name, SAMPLE_MAP_FILE);
    assert_eq!(map.match_strength, "full");
    assert_eq!(map.name.lossy, "- The Mysterious Island - COOP");
    assert_eq!(map.format, "HotA (format 10)");
    assert_eq!(map.hota_release.as_deref(), Some("1.8.1"));
    assert_eq!((map.size, map.has_underground), (252, true));
    assert_eq!((map.playable_players, map.human_players), (8, 6));
    assert_eq!(map.towns, Some(37));
    assert_eq!((map.placed_heroes, map.random_heroes), (Some(58), Some(6)));
    assert_eq!(map.body_error, None);
    // Every placed hero type has exactly one save record of that type.
    assert_eq!(map.heroes_placed_in_save, 58);
    // Wherever the map names a portrait for a hero type, the save agrees.
    eprintln!(
        "portrait IDs checked against the map: {} (consistent: {})",
        map.portraits_checked, map.portraits_consistent
    );
    assert_eq!(map.portraits_checked, 67);
    assert_eq!(map.portraits_checked, map.portraits_consistent);

    // Hero enrichment is keyed by hero type and kept apart from the save's
    // own values.
    let kenoby = state.get_hero(28).expect("record 28");
    let info = kenoby.map.expect("map info");
    assert_eq!(info.placement, "prison");
    assert_eq!(
        (info.x, info.y, info.level),
        (Some(184), Some(123), Some(0))
    );
    assert_eq!(info.map_portrait_id, Some(213));
    assert_eq!(info.portrait_matches, Some(true));
    assert_eq!(info.map_name.unwrap().lossy, "Ben Kenoby");
    let unplaced = state.get_hero(0).expect("record 0").map.expect("map info");
    assert_eq!(unplaced.placement, "not-placed");
    assert_eq!(unplaced.x, None);

    // Portraits: all 215 records resolve through the name tables, including
    // the 84 with IDs above 136 that used to be placeholders.
    let ids: Vec<u8> = state
        .session_ref()
        .unwrap()
        .document
        .heroes
        .iter()
        .map(|hero| hero.portrait.expect("identity").portrait_id)
        .collect();
    let mut formerly_missing = 0;
    let mut recovered = 0;
    let mut unresolved = Vec::new();
    for (record, portrait_id) in ids.iter().enumerate() {
        let portrait = state
            .hero_portrait(record, PortraitSize::Large)
            .expect("portrait command");
        if *portrait_id > 136 {
            formerly_missing += 1;
            recovered += portrait.resolved as usize;
        }
        if !portrait.resolved {
            unresolved.push((record, *portrait_id, portrait.reason));
        }
    }
    eprintln!("formerly missing portraits: {formerly_missing}, recovered: {recovered}");
    assert_eq!(formerly_missing, 84);
    assert_eq!(recovered, 84, "unresolved: {unresolved:?}");
    assert!(unresolved.is_empty(), "unresolved: {unresolved:?}");
    // Distinct portrait IDs produce distinct images (no shared fallback).
    let image = |state: &mut AppState, record| {
        state
            .hero_portrait(record, PortraitSize::Large)
            .unwrap()
            .png_base64
            .unwrap()
    };
    assert_ne!(image(&mut state, 28), image(&mut state, 156));
    assert_ne!(image(&mut state, 0), image(&mut state, 145));
}

#[test]
fn duplicates_need_a_choice_and_other_maps_are_refused() {
    let Some(fixture) = fixture("choice") else {
        eprintln!("no private sample save/map/data; skipping map ambiguity test");
        return;
    };
    // A private game-like layout in the OS temp dir: games/ beside Maps/,
    // with the same map present twice under different file names.
    let root = std::env::temp_dir().join(format!("h3forge-map-link-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("games")).unwrap();
    std::fs::create_dir_all(root.join("Maps/copies")).unwrap();
    let save = root.join("games/SaAndVla.GM2");
    std::fs::copy(&fixture.save, &save).unwrap();
    std::fs::copy(&fixture.map, root.join("Maps/island.h3m")).unwrap();
    std::fs::copy(&fixture.map, root.join("Maps/copies/renamed.h3m")).unwrap();

    let mut state = AppState::new();
    open(&mut state, &fixture, &save);
    let link = state.map_link().unwrap();
    assert_eq!(link.status, "ambiguous", "{}", link.message);
    assert!(link.map.is_none(), "nothing is linked without a choice");
    assert!(link.candidates.len() >= 2);
    assert!(state.get_hero(28).unwrap().map.is_none());

    // A different map is refused and leaves the state untouched.
    let other = h3forge_map::list_map_files(&fixture.maps, h3forge_map::ScanLimits::default())
        .into_iter()
        .find(|path| path.file_name().is_some_and(|name| name != SAMPLE_MAP_FILE))
        .expect("another map exists");
    let refused = state.set_manual_map(&other).unwrap_err();
    assert_eq!(refused.code, "map-failed");
    assert!(
        refused.message.contains("different map"),
        "{}",
        refused.message
    );
    assert_eq!(state.map_link().unwrap().status, "ambiguous");

    // Choosing a listed candidate links it.
    let chosen = state.select_map_candidate(0).unwrap();
    assert_eq!(chosen.status, "manual");
    assert_eq!(chosen.map.unwrap().match_strength, "full");
    assert!(state.get_hero(28).unwrap().map.is_some());
    assert!(state.select_map_candidate(99).is_err());

    std::fs::remove_dir_all(&root).unwrap();
}
