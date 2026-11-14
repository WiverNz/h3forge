//! Private sample regression; originals are only copied into .local/fixtures.
use h3forge_save::{HeroEdit, SaveDocument};

#[test]
fn xyron_army_slots_patch_only_the_corresponding_words() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping Xyron army: H3FORGE_FIXTURE_ROOT not set");
        return;
    };
    let copy = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/fixtures/xyron-army.GM2");
    std::fs::create_dir_all(copy.parent().unwrap()).unwrap();
    std::fs::copy(std::path::PathBuf::from(root).join("SaAndVla.GM2"), &copy).unwrap();
    let bytes = std::fs::read(&copy).unwrap();
    let document = SaveDocument::open(bytes.clone()).unwrap();
    // Ксирон in the save's Windows-1251 name encoding.
    let hero = document
        .heroes
        .iter()
        .find(|h| h.name_raw == [0xca, 0xf1, 0xe8, 0xf0, 0xee, 0xed])
        .unwrap();
    let expected = [
        (43, 1418),
        (45, 5080),
        (47, 6880),
        (49, 230),
        (51, 116),
        (53, 5008),
        (55, 2251),
    ];
    eprintln!("Xyron span {}, army {:?}", hero.span.start, hero.army);
    assert_eq!(hero.army.map(|s| (s.creature_id, s.count)), expected);
    for (slot, &(id, count)) in expected.iter().enumerate() {
        // Independent raw-word checks, not offsets obtained from the patch planner.
        let id_offset = hero.span.start + 113 + slot * 4;
        let count_offset = hero.span.start + 141 + slot * 4;
        assert_eq!(
            &document.payload()[id_offset..id_offset + 4],
            id.to_le_bytes()
        );
        assert_eq!(
            &document.payload()[count_offset..count_offset + 4],
            count.to_le_bytes()
        );
        let patches = document
            .plan_hero_edits(
                hero.id,
                &[HeroEdit::SetArmySlot {
                    slot: slot as u8,
                    creature_id: Some(id),
                    count: count + 1,
                }],
            )
            .unwrap();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].offset, count_offset);
        let reopened =
            SaveDocument::open(document.serialize_with_patches(&patches).unwrap()).unwrap();
        let mut expected_payload = document.payload().to_vec();
        expected_payload[count_offset..count_offset + 4]
            .copy_from_slice(&(count + 1).to_le_bytes());
        assert_eq!(reopened.payload(), expected_payload);
        let mut edited = expected;
        edited[slot].1 += 1;
        assert_eq!(
            reopened.heroes[hero.id]
                .army
                .map(|s| (s.creature_id, s.count)),
            edited
        );
    }
    assert_eq!(std::fs::read(copy).unwrap(), bytes);
}
