//! Town icons from the local archives (opt-in via `H3FORGE_FIXTURE_ROOT`
//! pointing at the `Data` folder; skips without it). Mapping evidence lives
//! in `src/town_icons.rs`. Every town type must decode to the documented
//! size, and the fort, village and per-type pictures must all differ.

use std::collections::HashSet;
use std::path::PathBuf;

use h3forge_resources::{HotATownIcons, LodArchive, TownIconSize, TownIcons};

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
        eprintln!("no asset root; skipping town icon check");
    }
    root
}

#[test]
fn hota_archive_resolves_all_twelve_town_types() {
    let Some(root) = root() else { return };
    let archive = LodArchive::open(root.join("HotA.lod")).unwrap();
    let mut icons = HotATownIcons::load(&archive).unwrap();
    let mut distinct = HashSet::new();
    for town_type in 0..=11u8 {
        for fort in [true, false] {
            for (size, dims) in [
                (TownIconSize::Large, (58, 64)),
                (TownIconSize::Small, (48, 32)),
            ] {
                let png = icons
                    .png(town_type, fort, size)
                    .unwrap()
                    .expect("icon resolves");
                assert_eq!(png_dims(png), dims);
                assert!(
                    distinct.insert(png.to_vec()),
                    "type {town_type} repeats a picture"
                );
            }
        }
    }
    assert_eq!(distinct.len(), 48);
    assert!(icons.png(12, true, TownIconSize::Large).unwrap().is_none());
}

#[test]
fn base_archive_resolves_the_nine_original_town_types() {
    let Some(root) = root() else { return };
    let archive = LodArchive::open(root.join("H3sprite.lod")).unwrap();
    let mut icons = TownIcons::load(&archive).unwrap();
    let mut distinct = HashSet::new();
    for town_type in 0..=8u8 {
        for fort in [true, false] {
            for (size, dims) in [
                (TownIconSize::Large, (58, 64)),
                (TownIconSize::Small, (48, 32)),
            ] {
                let png = icons
                    .png(town_type, fort, size)
                    .unwrap()
                    .expect("icon resolves");
                assert_eq!(png_dims(png), dims);
                assert!(
                    distinct.insert(png.to_vec()),
                    "type {town_type} repeats a picture"
                );
            }
        }
    }
    for town_type in [9u8, 10, 11, 12] {
        assert!(
            icons
                .png(town_type, true, TownIconSize::Large)
                .unwrap()
                .is_none()
        );
    }
}
