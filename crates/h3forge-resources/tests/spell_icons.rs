//! Verified spell-icon mapping against the local `H3sprite.lod/spells.def`
//! (opt-in via `H3FORGE_FIXTURE_ROOT`; skips without it). Evidence for the
//! positional ID mapping lives in `src/spell_icons.rs`.

use h3forge_resources::{LodArchive, SpellIcons};

fn png_dims(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG signature");
    assert_eq!(&png[12..16], b"IHDR");
    (
        u32::from_be_bytes(png[16..20].try_into().unwrap()),
        u32::from_be_bytes(png[20..24].try_into().unwrap()),
    )
}

#[test]
fn local_spells_def_resolves_all_seventy_spell_ids() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("no asset root; skipping spell icon check");
        return;
    };
    let archive = LodArchive::open(std::path::PathBuf::from(root).join("H3sprite.lod")).unwrap();
    let mut icons = SpellIcons::load(&archive).unwrap();
    let mut resolved = 0;
    for id in 0..=69u32 {
        let png = icons
            .png(id)
            .unwrap_or_else(|e| panic!("spell {id} decodes: {e}"))
            .unwrap_or_else(|| panic!("spell {id} resolves"));
        assert_eq!(png_dims(png), (78, 65), "spell {id} canvas");
        resolved += 1;
    }
    assert_eq!(resolved, 70);
}

#[test]
fn real_spell_frames_carry_transparent_index_zero_pixels() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("no asset root; skipping spell transparency check");
        return;
    };
    let archive = LodArchive::open(std::path::PathBuf::from(root).join("H3sprite.lod")).unwrap();
    let bytes = archive
        .read_entry(archive.find_entry("spells.def").unwrap())
        .unwrap();
    let def = h3forge_resources::DefFile::parse(
        bytes,
        h3forge_resources::ResourceLimits {
            max_image_pixels: 78 * 65,
            ..Default::default()
        },
    )
    .unwrap();
    // Every spellbook frame has palette-index-0 canvas padding; Summon Boat
    // (spell ID 0) is the representative transparent frame.
    let mut frames_with_transparency = 0;
    for index in 0..70 {
        let image = def.decode_index(index).unwrap();
        let rgba = image.to_rgba();
        let transparent = rgba.data.chunks(4).filter(|px| px[3] == 0).count();
        assert!(transparent > 0, "frame {index} has transparent padding");
        assert!(
            transparent < rgba.data.len() / 4,
            "frame {index} is not empty"
        );
        frames_with_transparency += 1;
    }
    assert_eq!(frames_with_transparency, 70);
}

#[test]
fn unverified_spell_ids_fall_back_to_none() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("no asset root; skipping spell fallback check");
        return;
    };
    let archive = LodArchive::open(std::path::PathBuf::from(root).join("H3sprite.lod")).unwrap();
    let mut icons = SpellIcons::load(&archive).unwrap();
    for id in [70u32, 77, 100, u32::MAX] {
        assert_eq!(icons.png(id).unwrap(), None, "id {id} stays unresolved");
    }
}
