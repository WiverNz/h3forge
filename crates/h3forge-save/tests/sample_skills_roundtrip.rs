//! Opt-in skill-edit round trip against a private temporary copy of the
//! real sample (H3F-010): reorder, a mastery change and an add in one
//! operation; the recompressed save reopens with the new order and the byte
//! diff is limited to the count, mastery and HotA order bytes of the touched
//! skills. The source copy stays untouched.

use std::path::{Path, PathBuf};

use h3forge_save::definitions::hero::{SKILL_COUNT, SKILL_LEVELS, SKILL_SLOTS_HOT_A};
use h3forge_save::{HeroEdit, SaveDocument};

fn fixture_copy() -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from)?;
    let source = root.join("SaAndVla.GM2");
    if !source.is_file() {
        return None;
    }
    let destination = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/fixtures");
    std::fs::create_dir_all(&destination).ok()?;
    let destination = destination.join("SaAndVla-skills.GM2");
    std::fs::copy(&source, &destination).ok()?;
    Some(destination)
}

#[test]
fn sample_skills_reorder_mastery_and_add_roundtrip() {
    let Some(copy) = fixture_copy() else {
        eprintln!("no private sample copy; skipping skills roundtrip test");
        return;
    };
    let original_bytes = std::fs::read(&copy).expect("sample reads");
    let document = SaveDocument::open(original_bytes.clone()).expect("opens");

    // Research candidate: owner 4, skills Wisdom (7), Intelligence (24),
    // Mysticism (8).
    let id = document
        .heroes
        .iter()
        .position(|hero| hero.span.start == 4_727_078)
        .expect("research candidate located");
    let hero = &document.heroes[id];
    let base: Vec<(u8, u8, u8)> = hero
        .skills
        .iter()
        .map(|skill| (skill.skill_id, skill.mastery, skill.slot))
        .collect();
    assert_eq!(base.len(), 3);
    assert!(base.iter().any(|(skill_id, _, _)| *skill_id == 7));

    // Reorder (Mysticism first), Wisdom -> Expert, add Logistics (2) Basic.
    let target = vec![(8u8, 1u8), (24, 2), (7, 3), (2, 1)];
    let edits = [HeroEdit::SetSkills {
        skills: target.clone(),
    }];
    let plan = document.plan_hero_edits(id, &edits).expect("plans");
    assert_eq!(
        plan.len(),
        7,
        "count + Wisdom mastery/slot + Intelligence mastery + Mysticism slot \
         + Logistics mastery/slot: {plan:#?}"
    );
    let output = document.serialize_with_patches(&plan).expect("serializes");

    let reopened = SaveDocument::open(output).expect("modified save reopens");
    assert_eq!(reopened.payload().len(), document.payload().len());
    let edited = &reopened.heroes[id];
    let effective: Vec<(u8, u8, u8)> = edited
        .skills
        .iter()
        .map(|skill| (skill.skill_id, skill.mastery, skill.slot))
        .collect();
    assert_eq!(
        effective,
        vec![(8, 1, 1), (24, 2, 2), (7, 3, 3), (2, 1, 4)],
        "order, mastery and addition survived"
    );
    assert_eq!(
        edited.primary.attack, hero.primary.attack,
        "attributes untouched"
    );
    assert_eq!(edited.level, hero.level, "level untouched");

    // Byte-level guarantee: only the planned offsets differ, and all of them
    // live in the count word, the mastery table or the HotA order table.
    let base_offset = hero.span.start;
    let mut changed = Vec::new();
    for (index, (before, after)) in document
        .payload()
        .iter()
        .zip(reopened.payload())
        .enumerate()
    {
        if before != after {
            changed.push(index - base_offset);
        }
    }
    let mut expected: Vec<usize> = plan
        .iter()
        .map(|patch| patch.offset - base_offset)
        .collect();
    expected.sort_unstable();
    assert_eq!(changed, expected, "diff limited to the patch union");
    for offset in &changed {
        let in_count = *offset == SKILL_COUNT;
        let in_mastery = (SKILL_LEVELS..SKILL_LEVELS + 30).contains(offset);
        let in_order = (SKILL_SLOTS_HOT_A..SKILL_SLOTS_HOT_A + 30).contains(offset);
        assert!(
            in_count || in_mastery || in_order,
            "offset {offset} is outside the skill tables"
        );
    }

    // The source copy on disk was never written to.
    assert_eq!(std::fs::read(&copy).unwrap(), original_bytes);
}
