//! Verified artifact-icon mapping against the local
//! `H3sprite.lod/Artifact.def` (opt-in via `H3FORGE_FIXTURE_ROOT`; skips
//! without it). Evidence for the name-based ID mapping lives in
//! `src/artifact_icons.rs`.

use h3forge_resources::{ARTIFACT_ID_MAX, ArtifactIcons, LodArchive};

fn png_dims(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG signature");
    assert_eq!(&png[12..16], b"IHDR");
    (
        u32::from_be_bytes(png[16..20].try_into().unwrap()),
        u32::from_be_bytes(png[20..24].try_into().unwrap()),
    )
}

#[test]
fn local_artifact_def_resolves_all_verified_ids() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("no asset root; skipping artifact icon check");
        return;
    };
    let archive = LodArchive::open(std::path::PathBuf::from(root).join("H3sprite.lod")).unwrap();
    let mut icons = ArtifactIcons::load(&archive).unwrap();
    let mut resolved = 0;
    for id in 0..=ARTIFACT_ID_MAX {
        let png = icons
            .png(id)
            .unwrap_or_else(|e| panic!("artifact {id} decodes: {e}"))
            .unwrap_or_else(|| panic!("artifact {id} resolves"));
        assert_eq!(png_dims(png), (44, 44), "artifact {id} canvas");
        resolved += 1;
    }
    assert_eq!(resolved, ARTIFACT_ID_MAX + 1);
    // HotA-specific IDs have no frame in Artifact.def: placeholder, never a
    // guessed neighbouring image.
    assert!(icons.png(144).unwrap().is_none());
    for id in 141..=143 {
        assert!(
            icons.png(id).unwrap().is_none(),
            "base unavailable tiles must not be artwork"
        );
    }
    assert!(icons.png(165).unwrap().is_none());
    assert!(icons.png(u32::MAX).unwrap().is_none());
}

#[test]
fn pinned_artifact_icon_content_and_transparency() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("no asset root; skipping artifact icon content check");
        return;
    };
    let archive = LodArchive::open(std::path::PathBuf::from(root).join("H3sprite.lod")).unwrap();
    let mut icons = ArtifactIcons::load(&archive).unwrap();
    // CRC-32 pinned decodes: (artifact ID, expected CRC). Spell scroll (1),
    // spellbook (0), Centaur's Axe (7). The axe is HrArt002, not HrArt007.
    for (id, crc) in [(0u32, 0xa2670df5), (1, 0xab988e94), (7, 0x45065d5b)] {
        let png = icons.png(id).unwrap().unwrap();
        assert_eq!(crc32fast::hash(png), crc, "artifact {id} PNG bytes");
    }
    // Real artwork has transparent padding around the item (palette index 0).
    let bytes = archive
        .read_entry(archive.find_entry("Artifact.def").unwrap())
        .unwrap();
    let def = h3forge_resources::DefFile::parse(
        bytes,
        h3forge_resources::ResourceLimits {
            max_image_pixels: 44 * 44,
            ..Default::default()
        },
    )
    .unwrap();
    // Independent expected-name cases cover both sides of the legacy numbering.
    for (id, name) in [
        (2, "HrArt122.pcx"),
        (3, "HrArt123.pcx"),
        (6, "HrArt126.pcx"),
        (7, "HrArt002.pcx"),
        (34, "HrArt029.pcx"),
        (89, "HrArt084.pcx"),
        (124, "HrArt119.pcx"),
        (126, "HrArt121.pcx"),
        (127, "HrArt127.pcx"),
    ] {
        let index = def
            .frames()
            .iter()
            .position(|frame| frame.name == name)
            .unwrap();
        let expected = h3forge_resources::encode_png(&def.decode_index(index).unwrap().to_rgba());
        assert_eq!(
            icons.png(id).unwrap().unwrap(),
            expected,
            "artifact {id} must use {name}"
        );
    }
    let frame = def
        .frames()
        .iter()
        .position(|frame| frame.name == "HrArt002.pcx")
        .expect("axe frame by name");
    let image = def.decode_index(frame).unwrap();
    let transparent = image
        .to_rgba()
        .data
        .chunks(4)
        .filter(|px| px[3] == 0)
        .count();
    assert!(
        transparent > 44 * 44 / 4,
        "axe icon has transparent padding: {transparent}"
    );
}

#[test]
fn local_hota_artifact_icons_and_fallback() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("no asset root; skipping HotA artifact check");
        return;
    };
    let archive = LodArchive::open(std::path::PathBuf::from(root).join("HotA.lod")).unwrap();
    let mut icons = h3forge_resources::HotAArtifactIcons::load(&archive).unwrap();
    let mut resolved = 0;
    for (id, expected) in [
        (141, 0x28a84e96),
        (142, 0x86283a4c),
        (143, 0xfd6d4ce2),
        (147, 0x6579b999),
        (148, 0x88ee5f33),
        (149, 0xa5590cea),
        (150, 0x17cc6eca),
        (151, 0xbf484b2a),
        (152, 0x99bda236),
        (153, 0x2681fa94),
        (154, 0x391f6869),
        (155, 0xf9c5ca45),
        (156, 0x0389a1aa),
        (157, 0x261dc925),
        (158, 0x9514a237),
        (159, 0xf6f880cd),
        (160, 0xddfb9c63),
        (161, 0x792ee97a),
        (162, 0x2e63ff2d),
        (163, 0x48905a26),
        (164, 0x5ee7e6d6),
        (165, 0x193e9d1a),
    ] {
        let png = icons.png(id).unwrap().unwrap();
        assert_eq!(png_dims(png), (44, 44));
        assert_eq!(crc32fast::hash(png), expected, "verified HotA artwork {id}");
        resolved += 1;
    }
    assert_eq!(resolved, 22);
    for id in [0, 140, 144, 145, 146, 166, u32::MAX] {
        assert!(icons.png(id).unwrap().is_none());
    }
}
