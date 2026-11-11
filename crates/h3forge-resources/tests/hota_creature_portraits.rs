//! Verified HotA creature portraits from the local `HotA.lod` twcrport D32F
//! container (opt-in via `H3FORGE_FIXTURE_ROOT`; skips without it).
//! Identification evidence lives in `src/creature_portraits.rs`: creature ID
//! `i` sits at frame `i + 2`; 94 base portraits match pixel-exactly, 36 more
//! within a small channel distance, 11 carry HotA's recolored "portrait fix"
//! art over the same silhouette. The CRC-32 constants pin the exact encoded
//! PNG bytes so any resource, decode, or mapping change is caught here.

use std::path::PathBuf;

use h3forge_resources::{HotACreaturePortraits, LodArchive, hota_creature_frame_index};

fn png_dims(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG signature");
    assert_eq!(&png[12..16], b"IHDR");
    (
        u32::from_be_bytes(png[16..20].try_into().unwrap()),
        u32::from_be_bytes(png[20..24].try_into().unwrap()),
    )
}

fn load() -> Option<HotACreaturePortraits> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT")?;
    let archive = LodArchive::open(PathBuf::from(root).join("HotA.lod")).unwrap();
    Some(HotACreaturePortraits::load(&archive).unwrap())
}

/// (creature id, pinned CRC-32 of the encoded PNG, or None for filler slots).
const PINNED: &[(u32, Option<u32>)] = &[
    (0, Some(0x848ccba0)),   // Pikeman (HotA recolored)
    (93, Some(0x2b125849)),  // Thunderbird
    (154, Some(0x2f1f0a77)), // Oceanid (HotA-exclusive)
    (165, Some(0x03276ef4)), // Haspid (HotA-exclusive)
    (199, Some(0x4481663d)), // Jotunn Warlord (HotA-exclusive)
    (122, None),             // base-blank slot: filler frame
    (149, None),             // war machine: filler frame
    (152, None),             // reserved gap: filler frame
    (200, None),             // beyond the container's ID range
];

#[test]
fn frame_index_rule() {
    assert_eq!(hota_creature_frame_index(0), Some(2));
    assert_eq!(hota_creature_frame_index(199), Some(201));
    for id in [200, 255, u32::MAX] {
        assert_eq!(hota_creature_frame_index(id), None);
    }
}

#[test]
fn local_hota_lod_resolves_verified_creature_portraits() {
    let Some(mut portraits) = load() else {
        eprintln!("no asset root; skipping HotA creature portrait check");
        return;
    };
    for &(id, pinned) in PINNED {
        match pinned {
            Some(crc) => {
                let png = portraits
                    .png(id)
                    .unwrap_or_else(|e| panic!("creature {id} decodes: {e}"))
                    .unwrap_or_else(|| panic!("creature {id} resolves"));
                assert_eq!(png_dims(png), (58, 64), "creature {id} canvas");
                assert_eq!(crc32fast::hash(png), crc, "creature {id} PNG bytes changed");
            }
            None => assert!(
                portraits.png(id).unwrap().is_none(),
                "creature {id} must stay a placeholder"
            ),
        }
    }
}

#[test]
fn resolved_hota_portraits_are_pairwise_distinct() {
    let Some(mut portraits) = load() else {
        eprintln!("no asset root; skipping HotA creature distinctness check");
        return;
    };
    let mut seen = Vec::new();
    for &(id, pinned) in PINNED {
        let Some(_) = pinned else { continue };
        let crc = crc32fast::hash(portraits.png(id).unwrap().unwrap());
        assert!(
            seen.iter().all(|other| *other != crc),
            "creature {id} duplicates another portrait"
        );
        seen.push(crc);
    }
}
