//! Private copy-only H3F-027 regressions. No originals are save destinations.
use h3forge_save::{SaveDocument, apply_patches, compress_gzip, neutral::NeutralEdit};

fn sample() -> Option<SaveDocument> {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping neutral fixture: H3FORGE_FIXTURE_ROOT is unset");
        return None;
    };
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/fixtures/neutral");
    std::fs::create_dir_all(&dir).unwrap();
    let copy = dir.join("sample.GM2");
    std::fs::copy(std::path::PathBuf::from(root).join("SaAndVla.GM2"), &copy).unwrap();
    Some(SaveDocument::open(std::fs::read(copy).unwrap()).unwrap())
}

#[test]
fn current_armies_and_exact_multislot_roundtrip() {
    let Some(doc) = sample() else {
        return;
    };
    let found = doc.neutral_armies();
    assert_eq!(found.unavailable_reason, None);
    assert_eq!(found.groups.iter().filter(|g| g.class == 54).count(), 291);
    let garrisons: Vec<_> = found.groups.iter().filter(|g| g.class != 54).collect();
    assert_eq!(garrisons.len(), 47);
    assert!(
        garrisons
            .iter()
            .all(|g| g.army.is_some() && g.read_only_reason.is_none())
    );
    if let Some(path) = std::env::var_os("H3FORGE_NEUTRAL_MAP") {
        let map = h3forge_map::read_map(
            &std::path::PathBuf::from(path),
            h3forge_map::MapLimits::default(),
        )
        .unwrap();
        assert_eq!(map.header.name_raw, doc.map.name_raw);
        let body = map.body.unwrap();
        assert_eq!(body.trailing_bytes, 124);
        let authored_garrisons: Vec<_> = body
            .neutral_armies
            .iter()
            .filter(|a| matches!(a.class, 33 | 219))
            .collect();
        assert_eq!(authored_garrisons.len(), 47);
        let mut same = 0;
        for g in &garrisons {
            let original = authored_garrisons
                .iter()
                .find(|a| [a.anchor.x, a.anchor.y, a.anchor.z] == g.anchor)
                .unwrap();
            if original
                .stacks
                .iter()
                .zip(g.army.unwrap())
                .all(|(&(id, count), s)| {
                    id.unwrap_or(u32::MAX) == s.creature_id && count == s.count
                })
            {
                same += 1;
            }
        }
        let monsters: Vec<_> = body
            .neutral_armies
            .iter()
            .filter(|a| a.class == 54)
            .collect();
        let matched = found
            .groups
            .iter()
            .filter(|g| g.class == 54)
            .filter(|g| {
                monsters.iter().any(|a| {
                    [a.anchor.x, a.anchor.y, a.anchor.z] == g.anchor
                        && a.stacks[0].0 == g.creature_id
                })
            })
            .count();
        println!(
            "H3M: {} authored armies, {} fixed monsters; {same}/47 garrison armies unchanged, {matched}/291 current monster identities matched",
            body.neutral_armies.len(),
            monsters.len()
        );
        assert_eq!(matched, 280);
        let random_matches = found
            .groups
            .iter()
            .filter(|g| g.class == 54)
            .filter(|g| {
                body.neutral_armies.iter().any(|a| {
                    matches!(a.class,71..=75|162..=164)
                        && [a.anchor.x, a.anchor.y, a.anchor.z] == g.anchor
                })
            })
            .count();
        assert_eq!(random_matches, 11);
        assert_eq!(same, 29);
    } else {
        eprintln!("skipping H3M cross-check: H3FORGE_NEUTRAL_MAP is unset");
    }
    println!(
        "291 wandering groups; 47 verified garrisons; {} occupied garrisons",
        garrisons
            .iter()
            .filter(|g| g.army.unwrap().iter().any(|s| !s.is_empty()))
            .count()
    );
    let g = garrisons
        .iter()
        .find(|g| g.position == [158, 35, 1])
        .unwrap();
    assert_eq!(
        g.army.unwrap().map(|s| s.count),
        [70, 210, 140, 315, 140, 210, 70]
    );
    let edits = [
        NeutralEdit {
            slot: 0,
            creature_id: Some(13),
            count: 1234,
        },
        NeutralEdit {
            slot: 6,
            creature_id: None,
            count: 0,
        },
    ];
    let patches = doc.plan_neutral_edits(g.id, &edits).unwrap();
    assert_eq!(patches.len(), 4);
    assert_eq!(
        patches
            .iter()
            .map(|p| (p.offset, p.expected.len()))
            .collect::<Vec<_>>(),
        [
            (4_633_848, 4),
            (4_633_876, 4),
            (4_633_872, 4),
            (4_633_900, 4)
        ]
    );
    let mut payload = doc.payload().to_vec();
    apply_patches(&mut payload, &patches).unwrap();
    for (i, (&before, &after)) in doc.payload().iter().zip(&payload).enumerate() {
        let expected = patches
            .iter()
            .find(|patch| (patch.offset..patch.end()).contains(&i));
        assert_eq!(
            after,
            expected
                .map(|patch| patch.replacement[i - patch.offset])
                .unwrap_or(before)
        );
    }
    let output = compress_gzip(&payload);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/fixtures/neutral/edited.GM2");
    std::fs::write(&path, output).unwrap();
    let reopened = SaveDocument::open(std::fs::read(path).unwrap()).unwrap();
    assert_eq!(reopened.payload(), payload);
    let after = reopened.neutral_armies();
    let army = after
        .groups
        .iter()
        .find(|v| v.id == g.id)
        .unwrap()
        .army
        .unwrap();
    assert_eq!((army[0].creature_id, army[0].count), (13, 1234));
    assert!(army[6].is_empty());
    let empty = garrisons
        .iter()
        .find(|g| g.army.unwrap().iter().all(|s| s.is_empty()))
        .unwrap();
    assert_eq!(
        doc.plan_neutral_edits(empty.id, &[edits[0]]).unwrap().len(),
        2
    );
    let mut stale = doc.payload().to_vec();
    stale[patches[0].offset] ^= 1;
    let before_stale = stale.clone();
    assert!(apply_patches(&mut stale, &patches).is_err());
    assert_eq!(stale, before_stale);
    let monster = found.groups.iter().find(|g| g.class == 54).unwrap();
    assert!(doc.plan_neutral_edits(monster.id, &edits).is_err());
    for edit in [
        NeutralEdit {
            slot: 7,
            ..edits[0]
        },
        NeutralEdit {
            count: 0,
            ..edits[0]
        },
        NeutralEdit {
            creature_id: Some(99999),
            ..edits[0]
        },
    ] {
        assert!(doc.plan_neutral_edits(g.id, &[edit]).is_err());
    }
    // Break one index/coordinate link: the entire garrison table loses write support.
    let mut malformed = doc.payload().to_vec();
    malformed[4_631_041 + 57] ^= 1;
    let malformed = SaveDocument::open(compress_gzip(&malformed)).unwrap();
    assert!(malformed.plan_neutral_edits(g.id, &edits).is_err());
    // An unrecognized creature must be retained, never normalized to empty.
    let mut unknown = doc.payload().to_vec();
    unknown[4_633_847 + 1..4_633_847 + 5].copy_from_slice(&99999u32.to_le_bytes());
    let unknown = SaveDocument::open(compress_gzip(&unknown)).unwrap();
    let found = unknown.neutral_armies();
    assert_eq!(
        found
            .groups
            .iter()
            .find(|v| v.id == g.id)
            .unwrap()
            .army
            .unwrap()[0]
            .creature_id,
        99999
    );
    assert!(unknown.plan_neutral_edits(g.id, &edits).is_err());
    let mut other_version = doc.payload().to_vec();
    other_version[12..16].copy_from_slice(&12u32.to_le_bytes());
    assert!(
        SaveDocument::open(compress_gzip(&other_version))
            .unwrap()
            .neutral_armies()
            .unavailable_reason
            .is_some()
    );
}
