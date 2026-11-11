//! Opt-in H3F-026 regression against the private before/after pair saved
//! around one native "Dismiss Hero" action (Galthran, hero type 71, Pink,
//! aboard a boat). Copies only; game originals are never written.
//!
//! `H3FORGE_FIXTURE_ROOT` must be the absolute path of the `games` folder.
use h3forge_save::{HeroOwner, SaveDocument, apply_patches, compress_gzip};

const HERO: usize = 71;
/// Payload offset where the game inserted two replay events (26 bytes).
const REPLAY_INSERT: usize = 4_945_037;

fn open(case: &str, name: &str) -> Option<SaveDocument> {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping private dismissal fixture: set H3FORGE_FIXTURE_ROOT");
        return None;
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/fixtures/dismissal")
        .join(case);
    std::fs::create_dir_all(&dir).unwrap();
    let source = std::path::PathBuf::from(root).join(name);
    if !source.exists() {
        eprintln!("skipping private dismissal fixture: {name} is missing");
        return None;
    }
    std::fs::copy(&source, dir.join(name)).unwrap();
    Some(SaveDocument::open(std::fs::read(dir.join(name)).unwrap()).unwrap())
}

#[test]
fn dismissal_matches_the_native_before_after_pair() {
    let (Some(before), Some(after)) = (
        open("pair", "SaAndVla_before.GM2"),
        open("pair", "SaAndVla_after.GM2"),
    ) else {
        return;
    };
    assert_eq!(before.hero_dismissal_blocker(HERO), None);
    let patches = before.plan_hero_dismissals(&[HERO]).unwrap();
    let hero = before.heroes[HERO].span.start;
    assert_eq!(hero, 4_438_181);
    let mut offsets: Vec<(usize, usize)> = patches
        .iter()
        .map(|p| (p.offset, p.expected.len()))
        .collect();
    offsets.sort_unstable();
    assert_eq!(
        offsets,
        [
            (346_104, 2),     // tile: actionable flag, object class
            (346_111, 1),     // tile: object properties (hero type)
            (4_326_698, 1),   // boat 23: exists
            (4_327_931, 5),   // Pink: count, selected hero, slots
            (hero - 20, 1),   // is_visible
            (hero, 1),        // owner
            (hero + 2, 2),    // temporary morale, luck
            (hero + 19, 8),   // destination x, y
            (hero + 47, 1),   // spell points 35 -> 40 (knowledge 4)
            (hero + 109, 3),  // visited bonuses
            (hero + 141, 1),  // army slot 0 count 155 -> 1
            (hero + 1080, 2), // sea movement 2500 -> 1500
            (4_605_309, 1),   // hero owner table entry
        ]
    );
    let mut patched = before.payload().to_vec();
    apply_patches(&mut patched, &patches).unwrap();

    // Every patched byte equals the byte the game itself wrote.
    let game = after.payload();
    for patch in &patches {
        assert_eq!(
            &game[patch.offset..patch.end()],
            &patched[patch.offset..patch.end()],
            "{}",
            patch.reason
        );
    }
    // The game appended two replay events; everything after is identical.
    assert_eq!(game.len(), patched.len() + 26);
    assert_eq!(&game[REPLAY_INSERT + 26..], &patched[REPLAY_INSERT..]);
    // Remaining differences are exactly the documented, not reproduced set.
    let mut rest = Vec::new();
    let mut i = 0;
    while i < REPLAY_INSERT {
        if game[i] == patched[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < REPLAY_INSERT && game[i] != patched[i] {
            i += 1;
        }
        rest.push((start, i - start));
    }
    assert_eq!(
        rest,
        [
            (907, 10),      // embedded save file name
            (4_335_985, 1), // town 13 spell mask byte, incidental
            // Land movement 2200: Equestrian's Gloves worn, bonus unverified.
            (hero + 31, 2),
            (hero + 35, 2),
            (hero + 1076, 2),
            (4_922_520, 1), // replay event count 996 -> 998
            (4_924_201, 1), // byte inside an older replay event
        ]
    );

    // Save and reopen: cross-references stay consistent.
    let reopened = SaveDocument::open(compress_gzip(&patched)).unwrap();
    assert_eq!(reopened.payload(), &patched[..]);
    assert_eq!(reopened.heroes.len(), before.heroes.len());
    assert_eq!(reopened.heroes[HERO].owner, HeroOwner::Neutral);
    let army = &reopened.heroes[HERO].army;
    assert_eq!((army[0].creature_id, army[0].count), (56, 1));
    assert!(army[1..].iter().all(|slot| slot.is_empty()));
    assert_eq!(reopened.heroes[HERO].spell_points, 40);
    assert_eq!(reopened.players.len(), 8);
    let pink = &reopened.players[7];
    assert_eq!((pink.hero_count, pink.active), (2, true));
    assert!(pink.read_only_reason.is_none());
    assert_eq!(reopened.towns.len(), before.towns.len());
    assert!(reopened.hero_dismissal_blocker(HERO).is_some());
    assert!(reopened.plan_hero_dismissals(&[HERO]).is_err());
}

#[test]
fn dismissal_gates_and_shared_roster() {
    let Some(doc) = open("gates", "SaAndVla_before.GM2") else {
        return;
    };
    let blocked = |id: usize, needle: &str| {
        let reason = doc.hero_dismissal_blocker(id).expect("blocked");
        assert!(reason.contains(needle), "{id}: {reason}");
        assert!(doc.plan_hero_dismissals(&[id]).is_err());
    };
    blocked(0, "does not belong"); // unowned pool hero
    blocked(69, "garrisoned"); // inside Pink's town
    blocked(85, "map object"); // standing on a visitable object
    assert!(doc.plan_hero_dismissals(&[HERO, HERO]).is_err());
    assert!(doc.plan_hero_dismissals(&[9999]).is_err());
    assert!(doc.plan_hero_dismissals(&[]).unwrap().is_empty());

    // Two heroes of one player share the roster bytes: one joint plan.
    assert_eq!(doc.hero_dismissal_blocker(78), None);
    let patches = doc.plan_hero_dismissals(&[78, HERO]).unwrap();
    let mut patched = doc.payload().to_vec();
    apply_patches(&mut patched, &patches).unwrap();
    assert_eq!(
        &patched[4_327_930..4_327_941],
        &[7, 1, 0xFF, 66, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    );
    let reopened = SaveDocument::open(compress_gzip(&patched)).unwrap();
    assert_eq!(reopened.players[7].hero_count, 1);
    assert!(reopened.players[7].read_only_reason.is_none());
    for id in [78, HERO] {
        let hero = &reopened.heroes[id];
        assert_eq!(hero.owner, HeroOwner::Neutral);
        // One creature of the first stack; every other slot blank.
        let first = doc.heroes[id].army.iter().find(|s| !s.is_empty()).unwrap();
        assert_eq!(
            (hero.army[0].creature_id, hero.army[0].count),
            (first.creature_id, 1)
        );
        assert!(hero.army[1..].iter().all(|slot| slot.is_empty()));
        // Hero 78 has Intelligence: its spell points are left untouched.
        let has_intelligence = hero.skills.iter().any(|skill| skill.skill_id == 24);
        assert_eq!(has_intelligence, id == 78);
        let expected = if has_intelligence {
            doc.heroes[id].spell_points
        } else {
            u16::from(hero.primary.knowledge) * 10
        };
        assert_eq!(hero.spell_points, expected);
    }
    assert!(doc.heroes[78].army.iter().filter(|s| !s.is_empty()).count() > 1);
    // Only bytes covered by patches changed.
    let changed = doc
        .payload()
        .iter()
        .zip(&patched)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        changed,
        patches.iter().map(|p| p.expected.len()).sum::<usize>()
    );
}

/// Second clean pair: Moandor (hero type 66, Pink) dismissed on plain land
/// with six stacks, Basic Intelligence and Basic Navigation. Every byte of
/// game state the game changed is reproduced.
#[test]
fn land_dismissal_reproduces_the_whole_native_diff() {
    const MOANDOR: usize = 66;
    const INSERT: usize = 4_945_063;
    let (Some(before), Some(after)) = (
        open("land", "SaAndVla_before2.GM2"),
        open("land", "SaAndVla_after2.GM2"),
    ) else {
        return;
    };
    assert_eq!(before.hero_dismissal_blocker(MOANDOR), None);
    let patches = before.plan_hero_dismissals(&[MOANDOR]).unwrap();
    let hero = before.heroes[MOANDOR].span.start;
    let mut patched = before.payload().to_vec();
    apply_patches(&mut patched, &patches).unwrap();
    let game = after.payload();
    // One appended replay event (8 bytes); the tail is identical.
    assert_eq!(game.len(), patched.len() + 8);
    assert_eq!(&game[INSERT + 8..], &patched[INSERT..]);
    let rest: Vec<usize> = (0..INSERT).filter(|&i| game[i] != patched[i]).collect();
    // Only the embedded file name and the replay event count remain.
    assert_eq!(rest, [913, 914, 915, 916, 917, 4_922_520]);
    // Nothing was written that the game left alone.
    for patch in &patches {
        assert_eq!(
            &game[patch.offset..patch.end()],
            &patched[patch.offset..patch.end()],
            "{}",
            patch.reason
        );
    }
    assert!(patches.iter().any(|p| p.offset == 332_866)); // land tile
    assert!(patches.iter().any(|p| p.offset == hero + 1080)); // sea word

    let reopened = SaveDocument::open(compress_gzip(&patched)).unwrap();
    let moandor = &reopened.heroes[MOANDOR];
    assert_eq!(moandor.owner, HeroOwner::Neutral);
    assert_eq!(
        (moandor.army[0].creature_id, moandor.army[0].count),
        (56, 1)
    );
    assert!(moandor.army[1..].iter().all(|slot| slot.is_empty()));
    assert_eq!(moandor.spell_points, 72); // knowledge 6, Basic Intelligence
    assert_eq!(
        (moandor.movement_total, moandor.movement_left),
        (2000, 2000)
    );
    assert_eq!(reopened.players[7].hero_count, 1);
    assert!(reopened.players[7].read_only_reason.is_none());
}
