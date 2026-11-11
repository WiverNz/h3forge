//! Opt-in equipment editing verification against the real sample save.
//!
//! Evidence chain validated here (H3F-012): a high-confidence hero with
//! cleanly editable wear slots accepts an equip (slot word + attribute bonus
//! + granted-spell availability patches), the recompressed output reopens
//! with the artifact worn and the diff limited to the planned patch union;
//! unequipping restores the original payload byte for byte. Without the
//! fixture the test passes without touching anything.

use std::path::{Path, PathBuf};

use h3forge_save::{EquipmentEdit, EquipmentItem, HeroConfidence, HeroEdit, SaveDocument};

fn fixture_copy() -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from)?;
    let source = root.join("SaAndVla.GM2");
    if !source.is_file() {
        return None;
    }
    let destination = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/fixtures");
    std::fs::create_dir_all(&destination).ok()?;
    let destination = destination.join("SaAndVla-equipment.GM2");
    std::fs::copy(&source, &destination).ok()?;
    Some(destination)
}

#[test]
fn sample_equipment_roundtrip_equip_and_unequip() {
    let Some(copy) = fixture_copy() else {
        eprintln!("no private sample copy; skipping equipment roundtrip test");
        return;
    };
    let original_bytes = std::fs::read(&copy).expect("sample reads");
    let document = SaveDocument::open(original_bytes.clone()).expect("opens");

    // First high-confidence hero with clean gating and an empty weapon slot.
    // Delta coupling makes equip + unequip an exact inverse on any such hero:
    // Centaur's Axe grants no spells and its +2 Attack delta restores on
    // removal, whatever unrelated bytes the save carries.
    let id = document
        .heroes
        .iter()
        .position(|hero| {
            hero.confidence == HeroConfidence::High
                && hero.equipment_gating.is_clear()
                && hero.equipment_items[3] == EquipmentItem::Empty
        })
        .expect("an editable hero with an empty weapon slot");
    let hero = &document.heroes[id];
    let attack_before = hero.primary.attack;
    assert_eq!(
        h3forge_save::artifacts::stat_bonus(7),
        Some([2, 0, 0, 0]),
        "Centaur's Axe is the +2 Attack weapon"
    );
    eprintln!(
        "candidate: {} attack={} spellbook={}",
        hero.name_lossy, attack_before, hero.has_spellbook
    );

    // Move-equip: Centaur's Axe enters the weapon slot. (The backpack side
    // of the move is a separate explicit edit; here we exercise the core op.)
    let equip = [HeroEdit::SetEquipmentSlot {
        slot: 3,
        item: EquipmentEdit::Artifact(7),
    }];
    let plan = document.plan_hero_edits(id, &equip).expect("plans");
    assert!(
        !plan.is_empty(),
        "equip plans at least the slot word and the attack bonus"
    );
    let output = document.serialize_with_patches(&plan).expect("serializes");
    let reopened = SaveDocument::open(output).expect("modified save reopens");
    let edited = &reopened.heroes[id];
    assert_eq!(edited.equipment_items[3], EquipmentItem::Artifact(7));
    assert_eq!(
        edited.primary.attack,
        attack_before.wrapping_add(2),
        "stored attack includes the axe bonus"
    );

    // Byte-level guarantee: only the planned byte ranges differ.
    let mut changed = Vec::new();
    for (index, (before, after)) in document
        .payload()
        .iter()
        .zip(reopened.payload())
        .enumerate()
    {
        if before != after {
            changed.push(index);
        }
    }
    let mut expected: Vec<usize> = plan
        .iter()
        .flat_map(|patch| patch.offset..patch.offset + patch.expected.len())
        .collect();
    expected.sort_unstable();
    assert_eq!(changed, expected, "diff limited to the patch union");

    // Unequip restores the original payload byte for byte.
    let clear = [HeroEdit::SetEquipmentSlot {
        slot: 3,
        item: EquipmentEdit::Empty,
    }];
    let clear_plan = reopened.plan_hero_edits(id, &clear).expect("plans");
    let restored =
        SaveDocument::open(reopened.serialize_with_patches(&clear_plan).unwrap()).unwrap();
    assert_eq!(restored.heroes[id].equipment_items[3], EquipmentItem::Empty);
    assert_eq!(restored.heroes[id].primary.attack, attack_before);
    assert_eq!(
        restored.payload(),
        document.payload(),
        "equip + unequip is an exact inverse"
    );

    // The source copy on disk was never written to.
    assert_eq!(std::fs::read(&copy).unwrap(), original_bytes);
}

/// Evidence for the combination-artifact reservation model: in every real
/// sample, the counters at 1047-1055 equal what the worn combination
/// artifacts imply, each worn artifact sits in a slot of its own kind, and
/// taking a worn combination off and putting it back is byte-exact.
#[test]
fn sample_reservation_counters_match_worn_combinations() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from) else {
        eprintln!("no private samples; skipping reservation model test");
        return;
    };
    let mut combos_seen = std::collections::BTreeSet::new();
    let mut saves = 0;
    for entry in std::fs::read_dir(&root).expect("fixture root lists") {
        let path = entry.expect("entry").path();
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = SaveDocument::open(bytes) else {
            continue;
        };
        saves += 1;
        for hero in &document.heroes {
            if hero.confidence != HeroConfidence::High {
                continue;
            }
            assert!(
                !hero.equipment_gating.reservation_mismatch,
                "{path:?} hero {}: {:?}",
                hero.id, hero.equipment_gating.reservation_bytes
            );
            for (slot, item) in hero.equipment_items.iter().enumerate() {
                let EquipmentItem::Artifact(id) = *item else {
                    continue;
                };
                if h3forge_save::artifacts::reserved_slots(id).is_empty()
                    || h3forge_save::artifacts::wear_slot_kind(slot as u8).is_none()
                {
                    continue;
                }
                combos_seen.insert(id);
                if !hero.equipment_gating.is_clear() {
                    continue;
                }
                let set = |item| HeroEdit::SetEquipmentSlot {
                    slot: slot as u8,
                    item,
                };
                let off = document
                    .plan_hero_edits(hero.id, &[set(EquipmentEdit::Empty)])
                    .expect("unequip plans");
                let without =
                    SaveDocument::open(document.serialize_with_patches(&off).unwrap()).unwrap();
                assert!(
                    without.heroes[hero.id].equipment_gating.is_clear(),
                    "counters cleared with the combination"
                );
                let on = without
                    .plan_hero_edits(hero.id, &[set(EquipmentEdit::Artifact(id))])
                    .expect("re-equip plans");
                let back =
                    SaveDocument::open(without.serialize_with_patches(&on).unwrap()).unwrap();
                // The slot word's second half may differ (game blank
                // encodings); everything else must be identical.
                let at = hero.span.start + 382 + slot * 8;
                let (a, b) = (back.payload(), document.payload());
                assert_eq!(a[..at + 4], b[..at + 4]);
                assert_eq!(a[at + 8..], b[at + 8..]);
            }
        }
    }
    eprintln!("reservation model: {saves} saves, worn combinations {combos_seen:?}");
    assert!(
        saves > 0 && !combos_seen.is_empty(),
        "samples must exercise the model"
    );
}
