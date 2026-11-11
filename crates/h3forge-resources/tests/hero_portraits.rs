//! Opt-in check of the hero portrait name tables against real archives
//! (H3F-021). `H3FORGE_FIXTURE_ROOT` must be the absolute path of a `Data`
//! folder with `H3bitmap.lod` and a HotA 1.8 `HotA.lod`; skips otherwise.

use std::collections::HashSet;
use std::path::PathBuf;

use h3forge_resources::{
    HOTA_PORTRAIT_MAX, HeroPortraitFile, LodArchive, ResourceLimits, decode_portrait_image,
    hero_portrait_file,
};

#[test]
fn every_table_entry_resolves_to_a_distinct_portrait_sized_image() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from) else {
        eprintln!("H3FORGE_FIXTURE_ROOT not set; skipping hero portrait table test");
        return;
    };
    let (Ok(base), Ok(hota)) = (
        LodArchive::open(root.join("H3bitmap.lod")),
        LodArchive::open(root.join("HotA.lod")),
    ) else {
        eprintln!("portrait archives not present; skipping hero portrait table test");
        return;
    };
    assert!(hota.has_opaque_ids(), "expects the HotA 1.8 archive layout");
    let limits = ResourceLimits::default();
    let mut seen = HashSet::new();
    let mut true_colour = 0;
    for id in 0..=HOTA_PORTRAIT_MAX {
        let bytes = match hero_portrait_file(id).expect("table covers 0-244") {
            HeroPortraitFile::Base { large, small } => {
                // Both size families exist under the table's exact names.
                base.find_entry(&small)
                    .unwrap_or_else(|_| panic!("portrait {id}: {small} missing"));
                let index = base
                    .find_entry(&large)
                    .unwrap_or_else(|_| panic!("portrait {id}: {large} missing"));
                base.read_entry(index).unwrap()
            }
            HeroPortraitFile::HotA { name, entry_id } => {
                let index = hota
                    .find_entry(entry_id)
                    .unwrap_or_else(|_| panic!("portrait {id}: {name} entry missing"));
                let bytes = hota.read_entry(index).unwrap();
                true_colour += h3forge_resources::is_p32(&bytes) as usize;
                bytes
            }
        };
        let image = decode_portrait_image(&bytes, limits)
            .unwrap_or_else(|error| panic!("portrait {id}: {error}"));
        // One HotA portrait (HCVC02, ID 180) is a row taller than the rest.
        let expected_height = if id == 180 { 65 } else { 64 };
        assert_eq!(
            (image.width, image.height),
            (58, expected_height),
            "portrait {id}"
        );
        assert!(
            seen.insert(image.data),
            "portrait {id} repeats another portrait's pixels"
        );
    }
    assert_eq!(seen.len(), 245);
    // 54 of the 82 HotA portraits ship as P32F, the rest as Heroes PCX.
    assert_eq!(true_colour, 54);
}
