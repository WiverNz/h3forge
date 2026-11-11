//! Synthetic map tests: hand-built payloads, no game data. They cover the
//! header reader, object stepping for hero/town objects, typed failures on
//! malformed input, bounded discovery and the matching rules.

use h3forge_map::{
    MapError, MapFormat, MapHeroKind, MapLimits, MatchOutcome, MatchStrength, SaveMapKey,
    ScanLimits, decompress_map, decompress_prefix, list_map_files, match_candidates, parse_header,
    parse_map, read_map_header,
};

fn text(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

/// Shadow of Death header: eight closed player slots except Red, who has a
/// customized main hero; one disposed hero.
fn sod_header(name: &[u8], description: &[u8], size: u32, underground: bool) -> Vec<u8> {
    let mut out = 0x1Cu32.to_le_bytes().to_vec();
    out.push(1);
    out.extend_from_slice(&size.to_le_bytes());
    out.push(underground as u8);
    text(&mut out, name);
    text(&mut out, description);
    out.push(2); // difficulty
    out.push(0); // level limit
    for player in 0..8 {
        out.extend_from_slice(&[(player == 0) as u8, 0, 0, 0]); // human, AI, tactic, selectable
        out.extend_from_slice(&[0xFF, 0x01, 0]); // factions (2 bytes), random faction
        out.push(0); // no main town
        out.push(0); // random hero
        if player == 0 {
            out.push(5); // main hero type
            out.push(200); // portrait
            text(&mut out, b"Lead");
        } else {
            out.push(0xFF);
        }
        out.push(0); // placeholder count
        out.extend_from_slice(&0u32.to_le_bytes()); // hero names
    }
    out.push(0xFF); // victory: standard
    out.push(0xFF); // loss: standard
    out.push(0); // no teams
    out.extend_from_slice(&[0xFF; 20]); // allowed heroes
    out.extend_from_slice(&0u32.to_le_bytes()); // placeholders
    out.push(1); // disposed heroes
    out.extend_from_slice(&[9, 0xFF]); // type 9, default portrait
    text(&mut out, b"Spare");
    out.push(0x01);
    out
}

fn template(out: &mut Vec<u8>, class: u32, subclass: u32) {
    text(out, b"obj.def");
    out.extend_from_slice(&[0; 16]);
    out.extend_from_slice(&class.to_le_bytes());
    out.extend_from_slice(&subclass.to_le_bytes());
    out.extend_from_slice(&[0; 18]);
}

fn object_start(out: &mut Vec<u8>, x: u8, y: u8, z: u8, template: u32) {
    out.extend_from_slice(&[x, y, z]);
    out.extend_from_slice(&template.to_le_bytes());
    out.extend_from_slice(&[0; 5]);
}

/// A complete 2x2 SoD map with one sign, one hero and one town.
fn sod_map(name: &[u8]) -> Vec<u8> {
    let mut out = sod_header(name, b"desc", 2, false);
    out.extend_from_slice(&[0; 31]); // reserved
    out.extend_from_slice(&[0; 18]); // allowed artifacts
    out.extend_from_slice(&[0; 13]); // allowed spells + skills
    out.extend_from_slice(&0u32.to_le_bytes()); // rumors
    out.extend_from_slice(&[0; 156]); // predefined heroes: none customized
    out.extend_from_slice(&[0; 2 * 2 * 7]); // terrain
    out.extend_from_slice(&3u32.to_le_bytes());
    template(&mut out, 91, 0); // sign
    template(&mut out, 34, 0); // hero
    template(&mut out, 98, 3); // town, faction 3
    out.extend_from_slice(&3u32.to_le_bytes());

    object_start(&mut out, 1, 1, 0, 0);
    text(&mut out, b"hello");
    out.extend_from_slice(&[0; 4]);

    object_start(&mut out, 1, 0, 0, 1);
    out.extend_from_slice(&7u32.to_le_bytes()); // identifier
    out.extend_from_slice(&[0, 5]); // owner Red, hero type 5
    out.push(1);
    text(&mut out, b"Lead"); // custom name
    out.push(0); // no experience
    out.extend_from_slice(&[1, 200]); // custom portrait 200
    out.extend_from_slice(&[0, 0]); // no skills, no garrison
    out.push(0); // formation
    out.push(0); // no artifacts
    out.push(0xFF); // patrol
    out.extend_from_slice(&[0, 0xFF]); // no biography, gender
    out.extend_from_slice(&[0, 0]); // no spells, no primary skills
    out.extend_from_slice(&[0; 16]);

    object_start(&mut out, 1, 1, 0, 2);
    out.extend_from_slice(&9u32.to_le_bytes()); // identifier
    out.push(0xFF); // neutral
    out.extend_from_slice(&[0, 0]); // no name, no garrison
    out.push(0); // formation
    out.extend_from_slice(&[0, 1]); // default buildings, has fort
    out.extend_from_slice(&[0; 18]); // spells
    out.extend_from_slice(&0u32.to_le_bytes()); // events
    out.extend_from_slice(&[0xFF, 0, 0, 0]); // alignment + padding

    out.extend_from_slice(&0u32.to_le_bytes()); // global events
    out
}

fn gzip_stored(payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1F, 0x8B, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xFF];
    for (index, chunk) in payload.chunks(60_000).enumerate() {
        let last = (index + 1) * 60_000 >= payload.len();
        out.push(last as u8);
        out.extend_from_slice(&(chunk.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(chunk.len() as u16)).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&[0; 4]); // CRC is advisory for maps
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out
}

#[test]
fn header_fields_and_custom_heroes_are_read() {
    let payload = sod_header(b"Test Map", b"About", 72, true);
    let header = parse_header(&payload, MapLimits::default()).unwrap();
    assert_eq!(header.format, MapFormat::ShadowOfDeath);
    assert_eq!((header.size, header.has_underground), (72, true));
    assert_eq!(header.name_raw, b"Test Map");
    assert_eq!(header.description_raw, b"About");
    assert_eq!(header.difficulty, 2);
    let main = header.players[0].main_hero.as_ref().unwrap();
    assert_eq!((main.hero_type, main.portrait), (5, Some(200)));
    assert!(header.players[0].is_playable() && !header.players[1].is_playable());
    assert_eq!(header.disposed_heroes.len(), 1);
    assert_eq!(header.disposed_heroes[0].portrait, None);
}

#[test]
fn player_teams_reject_invalid_ids_and_allow_unused_slots() {
    let mut payload = sod_header(b"Teams", b"", 72, false);
    let at = payload
        .windows(23)
        .position(|w| w[..3] == [255, 255, 0] && w[3..] == [255; 20])
        .unwrap()
        + 2;
    payload.splice(at..at + 1, [1, 0, 255, 255, 255, 255, 255, 255, 255]);
    let header = parse_header(&payload, MapLimits::default()).unwrap();
    assert_eq!(header.players[0].team, Some(0));
    assert!(header.players[1..].iter().all(|p| p.team.is_none()));
    payload[at + 1] = 1;
    assert!(parse_header(&payload, MapLimits::default()).is_err());
    payload[at] = 9;
    assert!(parse_header(&payload, MapLimits::default()).is_err());
}

#[test]
fn obelisk_anchors_and_grail_radius_are_read_only_and_bounded() {
    let mut payload = sod_header(b"Puzzle", b"", 2, true);
    payload.extend_from_slice(&[0; 31 + 18 + 13 + 4 + 156]);
    payload.extend_from_slice(&[0; 2 * 2 * 2 * 7]);
    payload.extend_from_slice(&2u32.to_le_bytes());
    template(&mut payload, 57, 0);
    template(&mut payload, 36, 0);
    payload.extend_from_slice(&2u32.to_le_bytes());
    let object_at = payload.len();
    object_start(&mut payload, 1, 1, 1, 0);
    object_start(&mut payload, 0, 1, 0, 1);
    payload.extend_from_slice(&7u32.to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());
    let original = payload.clone();
    let body = parse_map(&payload, MapLimits::default())
        .unwrap()
        .body
        .unwrap();
    assert_eq!(
        body.obelisks,
        [(0, h3forge_map::Position { x: 1, y: 1, z: 1 })]
    );
    assert_eq!(
        body.grails,
        [(h3forge_map::Position { x: 0, y: 1, z: 0 }, 7)]
    );
    assert_eq!(body.trailing_bytes, 0);
    assert_eq!(payload, original);
    payload[object_at] = 2;
    assert!(
        parse_map(&payload, MapLimits::default())
            .unwrap()
            .body
            .is_err()
    );
    assert!(
        parse_map(&original[..original.len() - 5], MapLimits::default())
            .unwrap()
            .body
            .is_err()
    );
}

#[test]
fn body_yields_heroes_and_towns_and_is_consumed_exactly() {
    let map = parse_map(&sod_map(b"Tiny"), MapLimits::default()).unwrap();
    let body = map.body.expect("body parses");
    assert_eq!(body.trailing_bytes, 0);
    assert_eq!(body.object_count, 3);
    assert_eq!(body.object_classes, vec![(34, 1), (91, 1), (98, 1)]);
    let hero = &body.heroes[0];
    assert_eq!(hero.kind, MapHeroKind::Hero);
    assert_eq!((hero.hero_type, hero.owner), (Some(5), Some(0)));
    assert_eq!(hero.portrait, Some(200));
    assert_eq!(hero.name_raw.as_deref(), Some(&b"Lead"[..]));
    assert_eq!((hero.tile().x, hero.tile().y), (0, 0));
    let town = &body.towns[0];
    assert_eq!(
        (town.faction, town.owner, town.identifier),
        (Some(3), None, Some(9))
    );
}

#[test]
fn malformed_input_fails_with_typed_errors() {
    let limits = MapLimits::default();
    assert!(matches!(
        parse_header(&0x99u32.to_le_bytes(), limits),
        Err(MapError::UnknownFormat { version: 0x99 })
    ));
    let payload = sod_header(b"Cut", b"", 36, false);
    assert!(matches!(
        parse_header(&payload[..40], limits),
        Err(MapError::Truncated { .. })
    ));
    // The header survives a body that ends early or names a bad template.
    let full = sod_map(b"Tiny");
    let cut = parse_map(&full[..full.len() - 30], limits).unwrap();
    assert_eq!(cut.header.name_raw, b"Tiny");
    assert!(matches!(cut.body, Err(MapError::Truncated { .. })));
    // An oversized text length is rejected before any allocation.
    let mut huge = 0x1Cu32.to_le_bytes().to_vec();
    huge.extend_from_slice(&[1, 36, 0, 0, 0, 0]);
    huge.extend_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        parse_header(&huge, limits),
        Err(MapError::Invalid { .. })
    ));
    // Decompression is bounded.
    let tiny = MapLimits {
        max_payload: 64,
        ..limits
    };
    assert!(matches!(
        decompress_map(&gzip_stored(&full), tiny),
        Err(MapError::TooLarge { .. })
    ));
    assert_eq!(decompress_map(&gzip_stored(&full), limits).unwrap(), full);
    assert_eq!(
        decompress_prefix(&gzip_stored(&full), 16).unwrap(),
        &full[..16]
    );
}

#[test]
fn matching_requires_agreeing_metadata_and_reports_ambiguity() {
    let limits = MapLimits::default();
    let header = |name: &[u8], desc: &[u8], size| {
        parse_header(&sod_header(name, desc, size, false), limits).unwrap()
    };
    let wanted = header(b"Island", b"The real one", 72);
    let same_name_other_size = header(b"Island", b"The real one", 108);
    let same_name_other_text = header(b"Island", b"A remake", 72);
    let other = header(b"Mainland", b"The real one", 72);
    let key = SaveMapKey {
        name_raw: b"Island",
        description_raw: Some(b"The real one"),
        size: Some(72),
        has_underground: Some(false),
        difficulty: Some(2),
    };
    let all = [
        other.clone(),
        same_name_other_size.clone(),
        wanted.clone(),
        same_name_other_text,
    ];
    assert_eq!(
        match_candidates(&key, all.iter()),
        MatchOutcome::Unique {
            index: 2,
            strength: MatchStrength::Full
        }
    );
    // Two byte-identical headers (e.g. a copy in a sub-folder) need a choice.
    let twins = [wanted.clone(), other.clone(), wanted];
    assert_eq!(
        match_candidates(&key, twins.iter()),
        MatchOutcome::Ambiguous {
            indexes: vec![0, 2]
        }
    );
    // A matching file name is irrelevant; conflicting metadata rejects.
    assert_eq!(
        match_candidates(&key, [other, same_name_other_size].iter()),
        MatchOutcome::NotFound
    );
    // A save with only a name still matches, at the weakest strength.
    let name_only = SaveMapKey {
        description_raw: None,
        size: None,
        has_underground: None,
        difficulty: None,
        ..key
    };
    assert!(matches!(
        match_candidates(&name_only, twins[..1].iter()),
        MatchOutcome::Unique {
            strength: MatchStrength::NameOnly,
            ..
        }
    ));
}

#[test]
fn discovery_is_bounded_to_the_given_folder_and_depth() {
    let root = std::env::temp_dir().join(format!("h3forge-map-scan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let deep = root.join("Maps/a/b/c");
    std::fs::create_dir_all(&deep).unwrap();
    let file = gzip_stored(&sod_map(b"Tiny"));
    std::fs::write(root.join("Maps/top.h3m"), &file).unwrap();
    std::fs::write(root.join("Maps/a/one.H3M"), &file).unwrap();
    std::fs::write(root.join("Maps/a/b/two.h3m"), &file).unwrap();
    std::fs::write(deep.join("too-deep.h3m"), &file).unwrap();
    std::fs::write(root.join("Maps/notes.txt"), b"not a map").unwrap();
    std::fs::write(root.join("outside.h3m"), &file).unwrap();

    let limits = ScanLimits::default();
    let found = list_map_files(&root.join("Maps"), limits);
    let names: Vec<String> = found
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["top.h3m", "one.H3M", "two.h3m"]);
    let capped = list_map_files(
        &root.join("Maps"),
        ScanLimits {
            max_files: 2,
            ..limits
        },
    );
    assert_eq!(capped.len(), 2);
    let header = read_map_header(&found[0], MapLimits::default(), limits).unwrap();
    assert_eq!(header.name_raw, b"Tiny");
    assert!(read_map_header(&root.join("Maps/notes.txt"), MapLimits::default(), limits).is_err());
    std::fs::remove_dir_all(&root).unwrap();
}
