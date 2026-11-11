//! Opt-in end-to-end edit verification against the real sample save.
//!
//! Evidence chain validated here (H3F-007/008): the owned spellbook-bearing
//! research candidate at span 4,727,078 accepts an attribute change and a
//! learned-spell toggle; the recompressed output reopens with identical
//! payload length, the modified values and byte differences limited to the
//! planned patch offsets; the source copy stays untouched. Without the
//! fixture the test passes without touching anything.

use std::path::{Path, PathBuf};

use h3forge_save::{HeroEdit, SaveDocument};

fn fixture_copy() -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from)?;
    let source = root.join("SaAndVla.GM2");
    if !source.is_file() {
        return None;
    }
    let destination = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/fixtures");
    std::fs::create_dir_all(&destination).ok()?;
    let destination = destination.join("SaAndVla-edit.GM2");
    std::fs::copy(&source, &destination).ok()?;
    Some(destination)
}

#[test]
fn sample_edit_roundtrip_preserves_everything_else() {
    let Some(copy) = fixture_copy() else {
        eprintln!("no private sample copy; skipping edit roundtrip test");
        return;
    };
    let original_bytes = std::fs::read(&copy).expect("sample reads");
    let document = SaveDocument::open(original_bytes.clone()).expect("opens");

    // Research candidate: owner 4, level 2, spellbook carried, primaries
    // 3/7/6/6, learned spells [3,15,17,20,25,33,35,37,42,43,44,47,64].
    let id = document
        .heroes
        .iter()
        .position(|hero| hero.span.start == 4_727_078)
        .expect("research candidate located");
    let hero = &document.heroes[id];
    assert_eq!(hero.primary.attack, 3);
    assert!(hero.has_spellbook);
    assert_eq!(hero.spells_learned[0], 0, "Summon Boat not yet learned");
    assert_eq!(hero.spells_learned[64], 1, "Remove Obstacle learned");

    let edits = [
        HeroEdit::SetAttack(9),
        HeroEdit::SetSpellLearned {
            spell_id: 0,
            learned: true,
        },
        HeroEdit::SetSpellLearned {
            spell_id: 64,
            learned: false,
        },
    ];
    let plan = document.plan_hero_edits(id, &edits).expect("plans");
    // attack + learned/available for spell 0 + learned/available for 64.
    assert_eq!(plan.len(), 5, "patches: {plan:#?}");
    let output = document.serialize_with_patches(&plan).expect("serializes");

    let reopened = SaveDocument::open(output).expect("modified save reopens");
    assert_eq!(reopened.payload().len(), document.payload().len());
    assert_eq!(reopened.heroes.len(), document.heroes.len(), "215 records");
    let edited = &reopened.heroes[id];
    assert_eq!(edited.primary.attack, 9);
    assert_eq!(edited.primary.defense, 7, "untouched attribute");
    assert_eq!(edited.spells_learned[0], 1, "Summon Boat learned");
    assert_eq!(edited.spells_available[0], 1, "available with spellbook");
    assert_eq!(edited.spells_learned[64], 0, "Remove Obstacle unlearned");
    assert_eq!(edited.spells_available[64], 0, "available cleared");

    // Byte-level guarantee: only the planned offsets differ.
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
    let mut expected: Vec<usize> = plan.iter().map(|patch| patch.offset).collect();
    expected.sort_unstable();
    assert_eq!(changed, expected, "diff limited to the patch union");

    // No-op save produces byte-identical payload output.
    let noop = document.serialize_with_patches(&[]).unwrap();
    let reopened_noop = SaveDocument::open(noop).unwrap();
    assert_eq!(reopened_noop.payload(), document.payload());

    // The source copy on disk was never written to.
    assert_eq!(std::fs::read(&copy).unwrap(), original_bytes);
}
