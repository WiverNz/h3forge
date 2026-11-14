//! Opt-in tests for H3F-013 town discovery and garrison editing against a
//! private copy of the real sample save and its source map.
//!
//! Set `H3FORGE_FIXTURE_ROOT` to a directory containing a copy of
//! `SaAndVla.GM2` and `H3FORGE_MAPS_ROOT` to the `Maps` folder holding
//! `[HotA] The Mysterious Island - v1-5-j - ALLIES.h3m`. Without the
//! variables these tests pass without touching anything. The sample bytes are
//! only read; writes go to a copy under the ignored `.local/fixtures/`
//! directory, never in `games/`, `Data/` or `Maps/`.

use std::path::{Path, PathBuf};

use h3forge_map::{MapLimits, ScanLimits, list_map_files, read_map};
use h3forge_save::definitions::towns::{self, GARRISON_SLOTS};
use h3forge_save::{
    ArmySlot, EditError, HeroConfidence, HeroOwner, SaveDocument, TownConfidence, TownEdit,
};

const EXPECTED_TOWNS: usize = 37;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn copy_destination(tag: &str) -> PathBuf {
    repo_root().join(format!(".local/fixtures/SaAndVla-towns-{tag}.GM2"))
}

fn open_sample_copy(tag: &str) -> Option<(SaveDocument, PathBuf)> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT")?;
    let source = PathBuf::from(&root).join("SaAndVla.GM2");
    if !source.is_file() {
        eprintln!("no SaAndVla.GM2 under {root:?}; skipping private sample test");
        return None;
    }
    let destination = copy_destination(tag);
    std::fs::create_dir_all(destination.parent().expect("fixture dir")).expect("fixture dir");
    let _ = std::fs::remove_file(&destination);
    std::fs::copy(&source, &destination).expect("sample copy writes");
    let canonical = destination.canonicalize().expect("copy canonicalizes");
    let repo = repo_root();
    for forbidden in ["games", "Data"] {
        assert!(
            !canonical.starts_with(repo.join(forbidden)),
            "test destination must stay out of repository {forbidden}/"
        );
    }
    let bytes = std::fs::read(&canonical).expect("copied sample reads");
    Some((SaveDocument::open(bytes).expect("sample opens"), canonical))
}

#[test]
fn sample_towns_discovered_and_verified() {
    let Some((document, _path)) = open_sample_copy("discover") else {
        return;
    };
    assert_eq!(document.towns.len(), EXPECTED_TOWNS);
    assert!(
        document
            .towns
            .iter()
            .all(|town| town.confidence == TownConfidence::High)
    );
    // Owner mix recorded during verification: 21 player-owned, 4 neutral,
    // 12 owner-4, ... (asserted loosely: player + neutral only).
    assert!(
        document
            .towns
            .iter()
            .all(|town| matches!(town.owner, HeroOwner::Player(0..=7) | HeroOwner::Neutral))
    );
    // First town is Brill at (71,185,0) with a garrisoned hero; the garrison
    // fields of hero-occupied towns are empty.
    let brill = &document.towns[0];
    assert_eq!(brill.name_lossy, "Brill");
    assert_eq!((brill.x, brill.y, brill.z), (71, 185, 0));
    assert_eq!(brill.town_type, 4);
    assert!(brill.garrison_hero.is_some());
    assert!(brill.garrison.iter().all(|slot| slot.is_empty()));

    // Every span but the last is exact by adjacency.
    for pair in document.towns.windows(2) {
        assert_eq!(pair[0].span.end, pair[1].span.start, "dense packing");
    }

    // Garrison ownership rule: exactly the towns naming a garrisoned hero
    // resolve to a hero record; all resolved heroes are invisible on the map.
    let occupied = document
        .towns
        .iter()
        .filter(|town| town.garrison_hero.is_some())
        .count();
    assert_eq!(occupied, 17, "17 garrisoned towns on the sample");
    let visited = document
        .towns
        .iter()
        .filter(|town| town.visiting_hero.is_some())
        .count();
    assert_eq!(visited, 1, "OldtownLibrary carries a visiting hero");
}

#[test]
fn sample_towns_cross_check_map() {
    let Some((document, _path)) = open_sample_copy("map") else {
        return;
    };
    let Some(maps_root) = std::env::var_os("H3FORGE_MAPS_ROOT") else {
        eprintln!("H3FORGE_MAPS_ROOT not set; skipping map cross-check");
        return;
    };
    let mut map = None;
    for file in list_map_files(&PathBuf::from(&maps_root), ScanLimits::default()) {
        if let Ok(parsed) = read_map(&file, MapLimits::default()) {
            if parsed.header.name_raw == document.map.name_raw {
                map = Some(parsed);
                break;
            }
        }
    }
    let Some(map) = map else {
        eprintln!("sample map not present; skipping map cross-check");
        return;
    };
    let body = map.body.expect("sample map body parses");
    assert!(!body.towns.is_empty());
    assert_eq!(
        document.towns.len(),
        body.towns.len(),
        "complete save town table"
    );

    // Every save town matches exactly one map town by entrance coordinates
    // (save x/y/z == map anchor x-2 / y / z, verified here), and the names
    // agree byte for byte. The town `type` byte equals the map faction for
    // every map town that declares one.
    for town in &document.towns {
        let candidates: Vec<_> = body
            .towns
            .iter()
            .filter(|candidate| {
                candidate.anchor.x.saturating_sub(2) == town.x
                    && candidate.anchor.y == town.y
                    && candidate.anchor.z == town.z
            })
            .collect();
        assert_eq!(
            candidates.len(),
            1,
            "town {} matches exactly one map object",
            town.name_lossy
        );
        let map_town = candidates[0];
        assert_eq!(
            map_town.name_raw.as_deref(),
            Some(town.name_raw.as_slice()),
            "map and save agree on the town name"
        );
        if let Some(faction) = map_town.faction {
            assert_eq!(
                faction as u8, town.town_type,
                "save type equals map faction for {}",
                town.name_lossy
            );
        }
    }
}

#[test]
fn sample_garrison_edit_roundtrip() {
    let Some((document, _path)) = open_sample_copy("roundtrip") else {
        return;
    };
    // Pick a proven unoccupied garrison town: Port Balloon (Gnoll x192, ...).
    let port_balloon = document
        .towns
        .iter()
        .find(|town| town.name_lossy == "Port Balloon")
        .expect("Port Balloon present");
    assert_eq!(port_balloon.garrison_hero, None);
    assert_eq!(
        port_balloon.garrison[0],
        ArmySlot {
            creature_id: 98,
            count: 192
        }
    );
    let base = port_balloon.span.start;

    // One valid modification: replace slot 1 with 250 pixemen.
    let plan = document
        .plan_town_edits(
            port_balloon.id,
            &[TownEdit::SetGarrisonSlot {
                slot: 0,
                creature_id: Some(0),
                count: 250,
            }],
        )
        .expect("plans");
    assert_eq!(plan.len(), 2);
    assert_eq!(plan[0].offset, base + towns::GARRISON_IDS);
    assert_eq!(plan[1].offset, base + towns::GARRISON_COUNTS);

    let bytes = document.serialize_with_patches(&plan).expect("serializes");
    let reopened = SaveDocument::open(bytes).expect("reopens");
    let edited = reopened
        .towns
        .iter()
        .find(|town| town.name_lossy == "Port Balloon")
        .expect("still located after the edit");
    assert_eq!(edited.garrison[0].creature_id, 0);
    assert_eq!(edited.garrison[0].count, 250);
    // Everything outside the patched garrison words is identical.
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
    for offset in &changed {
        assert!(
            plan.iter()
                .any(|patch| (*offset..patch.end()).contains(offset)),
            "only the garrison words differ"
        );
    }
    assert!(!changed.is_empty());
    // Town count and identities survive.
    assert_eq!(reopened.towns.len(), EXPECTED_TOWNS);
    assert_eq!(reopened.heroes.len(), 215);

    // Clear slot 2 (Lizard Warrior x55) and confirm the empty encoding.
    let plan = reopened
        .plan_town_edits(
            edited.id,
            &[TownEdit::SetGarrisonSlot {
                slot: 1,
                creature_id: None,
                count: 0,
            }],
        )
        .expect("plans clear");
    let bytes = reopened.serialize_with_patches(&plan).expect("serializes");
    let cleared = SaveDocument::open(bytes).expect("reopens");
    let cleared_town = &cleared.towns[edited.id];
    assert_eq!(cleared_town.garrison[1].creature_id, u32::MAX);
    assert_eq!(cleared_town.garrison[1].count, 0);
}

#[test]
fn sample_hero_occupied_garrison_is_blocked() {
    let Some((document, _path)) = open_sample_copy("blocked") else {
        return;
    };
    let blocked = document
        .towns
        .iter()
        .find(|town| town.garrison_hero.is_some())
        .expect("a garrisoned town exists");
    assert!(matches!(
        document.plan_town_edits(
            blocked.id,
            &[TownEdit::SetGarrisonSlot {
                slot: 0,
                creature_id: Some(0),
                count: 1,
            }]
        ),
        Err(EditError::InvalidValue { .. })
    ));
    // The linked hero is a real, high-confidence record.
    let hero = &document.heroes[blocked.garrison_hero.unwrap()];
    assert_eq!(hero.confidence, HeroConfidence::High);
}

#[test]
fn sample_town_edit_validation() {
    let Some((document, _path)) = open_sample_copy("validate") else {
        return;
    };
    let editable = document
        .towns
        .iter()
        .find(|town| town.garrison_hero.is_none())
        .expect("an unoccupied town exists");
    for edit in [
        TownEdit::SetGarrisonSlot {
            slot: GARRISON_SLOTS as u8,
            creature_id: Some(0),
            count: 1,
        },
        TownEdit::SetGarrisonSlot {
            slot: 0,
            creature_id: Some(210),
            count: 1,
        },
        TownEdit::SetGarrisonSlot {
            slot: 0,
            creature_id: Some(0),
            count: 0,
        },
        TownEdit::SetGarrisonSlot {
            slot: 0,
            creature_id: None,
            count: 1,
        },
    ] {
        assert!(
            matches!(
                document.plan_town_edits(editable.id, std::slice::from_ref(&edit)),
                Err(EditError::InvalidValue { .. })
            ),
            "{edit:?} must be rejected"
        );
    }
    // Unknown IDs and out-of-range towns error, not panic.
    assert!(matches!(
        document.plan_town_edits(document.towns.len(), &[]),
        Err(EditError::TownNotFound { .. })
    ));
}
