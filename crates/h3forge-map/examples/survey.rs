//! Development probe: parses every map under a folder and reports how far
//! each one got. Prints one detailed block for maps whose internal name
//! contains the optional second argument.
//!
//! `cargo run -p h3forge-map --example survey -- <Maps dir> [name filter]`

use std::collections::BTreeMap;

use h3forge_map::{MapLimits, ScanLimits, list_map_files, read_map};

fn main() {
    let mut args = std::env::args().skip(1);
    let root = std::path::PathBuf::from(args.next().expect("maps directory"));
    let filter = args.next();
    let mut tally: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for path in list_map_files(&root, ScanLimits::default()) {
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        match read_map(&path, MapLimits::default()) {
            Err(error) => println!("HEADER FAIL {file}: {error}"),
            Ok(map) => {
                let entry = tally.entry(map.header.format.label()).or_default();
                entry.0 += 1;
                let name = String::from_utf8_lossy(&map.header.name_raw).into_owned();
                match &map.body {
                    // Stock editors end every map with 124 padding bytes.
                    Ok(body) if matches!(body.trailing_bytes, 0 | 124) => entry.1 += 1,
                    Ok(body) => {
                        entry.2 += 1;
                        println!("TRAILING {} bytes: {file}", body.trailing_bytes);
                    }
                    Err(error) => println!("BODY FAIL {file}: {error}"),
                }
                if filter.as_deref().is_some_and(|f| name.contains(f)) {
                    println!("== {file}: {name:?} {:?}", map.header.format);
                    println!(
                        "   release {:?} size {} underground {} difficulty {}",
                        map.header.hota_release,
                        map.header.size,
                        map.header.has_underground,
                        map.header.difficulty
                    );
                    for (index, player) in map.header.players.iter().enumerate() {
                        if player.is_playable() {
                            println!(
                                "   player {index}: human {} town {:?} team {:?} main {:?}",
                                player.can_human_play,
                                player.main_town,
                                player.team,
                                player.main_hero.as_ref().map(|h| (h.hero_type, h.portrait))
                            );
                        }
                    }
                    println!("   disposed heroes: {}", map.header.disposed_heroes.len());
                    if let Ok(body) = &map.body {
                        println!(
                            "   objects {} heroes {} towns {} predefined {} trailing {}",
                            body.object_count,
                            body.heroes.len(),
                            body.towns.len(),
                            body.predefined_hero_types.len(),
                            body.trailing_bytes
                        );
                        for hero in &body.heroes {
                            println!(
                                "   hero {:?} type {:?} owner {:?} at {:?} portrait {:?} name {:?}",
                                hero.kind,
                                hero.hero_type,
                                hero.owner,
                                hero.tile(),
                                hero.portrait,
                                hero.name_raw
                                    .as_ref()
                                    .map(|n| String::from_utf8_lossy(n).into_owned())
                            );
                        }
                    }
                }
            }
        }
    }
    for (format, (total, clean, trailing)) in tally {
        println!(
            "{format}: {total} maps, {clean} walked to the end, {trailing} with unexpected trailing bytes"
        );
    }
}
