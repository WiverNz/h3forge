//! Verified HotA-only skill icons (Interference 28, Runes 29) from the local
//! `HotA.lod` D32F container (opt-in via `H3FORGE_FIXTURE_ROOT`; skips
//! without it). Identification evidence lives in `src/hota32.rs` and
//! `src/skill_icons.rs`: frames 87-89 are pixel-exact with the verified
//! Interference references, 90-92 with the Runes references. The CRC-32
//! constants below pin the exact decoded PNG bytes so any future resource,
//! decode, or mapping change is caught here.

use std::path::PathBuf;

use h3forge_resources::{HotASkillIcons, LodArchive};

fn png_dims(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG signature");
    assert_eq!(&png[12..16], b"IHDR");
    (
        u32::from_be_bytes(png[16..20].try_into().unwrap()),
        u32::from_be_bytes(png[20..24].try_into().unwrap()),
    )
}

fn load() -> Option<HotASkillIcons> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT")?;
    let archive = LodArchive::open(PathBuf::from(root).join("HotA.lod")).unwrap();
    Some(HotASkillIcons::load(&archive).unwrap())
}

/// (skill id, mastery, pinned CRC-32 of the encoded PNG bytes).
const PINNED: &[(u32, u8, u32)] = &[
    (28, 1, 0x7166d394),
    (28, 2, 0x40ab3c7a),
    (28, 3, 0xe5caa5e3),
    (29, 1, 0x83133791),
    (29, 2, 0x2a1a3cea),
    (29, 3, 0xf7d773be),
];

#[test]
fn local_hota_lod_resolves_all_six_hota_skill_icons() {
    let Some(mut icons) = load() else {
        eprintln!("no asset root; skipping HotA skill icon check");
        return;
    };
    assert_eq!(PINNED.len(), 6);
    for &(id, mastery, crc) in PINNED {
        let png = icons
            .png(id, mastery)
            .unwrap_or_else(|e| panic!("skill {id} mastery {mastery} decodes: {e}"))
            .unwrap_or_else(|| panic!("skill {id} mastery {mastery} resolves"));
        assert_eq!(
            png_dims(png),
            (32, 32),
            "skill {id} mastery {mastery} canvas"
        );
        assert_ne!(crc, 0, "test constant for ({id}, {mastery}) not filled in");
        assert_eq!(
            crc32fast::hash(png),
            crc,
            "skill {id} mastery {mastery} PNG bytes changed"
        );
    }
}

#[test]
fn real_hota_frames_are_pairwise_distinct_and_fully_opaque() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("no asset root; skipping HotA skill frame check");
        return;
    };
    let archive = LodArchive::open(PathBuf::from(root).join("HotA.lod")).unwrap();
    let data = archive
        .read_entry(archive.find_entry(HotASkillIcons::RESOURCE_ID).unwrap())
        .unwrap();
    let file =
        h3forge_resources::Hota32File::parse(data, h3forge_resources::ResourceLimits::default())
            .unwrap();
    let mut seen = Vec::new();
    for id in 28..=29u32 {
        for mastery in 1..=3u8 {
            let frame = 2 + mastery as usize + 3 * id as usize;
            let name = format!("secsk32.A{frame:03}");
            let image = file.decode_named(&name).unwrap();
            // VERIFIED format fact: these icons are opaque squares with a
            // drawn background (no alpha-0 pixels), unlike secsk32.def whose
            // palette index 0 is transparent. The interference frames match
            // the verified VCMI references pixel-exactly including alpha.
            assert!(
                image.data.as_chunks::<4>().0.iter().all(|px| px[3] == 255),
                "{name} is not fully opaque; the stored alpha channel changed"
            );
            assert!(
                seen.iter().all(|other| *other != image.data),
                "{name} duplicates another frame's pixels"
            );
            seen.push(image.data);
        }
    }
}

#[test]
fn non_hota_skill_ids_fall_back() {
    let Some(mut icons) = load() else {
        eprintln!("no asset root; skipping HotA skill fallback check");
        return;
    };
    for (id, mastery) in [(0, 1), (27, 3), (30, 1), (u32::MAX, 2), (28, 0), (29, 4)] {
        assert!(
            icons.png(id, mastery).unwrap().is_none(),
            "skill {id} mastery {mastery} must stay unresolved"
        );
    }
}
