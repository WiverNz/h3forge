//! Town building icons from the local archives (opt-in via
//! `H3FORGE_FIXTURE_ROOT` pointing at the `Data` folder; skips without it).
//! Mapping evidence lives in `src/building_icons.rs`. The expected slot sets
//! pin which building IDs carry a real picture per faction, so a container,
//! decoder or filler-rule change is caught here.

use std::path::PathBuf;

use h3forge_resources::{BuildingIcons, HotABuildingIcons, LodArchive};

fn png_dims(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG signature");
    (
        u32::from_be_bytes(png[16..20].try_into().unwrap()),
        u32::from_be_bytes(png[20..24].try_into().unwrap()),
    )
}

fn root() -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from);
    if root.is_none() {
        eprintln!("no asset root; skipping building icon check");
    }
    root
}

/// Building IDs 0-43 without a picture, per town type.
const UNUSED: [&[u8]; 12] = [
    &[4, 20, 23, 24, 25, 27, 28, 29],    // Castle
    &[6, 20, 23, 27, 28, 29],            // Rampart
    &[6, 20, 24, 25, 27, 28, 29],        // Tower
    &[6, 17, 20, 27, 28, 29],            // Inferno
    &[20, 23, 24, 25, 27, 28, 29],       // Necropolis
    &[6, 20, 24, 25, 27, 28, 29],        // Dungeon
    &[3, 4, 6, 20, 24, 25, 27, 28, 29],  // Stronghold
    &[3, 4, 20, 23, 24, 25, 27, 28, 29], // Fortress
    &[22, 23, 24, 25, 27, 28, 29],       // Conflux
    &[4, 17, 20, 27, 28, 29],            // Cove
    &[24, 25, 27, 28, 29],               // Factory
    &[4, 23, 24, 25, 27, 28, 29],        // Bulwark
];

#[test]
fn every_faction_resolves_its_building_icons() {
    let Some(root) = root() else { return };
    let sprite = LodArchive::open(root.join("H3sprite.lod")).unwrap();
    let hota = LodArchive::open(root.join("HotA.lod")).unwrap();
    let mut base = BuildingIcons::new();
    let mut extra = HotABuildingIcons::new();
    let mut resolved = 0;
    for town_type in 0..12u8 {
        let mut unused = Vec::new();
        for building in 0..=43u8 {
            let png = if town_type < 9 {
                base.png(&sprite, town_type, building).unwrap()
            } else {
                extra.png(&hota, town_type, building).unwrap()
            };
            match png {
                Some(png) => {
                    assert_eq!(png_dims(png), (150, 70));
                    resolved += 1;
                }
                None => unused.push(building),
            }
        }
        assert_eq!(unused, UNUSED[town_type as usize], "town type {town_type}");
    }
    eprintln!("{resolved} building icons resolve");
    // Out-of-range requests never fall back to another picture.
    assert!(base.png(&sprite, 0, 44).unwrap().is_none());
    assert!(base.png(&sprite, 9, 0).unwrap().is_none());
    assert!(extra.png(&hota, 8, 0).unwrap().is_none());
    assert!(extra.png(&hota, 12, 0).unwrap().is_none());
}
