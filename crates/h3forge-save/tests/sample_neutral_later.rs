//! Two generations of the same private map; originals are never edited.
use h3forge_save::{SaveDocument, apply_patches, compress_gzip, neutral::NeutralEdit};

#[test]
fn later_save_creatures() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping later neutral fixtures: H3FORGE_FIXTURE_ROOT unset");
        return;
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/fixtures/neutral-later");
    std::fs::create_dir_all(&dir).unwrap();
    for (name, expected_monsters, tile_start) in [
        ("SaAndVla_red_tent_before.GM2", 1119, 2263),
        ("SaAndVla.GM2", 469, 2253),
    ] {
        let original_path = std::path::PathBuf::from(&root).join(name);
        let original = std::fs::read(&original_path).unwrap();
        let copy = dir.join(name);
        std::fs::write(&copy, &original).unwrap();
        let doc = SaveDocument::open(std::fs::read(copy).unwrap()).unwrap();
        let found = doc.neutral_armies();
        assert_eq!(found.unavailable_reason, None, "{name}");
        assert_eq!(
            found.groups.iter().filter(|g| g.class == 54).count(),
            expected_monsters
        );
        assert_eq!(found.groups.iter().filter(|g| g.class == 33).count(), 46);
        assert!(
            found
                .groups
                .iter()
                .all(|g| g.army.is_some() && g.read_only_reason.is_none())
        );
        println!("{name}: {expected_monsters} wandering groups, 46 verified garrisons");

        for class in [54, 33] {
            let group = found
                .groups
                .iter()
                .find(|g| g.class == class && !g.army.unwrap()[0].is_empty())
                .unwrap();
            let slot = group.army.unwrap()[0];
            let patches = doc
                .plan_neutral_edits(
                    group.id,
                    &[NeutralEdit {
                        slot: 0,
                        creature_id: Some(slot.creature_id),
                        count: slot.count + 1,
                    }],
                )
                .unwrap();
            assert_eq!(patches.len(), 1);
            assert_eq!(patches[0].expected.len(), 4);
            let mut payload = doc.payload().to_vec();
            apply_patches(&mut payload, &patches).unwrap();
            for (index, (&before, &after)) in doc.payload().iter().zip(&payload).enumerate() {
                if !(patches[0].offset..patches[0].end()).contains(&index) {
                    assert_eq!(before, after);
                }
            }
            let reopened = SaveDocument::open(compress_gzip(&payload)).unwrap();
            let after = reopened.neutral_armies();
            assert_eq!(after.unavailable_reason, None);
            assert_eq!(after.groups.len(), found.groups.len());
            assert_eq!(
                after
                    .groups
                    .iter()
                    .find(|g| g.id == group.id)
                    .unwrap()
                    .army
                    .unwrap()[0]
                    .count,
                slot.count + 1
            );
        }
        // Break a rendered object's reference: stronger discovery must refuse it.
        let mut malformed = doc.payload().to_vec();
        let mut at = tile_start;
        while u32::from_le_bytes(malformed[at + 18..at + 22].try_into().unwrap()) == 0 {
            at += 22;
        }
        malformed[at + 22..at + 24].copy_from_slice(&u16::MAX.to_le_bytes());
        let malformed = SaveDocument::open(compress_gzip(&malformed)).unwrap();
        assert!(malformed.neutral_armies().unavailable_reason.is_some());
        assert_eq!(std::fs::read(original_path).unwrap(), original);
    }
}
