//! Opt-in tests for H3F-022 town building masks and Mage Guild state against
//! a private copy of the real HotA 44/11 sample save.
//!
//! Set `H3FORGE_FIXTURE_ROOT` to a directory containing `SaAndVla.GM2`.
//! Without it these tests pass without touching anything. The sample is only
//! read; the working copy lives under the ignored `.local/fixtures/`.

use std::path::{Path, PathBuf};

use h3forge_save::definitions::towns as town_layout;
use h3forge_save::town_state::{self, building, layout};
use h3forge_save::{BytePatch, SaveDocument, TownEdit, TownView};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn open_sample_copy(tag: &str) -> Option<SaveDocument> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT")?;
    let source = PathBuf::from(&root).join("SaAndVla.GM2");
    if !source.is_file() {
        eprintln!("no SaAndVla.GM2 under {root:?}; skipping private sample test");
        return None;
    }
    let destination = repo_root().join(format!(".local/fixtures/SaAndVla-town-state-{tag}.GM2"));
    std::fs::create_dir_all(destination.parent().expect("fixture dir")).expect("fixture dir");
    let _ = std::fs::remove_file(&destination);
    std::fs::copy(&source, &destination).expect("sample copy writes");
    let bytes = std::fs::read(&destination).expect("copied sample reads");
    Some(SaveDocument::open(bytes).expect("sample opens"))
}

fn town<'a>(document: &'a SaveDocument, name: &str) -> &'a TownView {
    document
        .towns
        .iter()
        .find(|town| town.name_lossy == name)
        .unwrap_or_else(|| panic!("{name} present"))
}

/// Asserts that the payloads differ exactly inside the planned patches and
/// that every patch changed at least one byte.
fn assert_diff_is_exactly(before: &[u8], after: &[u8], plan: &[BytePatch]) {
    assert_eq!(before.len(), after.len(), "payload length is preserved");
    for (offset, (old, new)) in before.iter().zip(after).enumerate() {
        if old != new {
            assert!(
                plan.iter()
                    .any(|patch| (patch.offset..patch.end()).contains(&offset)),
                "byte {offset} changed outside every planned patch"
            );
        }
    }
    for patch in plan {
        assert_ne!(
            before[patch.offset..patch.end()],
            after[patch.offset..patch.end()],
            "patch '{}' changed nothing",
            patch.reason
        );
    }
}

#[test]
fn sample_town_state_parses_and_follows_the_model() {
    let Some(document) = open_sample_copy("parse") else {
        return;
    };
    assert_eq!(document.towns.len(), 37);
    let mut guild_spells = 0usize;
    let mut building_bits = 0u32;
    let mut model_exceptions = Vec::new();
    for town in &document.towns {
        let state = town.state.as_ref().expect("44/11 town carries state");
        assert_eq!(state.mage_guild_level, town.mage_guild_level);
        assert_eq!(
            state.guild_issue(),
            None,
            "{}: guild fields follow the verified representation",
            town.name_lossy
        );
        // The logical mask never exceeds the 44 defined bits and always
        // carries the Village Hall.
        assert_eq!(state.built >> building::COUNT, 0, "{}", town.name_lossy);
        assert!(
            state.is_built(building::VILLAGE_HALL),
            "{}",
            town.name_lossy
        );
        building_bits += state.built.count_ones();
        assert_eq!(state.buildings_issue(), None, "{}", town.name_lossy);
        if state.shown_with_successor() != 0 {
            model_exceptions.push((
                town.name_lossy.clone(),
                state.town_type,
                state.shown_with_successor(),
            ));
        }
        for level in 0..town_state::GUILD_LEVELS {
            let spells = state.guild_spells(level);
            assert!(spells.len() <= state.guild_capacity(level));
            guild_spells += spells.len();
        }
    }
    // Only the two Factory towns draw a level beside its upgrade: Mage
    // Guild 4 (bit 3) stays visible under level 5.
    assert_eq!(
        model_exceptions,
        vec![
            ("Nostromo".to_owned(), 10, 1 << 3),
            ("Nautilus".to_owned(), 10, 1 << 3)
        ]
    );
    eprintln!("sample: {building_bits} built building bits, {guild_spells} guild spells");

    // Known towns.
    let brill = town(&document, "Brill").state.as_ref().expect("state");
    assert_eq!(brill.town_type, 4, "Necropolis");
    assert_eq!(brill.guild_counts, [5, 4, 3, 2, 1]);
    assert_eq!(brill.guild_spells(0), vec![15, 46, 43, 42, 31]);
    assert_eq!(brill.guild_spells(4), vec![40]);
    assert!(brill.is_built(building::CASTLE) && brill.is_built(building::FORT));
    assert!(brill.is_built(building::CITY_HALL) && !brill.is_built(building::CAPITOL));
    // Castle hides Fort and Citadel; the top Mage Guild hides the others.
    assert_eq!(brill.visible & 0x1F, 1 << 4);
    assert_eq!(brill.visible >> building::FORT & 0b111, 0b100);

    let tabor = town(&document, "Tabor Island")
        .state
        .as_ref()
        .expect("state");
    assert_eq!(tabor.mage_guild_level, 0);
    assert_eq!(tabor.guild_counts, [0; 5]);
    assert_eq!(
        tabor.built,
        1 << building::VILLAGE_HALL | 1 << (building::DWELLING_1 + 2)
    );
    assert_eq!(tabor.visible, tabor.built);
    assert!(!tabor.has_fort());

    // Faction guild limits come from the save's own allowed mask.
    assert_eq!(
        town(&document, "North Mandible")
            .state
            .as_ref()
            .unwrap()
            .max_guild_level(),
        3
    );
    assert_eq!(
        town(&document, "Reptile Promnt")
            .state
            .as_ref()
            .unwrap()
            .max_guild_level(),
        4
    );
    assert_eq!(brill.max_guild_level(), 5);

    // Tower with a Library: one extra spell per level.
    let library = town(&document, "OldtownLibrary")
        .state
        .as_ref()
        .expect("state");
    assert!(library.has_library());
    assert_eq!(library.guild_capacity(0), 6);
    assert_eq!(library.guild_counts, [2, 2, 4, 2, 2]);
    assert_eq!(library.guild_spells(2), vec![12, 60, 34, 25]);
    let corral = town(&document, "Corral").state.as_ref().expect("state");
    assert!(!corral.has_library());
    assert_eq!(corral.guild_capacity(0), 5);
}

#[test]
fn sample_building_edit_roundtrip() {
    let Some(document) = open_sample_copy("buildings") else {
        return;
    };
    let port = town(&document, "Port Balloon");
    let state = port.state.as_ref().expect("state");
    assert_eq!(state.mage_guild_level, 2);
    assert_eq!(state.guild_counts, [5, 4, 0, 0, 0]);
    assert_eq!(state.guild_spells(2), vec![49, 63, 14]);

    // Construct Mage Guild level 3.
    let target = state
        .toggle_building(state.built, 2, true)
        .expect("Mage Guild 3 is allowed in this Fortress");
    let plan = document
        .plan_town_edits(port.id, &[TownEdit::SetBuildings { built: target }])
        .expect("plans");
    let tail = port.span.start + town_layout::NAME + port.name_raw.len();
    let mut offsets: Vec<usize> = plan.iter().map(|patch| patch.offset).collect();
    offsets.sort_unstable();
    assert_eq!(
        offsets,
        vec![
            port.span.start + town_layout::MAGE_GUILD_LEVEL,
            tail + layout::GUILD_COUNTS + 2,
            tail + layout::VISIBLE,
            tail + layout::BUILT,
        ]
    );
    let bytes = document.serialize_with_patches(&plan).expect("serializes");
    let reopened = SaveDocument::open(bytes).expect("reopens");
    assert_diff_is_exactly(document.payload(), reopened.payload(), &plan);
    assert_eq!(reopened.towns.len(), 37);
    assert_eq!(reopened.heroes.len(), document.heroes.len());
    let edited = town(&reopened, "Port Balloon");
    let edited_state = edited.state.as_ref().expect("state");
    assert_eq!(edited.mage_guild_level, 3);
    assert_eq!(edited_state.guild_counts, [5, 4, 3, 0, 0]);
    assert_eq!(edited_state.built, state.built | 1 << 2);
    assert_eq!(edited_state.visible & 0x1F, 1 << 2, "only level 3 is drawn");
    assert_eq!(edited_state.visible & !0x1F, state.visible & !0x1F);
    assert_eq!(edited_state.allowed, state.allowed);
    assert_eq!(edited_state.guild_slots, state.guild_slots);
    assert_eq!(edited_state.guild_issue(), None);
    assert_eq!(edited_state.buildings_issue(), None);

    // Remove the Blacksmith and the guild again on the edited save: the
    // payload returns to the original bytes except for the Blacksmith bit.
    let target = edited_state
        .toggle_building(edited_state.built, building::BLACKSMITH, false)
        .and_then(|mask| edited_state.toggle_building(mask, 2, false))
        .expect("removals are allowed");
    let plan = reopened
        .plan_town_edits(edited.id, &[TownEdit::SetBuildings { built: target }])
        .expect("plans");
    let bytes = reopened.serialize_with_patches(&plan).expect("serializes");
    let restored = SaveDocument::open(bytes).expect("reopens");
    assert_diff_is_exactly(reopened.payload(), restored.payload(), &plan);
    let differing: Vec<usize> = document
        .payload()
        .iter()
        .zip(restored.payload())
        .enumerate()
        .filter(|(_, (old, new))| old != new)
        .map(|(offset, _)| offset)
        .collect();
    // Blacksmith is bit 16: byte 2 of the visible and built masks.
    assert_eq!(
        differing,
        vec![tail + layout::VISIBLE + 2, tail + layout::BUILT + 2]
    );
}

#[test]
fn sample_guild_spell_edit_roundtrip() {
    let Some(document) = open_sample_copy("guild") else {
        return;
    };
    let port = town(&document, "Port Balloon");
    let state = port.state.as_ref().expect("state");
    assert_eq!(state.guild_spells(0), vec![37, 53, 15, 31, 35]);
    let tail = port.span.start + town_layout::NAME + port.name_raw.len();

    // Replace Cure (37) with Bless (41) in the built level 1 guild.
    let plan = document
        .plan_town_edits(
            port.id,
            &[TownEdit::SetGuildSpells {
                level: 1,
                spells: vec![41, 53, 15, 31, 35],
            }],
        )
        .expect("plans");
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].offset, tail + layout::SPELLS);
    assert_eq!(&plan[0].expected[..], &37i32.to_le_bytes());
    assert_eq!(&plan[0].replacement[..], &41i32.to_le_bytes());
    let bytes = document.serialize_with_patches(&plan).expect("serializes");
    let reopened = SaveDocument::open(bytes).expect("reopens");
    assert_diff_is_exactly(document.payload(), reopened.payload(), &plan);
    let edited = town(&reopened, "Port Balloon");
    let edited_state = edited.state.as_ref().expect("state");
    assert_eq!(edited_state.guild_spells(0), vec![41, 53, 15, 31, 35]);
    assert_eq!(edited_state.guild_counts, state.guild_counts);
    assert_eq!(edited_state.banned_spells, state.banned_spells);
    assert_eq!(edited_state.guild_issue(), None);

    // Remove two spells from the built level: slots empty out and the count
    // byte follows. Editing an unbuilt level leaves its count at zero.
    let plan = reopened
        .plan_town_edits(
            edited.id,
            &[
                TownEdit::SetGuildSpells {
                    level: 1,
                    spells: vec![41, 53, 15],
                },
                TownEdit::SetGuildSpells {
                    level: 3,
                    spells: vec![49, 63],
                },
            ],
        )
        .expect("plans");
    let bytes = reopened.serialize_with_patches(&plan).expect("serializes");
    let shrunk = SaveDocument::open(bytes).expect("reopens");
    assert_diff_is_exactly(reopened.payload(), shrunk.payload(), &plan);
    let shrunk_state = town(&shrunk, "Port Balloon").state.as_ref().expect("state");
    assert_eq!(shrunk_state.guild_counts, [3, 4, 0, 0, 0]);
    assert_eq!(shrunk_state.guild_slots[0][..5], [41, 53, 15, -1, -1]);
    assert_eq!(
        shrunk_state.guild_slots[0][5], state.guild_slots[0][5],
        "slot past capacity"
    );
    assert_eq!(shrunk_state.guild_spells(2), vec![49, 63]);
    assert_eq!(shrunk_state.guild_issue(), None);

    // Restoring the original lists restores the original payload.
    let plan = shrunk
        .plan_town_edits(
            port.id,
            &[
                TownEdit::SetGuildSpells {
                    level: 1,
                    spells: vec![37, 53, 15, 31, 35],
                },
                TownEdit::SetGuildSpells {
                    level: 3,
                    spells: vec![49, 63, 14],
                },
            ],
        )
        .expect("plans");
    let bytes = shrunk.serialize_with_patches(&plan).expect("serializes");
    let restored = SaveDocument::open(bytes).expect("reopens");
    assert_eq!(restored.payload(), document.payload());
}

#[test]
fn sample_town_state_refusals() {
    let Some(document) = open_sample_copy("refusals") else {
        return;
    };
    let port = town(&document, "Port Balloon");
    let state = port.state.as_ref().expect("state");
    let plan = |edit: TownEdit| document.plan_town_edits(port.id, &[edit]);

    // Unchanged targets plan nothing.
    assert!(
        plan(TownEdit::SetBuildings { built: state.built })
            .expect("no-op")
            .is_empty()
    );
    assert!(
        plan(TownEdit::SetGuildSpells {
            level: 2,
            spells: state.guild_spells(1),
        })
        .expect("no-op")
        .is_empty()
    );
    // Locked structures, unavailable buildings and broken chains.
    for id in [
        building::CAPITOL,
        building::SHIPYARD,
        building::GRAIL,
        building::SPECIAL_22,
    ] {
        assert!(
            plan(TownEdit::SetBuildings {
                built: state.built ^ 1 << id
            })
            .is_err(),
            "{id}"
        );
    }
    assert!(!state.is_allowed(building::TAVERN));
    assert!(
        plan(TownEdit::SetBuildings {
            built: state.built | 1 << building::TAVERN
        })
        .is_err()
    );
    assert!(
        plan(TownEdit::SetBuildings {
            built: state.built | 1 << 3
        })
        .is_err(),
        "MG4 without MG3"
    );
    assert!(
        plan(TownEdit::SetBuildings {
            built: state.built | 0b1100
        })
        .is_err(),
        "Fortress stops at MG3"
    );
    // Spell rules: wrong level, duplicate, capacity, banned (Summon Boat is
    // banned here), guild level that cannot exist, level out of range.
    assert!(state.spell_banned(0));
    for (level, spells) in [
        (1u8, vec![16]),
        (1, vec![15, 15]),
        (1, vec![37, 53, 15, 31, 35, 41]),
        (1, vec![0]),
        (4, vec![38]),
        (6, vec![]),
    ] {
        assert!(plan(TownEdit::SetGuildSpells { level, spells }).is_err());
    }

    // Factory with Mage Guild 4 and 5 both drawn: the guild chain is locked
    // and its visible bits are preserved; other buildings are editable.
    let nostromo = town(&document, "Nostromo");
    let factory = nostromo.state.as_ref().expect("state");
    for level in 0..5u8 {
        assert!(
            factory.building_lock(level).is_some(),
            "guild level {}",
            level + 1
        );
        assert!(
            document
                .plan_town_edits(
                    nostromo.id,
                    &[TownEdit::SetBuildings {
                        built: factory.built & !(0x1F << level & 0x1F)
                    }]
                )
                .is_err()
        );
    }
    let plan = document
        .plan_town_edits(
            nostromo.id,
            &[TownEdit::SetBuildings {
                built: factory.built & !(1 << building::BLACKSMITH),
            }],
        )
        .expect("Blacksmith removal plans");
    let reopened = SaveDocument::open(document.serialize_with_patches(&plan).expect("serializes"))
        .expect("reopens");
    assert_diff_is_exactly(document.payload(), reopened.payload(), &plan);
    let edited = town(&reopened, "Nostromo").state.as_ref().expect("state");
    assert_eq!(edited.visible & 0x1F, 0b11000, "levels 4 and 5 stay drawn");
    assert_eq!(
        edited.visible,
        factory.visible & !(1 << building::BLACKSMITH)
    );
    assert_eq!(edited.built, factory.built & !(1 << building::BLACKSMITH));

    // A garrisoned hero blocks only the garrison, not the town's buildings.
    let occupied = document
        .towns
        .iter()
        .find(|town| {
            town.garrison_hero.is_some()
                && town.state.as_ref().is_some_and(|state| {
                    state.building_lock(building::BLACKSMITH).is_none()
                        && state.is_built(building::BLACKSMITH)
                })
        })
        .expect("a hero-occupied town with an editable Blacksmith");
    let occupied_state = occupied.state.as_ref().expect("state");
    assert!(
        document
            .plan_town_edits(
                occupied.id,
                &[TownEdit::SetGarrisonSlot {
                    slot: 0,
                    creature_id: Some(0),
                    count: 1
                }]
            )
            .is_err()
    );
    assert_eq!(
        document
            .plan_town_edits(
                occupied.id,
                &[TownEdit::SetBuildings {
                    built: occupied_state.built & !(1 << building::BLACKSMITH)
                }]
            )
            .expect("building edit plans")
            .len(),
        2
    );
}

#[test]
fn sample_horde_and_shipyard_roundtrip() {
    let Some(document) = open_sample_copy("horde") else {
        return;
    };
    // Every built horde of a verified faction is stored in the editable
    // form: both flags set, the upgraded flag drawn, its dwelling hidden.
    let mut verified_hordes = 0;
    let mut unverified_hordes = Vec::new();
    for town in &document.towns {
        let state = town.state.as_ref().expect("state");
        for base in [building::HORDE_1, building::HORDE_2] {
            if !state.is_built(base) {
                continue;
            }
            match state.building_lock(base) {
                None => verified_hordes += 1,
                Some(reason) => unverified_hordes.push((town.name_lossy.clone(), base, reason)),
            }
        }
    }
    assert_eq!(verified_hordes, 25);
    // Cove's Roost (second horde) cannot be told apart from the Gunpowder
    // Warehouse in the sample and stays locked.
    assert_eq!(unverified_hordes.len(), 4);
    assert!(
        unverified_hordes
            .iter()
            .all(|(_, base, reason)| *base == building::HORDE_2 && reason.contains("not verified"))
    );

    // Brill (Necropolis): remove Unearthed Graves, then build it again.
    let brill = town(&document, "Brill");
    let state = brill.state.as_ref().expect("state");
    let graves = 1u64 << building::HORDE_1 | 1 << building::HORDE_1_UPGRADED;
    let skeleton_upgrade = 1u64 << building::DWELLING_1_UPGRADED;
    assert_eq!(state.built & graves, graves);
    assert_eq!(
        state.visible & (graves | skeleton_upgrade),
        1 << building::HORDE_1_UPGRADED
    );
    let target = state
        .toggle_building(state.built, building::HORDE_1, false)
        .expect("horde removal is allowed");
    assert_eq!(target, state.built & !graves);
    let plan = document
        .plan_town_edits(brill.id, &[TownEdit::SetBuildings { built: target }])
        .expect("plans");
    assert_eq!(plan.len(), 2, "visible and built masks only");
    let removed = SaveDocument::open(document.serialize_with_patches(&plan).expect("serializes"))
        .expect("reopens");
    assert_diff_is_exactly(document.payload(), removed.payload(), &plan);
    let removed_state = town(&removed, "Brill").state.as_ref().expect("state");
    assert_eq!(
        removed_state.visible & (graves | skeleton_upgrade),
        skeleton_upgrade
    );
    assert_eq!(removed_state.buildings_issue(), None);
    assert_eq!(removed_state.guild_slots, state.guild_slots);
    let target = removed_state
        .toggle_building(removed_state.built, building::HORDE_1_UPGRADED, true)
        .expect("horde construction is allowed");
    let plan = removed
        .plan_town_edits(brill.id, &[TownEdit::SetBuildings { built: target }])
        .expect("plans");
    let rebuilt = SaveDocument::open(removed.serialize_with_patches(&plan).expect("serializes"))
        .expect("reopens");
    assert_eq!(
        rebuilt.payload(),
        document.payload(),
        "original bytes restored"
    );

    // Reptile Promnt (Castle on the coast): remove the Shipyard and rebuild.
    let reptile = town(&document, "Reptile Promnt");
    let state = reptile.state.as_ref().expect("state");
    assert_eq!(state.boat, Some((23, 190)));
    assert!(state.is_built(building::SHIPYARD) && !state.is_built(building::SHIP));
    let target = state
        .toggle_building(state.built, building::SHIPYARD, false)
        .expect("shipyard removal is allowed");
    let plan = document
        .plan_town_edits(reptile.id, &[TownEdit::SetBuildings { built: target }])
        .expect("plans");
    let removed = SaveDocument::open(document.serialize_with_patches(&plan).expect("serializes"))
        .expect("reopens");
    let tail = reptile.span.start + town_layout::NAME + reptile.name_raw.len();
    let differing: Vec<usize> = document
        .payload()
        .iter()
        .zip(removed.payload())
        .enumerate()
        .filter(|(_, (old, new))| old != new)
        .map(|(offset, _)| offset)
        .collect();
    assert_eq!(
        differing,
        vec![tail + layout::VISIBLE, tail + layout::BUILT]
    );
    let removed_state = town(&removed, "Reptile Promnt")
        .state
        .as_ref()
        .expect("state");
    assert_eq!(removed_state.boat, state.boat, "boat tile untouched");
    let target = removed_state
        .toggle_building(removed_state.built, building::SHIPYARD, true)
        .expect("shipyard construction is allowed");
    let plan = removed
        .plan_town_edits(reptile.id, &[TownEdit::SetBuildings { built: target }])
        .expect("plans");
    let rebuilt = SaveDocument::open(removed.serialize_with_patches(&plan).expect("serializes"))
        .expect("reopens");
    assert_eq!(rebuilt.payload(), document.payload());

    // Still refused: no boat tile, prohibited by the map, the Factory slot,
    // and every fortification level.
    let no_water = town(&document, "Port Royal").state.as_ref().expect("state");
    assert_eq!(no_water.boat, None);
    assert!(no_water.building_lock(building::SHIPYARD).is_some());
    let prohibited = town(&document, "FlyingDutchman")
        .state
        .as_ref()
        .expect("state");
    assert!(prohibited.boat.is_some() && !prohibited.is_allowed(building::SHIPYARD));
    assert!(
        prohibited
            .toggle_building(prohibited.built, building::SHIPYARD, true)
            .is_err()
    );
    let nautilus = town(&document, "Nautilus").state.as_ref().expect("state");
    assert!(nautilus.building_lock(building::SHIPYARD).is_some());
    let brill_state = brill.state.as_ref().expect("state");
    for id in [
        building::FORT,
        building::CITADEL,
        building::CASTLE,
        building::CAPITOL,
        building::GRAIL,
    ] {
        assert!(brill_state.building_lock(id).is_some(), "{id}");
    }
}
