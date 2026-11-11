//! Opt-in check against the private reference map. Set `H3FORGE_MAPS_ROOT`
//! to the absolute path of a `Maps` folder holding
//! `[HotA] The Mysterious Island - v1-5-j - ALLIES.h3m` (any sub-folder up
//! to the discovery depth). Skips with a message otherwise; the map is only
//! read, never copied or modified.

use h3forge_map::{
    MapFormat, MapHeroKind, MapLimits, MatchOutcome, MatchStrength, SaveMapKey, ScanLimits,
    list_map_files, match_candidates, read_map, read_map_header,
};

const SAMPLE_NAME: &[u8] = b"- The Mysterious Island - COOP";

#[test]
fn sample_map_is_discovered_and_parsed_to_the_end() {
    let Some(root) = std::env::var_os("H3FORGE_MAPS_ROOT").map(std::path::PathBuf::from) else {
        eprintln!("H3FORGE_MAPS_ROOT not set; skipping sample map test");
        return;
    };
    let files = list_map_files(&root, ScanLimits::default());
    let mut headers = Vec::new();
    let mut paths = Vec::new();
    for file in &files {
        if let Ok(header) = read_map_header(file, MapLimits::default(), ScanLimits::default()) {
            headers.push(header);
            paths.push(file.clone());
        }
    }
    // The values a HotA 44/11 save of this map retains (size, levels,
    // difficulty); the description is left to the header comparison.
    let key = SaveMapKey {
        name_raw: SAMPLE_NAME,
        description_raw: None,
        size: Some(252),
        has_underground: Some(true),
        difficulty: Some(3),
    };
    let MatchOutcome::Unique { index, strength } = match_candidates(&key, headers.iter()) else {
        eprintln!("sample map not present under H3FORGE_MAPS_ROOT; skipping");
        return;
    };
    assert_eq!(strength, MatchStrength::NameAndDimensions);
    // Matched by internal name: the file name does not contain "COOP".
    let file_name = paths[index].file_name().unwrap().to_string_lossy();
    assert!(!file_name.contains("COOP"), "{file_name}");

    let map = read_map(&paths[index], MapLimits::default()).expect("sample map reads");
    let header = &map.header;
    assert_eq!(header.format, MapFormat::HornOfTheAbyss { subrevision: 10 });
    assert_eq!(header.hota_release, Some((1, 8, 1)));
    assert_eq!((header.size, header.has_underground), (252, true));
    assert_eq!(header.description_raw.len(), 295);
    assert_eq!(header.players.iter().filter(|p| p.is_playable()).count(), 8);
    assert_eq!(
        header.players.iter().filter(|p| p.can_human_play).count(),
        6
    );
    assert_eq!(header.disposed_heroes.len(), 214);
    let main = header.players[0].main_hero.as_ref().expect("Red main hero");
    assert_eq!((main.hero_type, main.portrait), (150, Some(161)));

    let body = map.body.expect("sample body parses");
    // Stock editors end every map with 124 padding bytes; reaching exactly
    // that point proves every object was stepped over with the right width.
    assert_eq!(body.trailing_bytes, 124);
    assert_eq!(body.object_count, 37_696);
    assert_eq!(body.towns.len(), 37);
    assert_eq!(body.heroes.len(), 64);
    let random = body
        .heroes
        .iter()
        .filter(|hero| hero.kind == MapHeroKind::Random)
        .count();
    assert_eq!(random, 6);
    // Every placed hero type is unique, so the type is a usable identity.
    let mut types: Vec<u8> = body
        .heroes
        .iter()
        .filter_map(|hero| hero.hero_type)
        .collect();
    types.sort_unstable();
    let before = types.len();
    types.dedup();
    assert_eq!((before, types.len()), (58, 58));
    let kenoby = body
        .heroes
        .iter()
        .find(|hero| hero.hero_type == Some(28))
        .expect("hero type 28 is placed");
    assert_eq!(kenoby.kind, MapHeroKind::Prison);
    assert_eq!(kenoby.portrait, Some(213));
    assert_eq!(kenoby.name_raw.as_deref(), Some(&b"Ben Kenoby"[..]));
    assert_eq!(
        (kenoby.tile().x, kenoby.tile().y, kenoby.tile().z),
        (184, 123, 0)
    );
}
