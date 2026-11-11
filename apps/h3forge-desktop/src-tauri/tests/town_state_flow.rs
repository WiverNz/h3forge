//! Town building and Mage Guild editing through the plain session state
//! (H3F-022, no Tauri runtime): pending overlay, toggle-back, discard, Save
//! As with an exact payload diff, and the town icon command. Opt-in via
//! `H3FORGE_FIXTURE_ROOT` (the `games` folder); skips when the sample is
//! absent. Only a private copy under `.local/fixtures/` is opened and all
//! output goes to an OS temporary directory.

use std::path::{Path, PathBuf};

use h3forge_desktop::dto::{BuildingEditDto, GuildEditDto, TownEditRequestDto};
use h3forge_desktop::session::AppState;
use h3forge_save::SaveDocument;
use h3forge_save::definitions::towns;
use h3forge_save::town_state::layout;

fn sample_copy(tag: &str) -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from)?;
    let source = root.join("SaAndVla.GM2");
    if !source.is_file() {
        return None;
    }
    let destination = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.local/fixtures");
    std::fs::create_dir_all(&destination).ok()?;
    let destination = destination.join(format!("SaAndVla-town-state-flow-{tag}.GM2"));
    let _ = std::fs::remove_file(&destination);
    std::fs::copy(&source, &destination).ok()?;
    Some(destination)
}

fn repo(relative: &str) -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(relative)
        .canonicalize()
        .ok()
}

fn open_sample(state: &mut AppState, copy: &Path) -> bool {
    let has_installation =
        repo("Data").is_some_and(|data| state.set_manual_installation(data).is_ok());
    let grant = state.issue_grant(copy.to_path_buf());
    state
        .open_granted_save(&grant.grant_id)
        .expect("sample opens");
    has_installation
}

fn town_id(state: &AppState, name: &str) -> usize {
    state
        .list_towns()
        .expect("towns listed")
        .iter()
        .find(|town| town.name.lossy == name)
        .unwrap_or_else(|| panic!("{name} present"))
        .id
}

fn build(id: u8, built: bool) -> TownEditRequestDto {
    TownEditRequestDto {
        building: Some(BuildingEditDto { id, built }),
        ..TownEditRequestDto::default()
    }
}

fn guild(level: u8, spells: &[u8]) -> TownEditRequestDto {
    TownEditRequestDto {
        guild: Some(GuildEditDto {
            level,
            spells: spells.to_vec(),
        }),
        ..TownEditRequestDto::default()
    }
}

#[test]
fn building_and_guild_edits_flow_to_a_saved_copy() {
    let Some(copy) = sample_copy("ok") else {
        eprintln!("no private sample copy; skipping town state flow test");
        return;
    };
    let original_bytes = std::fs::read(&copy).unwrap();
    let mut state = AppState::new();
    let has_installation = open_sample(&mut state, &copy);
    let base = state.session_ref().unwrap().document.clone();
    let id = town_id(&state, "Port Balloon");

    // Read-only projection of the stored state.
    let initial = state.get_town_edit_state(id).expect("edit state");
    let buildings = initial.buildings.as_ref().expect("buildings decoded");
    assert_eq!(buildings.read_only_reason, None);
    assert_eq!(buildings.items.len(), 44);
    let item = |id: usize| &buildings.items[id];
    assert!(item(1).built && item(1).editable, "Mage Guild 2 built");
    assert!(
        !item(2).built && item(2).allowed && item(2).editable,
        "Mage Guild 3 available"
    );
    assert!(
        !item(3).allowed && !item(3).editable,
        "Fortress stops at level 3"
    );
    assert!(
        !item(5).allowed && !item(5).editable,
        "Tavern prohibited here"
    );
    assert!(item(7).built && !item(7).editable, "Fort is locked");
    assert!(!item(13).editable, "Capitol is locked");
    assert_eq!(item(17).name, "Cage of Warlords");
    assert_eq!(item(2).group, "mageGuild");
    let guild_state = initial.guild.as_ref().expect("guild decoded");
    assert_eq!((guild_state.built_level, guild_state.max_level), (2, 3));
    assert!(guild_state.levels[0].built && guild_state.levels[0].editable);
    assert!(!guild_state.levels[2].built && guild_state.levels[2].editable);
    assert!(!guild_state.levels[3].available && !guild_state.levels[3].editable);
    let spell_ids =
        |level: usize, response: &h3forge_desktop::dto::TownEditResponseDto| -> Vec<usize> {
            response.guild.as_ref().unwrap().levels[level]
                .spells
                .iter()
                .map(|spell| spell.id)
                .collect()
        };
    assert_eq!(spell_ids(0, &initial), vec![37, 53, 15, 31, 35]);
    assert!(
        guild_state.spell_choices[0].banned,
        "Summon Boat banned in this town"
    );

    // Build Mage Guild 3: pending overlay, nothing written.
    let response = state
        .apply_town_request(id, &build(2, true))
        .expect("build applies");
    let after = response.buildings.as_ref().unwrap();
    assert!(after.items[2].built && after.items[2].dirty);
    assert_eq!(response.guild.as_ref().unwrap().built_level, 3);
    assert!(response.guild.as_ref().unwrap().levels[2].built);
    assert_eq!(response.dirty_fields, vec!["Buildings"]);
    assert_eq!(std::fs::read(&copy).unwrap(), original_bytes);

    // Toggling it back restores the base state and clears the pending edit.
    let response = state
        .apply_town_request(id, &build(2, false))
        .expect("toggle back");
    assert!(response.dirty_fields.is_empty());
    assert!(state.pending_edits().unwrap().is_empty());

    // Refusals leave the pending set untouched.
    assert!(
        state.apply_town_request(id, &build(13, true)).is_err(),
        "Capitol"
    );
    assert!(
        state.apply_town_request(id, &build(5, true)).is_err(),
        "Tavern prohibited"
    );
    assert!(
        state.apply_town_request(id, &guild(1, &[0])).is_err(),
        "banned spell"
    );
    assert!(
        state.apply_town_request(id, &guild(1, &[16])).is_err(),
        "wrong level"
    );
    assert!(state.pending_edits().unwrap().is_empty());

    // Discard drops both kinds of town edit.
    state.apply_town_request(id, &build(2, true)).unwrap();
    state
        .apply_town_request(id, &guild(1, &[41, 53, 15, 31, 35]))
        .unwrap();
    assert_eq!(state.pending_edits().unwrap().len(), 2);
    assert!(state.discard_town_edits(id).unwrap().is_empty());
    let reverted = state.get_town_edit_state(id).unwrap();
    assert_eq!(spell_ids(0, &reverted), vec![37, 53, 15, 31, 35]);
    assert!(!reverted.buildings.as_ref().unwrap().items[2].built);

    // Apply both again and Save As.
    state.apply_town_request(id, &build(2, true)).unwrap();
    let response = state
        .apply_town_request(id, &guild(1, &[41, 53, 15, 31, 35]))
        .unwrap();
    assert_eq!(spell_ids(0, &response), vec![41, 53, 15, 31, 35]);
    assert!(response.guild.as_ref().unwrap().levels[0].dirty);
    let pending = state.pending_edits().unwrap();
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|edit| edit.record_kind == "town"));
    assert!(
        pending
            .iter()
            .any(|edit| edit.after == "build Mage Guild Level 3")
    );
    assert!(
        pending
            .iter()
            .any(|edit| edit.before.starts_with("Cure") && edit.after.starts_with("Bless"))
    );

    let dir = std::env::temp_dir().join(format!("h3forge-town-state-flow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("SaAndVla town state (edited).GM2");
    state.save_save_as(&dest).expect("save succeeds");
    assert_eq!(
        std::fs::read(&copy).unwrap(),
        original_bytes,
        "source untouched"
    );
    assert!(state.pending_edits().unwrap().is_empty());

    // Reopen the written file: the diff is exactly the intended bytes.
    let reopened = SaveDocument::open(std::fs::read(&dest).unwrap()).expect("reopens");
    let town = base
        .towns
        .iter()
        .find(|town| town.name_lossy == "Port Balloon")
        .unwrap();
    let start = town.span.start;
    let tail = start + towns::NAME + town.name_raw.len();
    let changed: Vec<usize> = base
        .payload()
        .iter()
        .zip(reopened.payload())
        .enumerate()
        .filter(|(_, (before, after))| before != after)
        .map(|(offset, _)| offset)
        .collect();
    assert_eq!(
        changed,
        vec![
            start + towns::MAGE_GUILD_LEVEL,
            tail + layout::GUILD_COUNTS + 2,
            tail + layout::VISIBLE,
            tail + layout::BUILT,
            tail + layout::SPELLS,
        ]
    );
    assert_eq!(base.payload().len(), reopened.payload().len());
    let edited = reopened.towns[town.id].state.as_ref().unwrap();
    assert_eq!(reopened.towns[town.id].mage_guild_level, 3);
    assert_eq!(edited.guild_counts, [5, 4, 3, 0, 0]);
    assert_eq!(edited.guild_spells(0), vec![41, 53, 15, 31, 35]);
    assert_eq!(edited.guild_issue(), None);
    assert_eq!(edited.buildings_issue(), None);
    assert_eq!(reopened.towns.len(), 37);
    assert_eq!(reopened.heroes.len(), base.heroes.len());

    // The session now sits on the saved file with the edits as its base.
    let saved = state.get_town_edit_state(id).unwrap();
    assert!(saved.dirty_fields.is_empty());
    assert_eq!(saved.guild.as_ref().unwrap().built_level, 3);

    // Town icons: every sample town type resolves from the installation.
    if has_installation {
        for town in state.list_towns().unwrap() {
            for large in [true, false] {
                let icon = state
                    .town_icon(town.town_type, town.has_fort, large)
                    .unwrap();
                assert!(icon.resolved, "{} icon", town.name.lossy);
            }
        }
        assert!(!state.town_icon(12, true, true).unwrap().resolved);
        // Building pictures: Fortress Mage Guild 1 and Cage of Warlords
        // resolve, Fortress has no Mage Guild 4, Cove and Bulwark come from
        // HotA.lod, and unknown types never borrow a picture.
        for (town_type, building, resolved) in [
            (7u8, 0u8, true),
            (7, 17, true),
            (7, 3, false),
            (9, 23, true),
            (10, 6, true),
            (11, 21, true),
            (11, 4, false),
            (12, 0, false),
            (0, 44, false),
        ] {
            assert_eq!(
                state.building_icon(town_type, building).unwrap().resolved,
                resolved,
                "type {town_type} building {building}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn garrisoned_hero_blocks_only_the_garrison() {
    let Some(copy) = sample_copy("hero") else {
        return;
    };
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    // Brill: a garrisoned hero owns the garrison.
    let id = town_id(&state, "Brill");
    let edit_state = state.get_town_edit_state(id).unwrap();
    assert!(edit_state.garrisoned_hero.is_some());
    assert!(edit_state.garrison_read_only_reason.is_some());
    let buildings = edit_state.buildings.as_ref().unwrap();
    assert_eq!(buildings.read_only_reason, None);
    let blacksmith = &buildings.items[16];
    assert!(blacksmith.built && blacksmith.editable);
    // A horde building is built, so existing dwellings cannot be removed.
    assert!(buildings.items[31].built && !buildings.items[31].editable);
    let response = state
        .apply_town_request(id, &build(16, false))
        .expect("building edit applies");
    assert!(!response.buildings.as_ref().unwrap().items[16].built);
    assert_eq!(response.dirty_fields, vec!["Buildings"]);

    // Factory drawing Mage Guild 4 beside level 5: only the guild chain is
    // locked; other buildings and the guild spells stay editable.
    let factory = town_id(&state, "Nostromo");
    let edit_state = state.get_town_edit_state(factory).unwrap();
    let buildings = edit_state.buildings.as_ref().unwrap();
    assert_eq!(buildings.read_only_reason, None);
    assert!(
        buildings.items[..5]
            .iter()
            .all(|item| item.built && !item.editable)
    );
    assert!(buildings.items[16].editable);
    assert_eq!(buildings.items[6].name, "Mana Generator");
    assert_eq!(buildings.items[18].name, "Pen");
    assert_eq!(buildings.items[32].name, "Ranch");
    assert_eq!(edit_state.guild.as_ref().unwrap().read_only_reason, None);
    assert!(state.apply_town_request(factory, &build(4, false)).is_err());
    assert!(state.apply_town_request(factory, &build(16, false)).is_ok());
}

#[test]
fn garrison_hero_reference_keeps_raw_cp1251_name_bytes() {
    let Some(copy) = sample_copy("names") else {
        return;
    };
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    // Tadorn Fens is garrisoned by a hero whose name is stored in CP1251
    // ("Сандро"); it is not valid UTF-8, so a lossy string shows mojibake.
    let id = town_id(&state, "Tadorn Fens");
    let expected: &[u8] = &[0xd1, 0xe0, 0xed, 0xe4, 0xf0, 0xee];
    let detail = state.get_town(id).unwrap();
    let hero = detail.garrisoned_hero.expect("garrisoned hero");
    assert_eq!(hero.name.raw, expected);
    assert!(std::str::from_utf8(&hero.name.raw).is_err());
    // Same bytes as the hero record the reference resolves to.
    let record = &state.session_ref().unwrap().document.heroes[hero.id];
    assert_eq!(record.name_raw, expected);
    let edit_state = state.get_town_edit_state(id).unwrap();
    assert_eq!(edit_state.garrisoned_hero.unwrap().name.raw, expected);
    // Backend messages no longer embed a lossily decoded hero name.
    let reason = edit_state.garrison_read_only_reason.unwrap();
    assert!(reason.is_ascii(), "{reason}");
    // Pending edits carry raw record names as well.
    state.apply_town_request(id, &build(16, false)).unwrap();
    let pending = state.pending_edits().unwrap();
    assert_eq!(pending[0].record_name.raw, b"Tadorn Fens");
}

#[test]
fn garrisoned_visiting_and_independent_town_armies() {
    let Some(copy) = sample_copy("armies") else {
        return;
    };
    let original_bytes = std::fs::read(&copy).unwrap();
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let base = state.session_ref().unwrap().document.clone();

    // Every town with a garrisoned hero stores an empty garrison of its own:
    // the hero's army is the garrison.
    let mut garrisoned = 0;
    for town in &base.towns {
        if town.garrison_hero.is_some() {
            garrisoned += 1;
            assert!(
                town.garrison.iter().all(|slot| slot.is_empty()),
                "{}",
                town.name_lossy
            );
        }
    }
    assert_eq!(garrisoned, 17);

    // Mount Franklin: garrisoned hero. The town slots stay closed, the
    // hero's real stacks are offered instead and resolve by record ID.
    let id = town_id(&state, "Mount Franklin");
    let edit_state = state.get_town_edit_state(id).unwrap();
    assert!(
        edit_state
            .garrison
            .iter()
            .all(|slot| slot.empty && !slot.editable)
    );
    let hero_ref = edit_state
        .garrisoned_hero
        .as_ref()
        .expect("garrisoned hero");
    let army = edit_state
        .garrison_hero_army
        .as_ref()
        .expect("hero army offered");
    assert_eq!(army.hero_id, hero_ref.id);
    assert_eq!(army.hero_id, base.towns[id].garrison_hero.unwrap());
    let hero = &base.heroes[army.hero_id];
    assert_eq!(
        hero.portrait.map(|identity| identity.hero_type),
        base.towns[id].garrisoned_hero_type
    );
    assert_eq!(army.read_only_reason, None);
    assert!(
        army.slots.iter().any(|slot| !slot.empty),
        "the hero carries troops"
    );
    for (slot, stored) in army.slots.iter().zip(&hero.army) {
        assert_eq!(
            (slot.creature_id, slot.count),
            (stored.creature_id, stored.count)
        );
    }

    // Editing that hero's army (what the town screen does) and a town
    // building together: both stay pending across town and hero views.
    let slot = army
        .slots
        .iter()
        .position(|slot| !slot.empty && slot.editable)
        .unwrap();
    let creature_id = army.slots[slot].creature_id;
    let new_count = army.slots[slot].count + 5;
    state
        .apply_hero_edit(
            army.hero_id,
            vec![h3forge_save::HeroEdit::SetArmySlot {
                slot: slot as u8,
                creature_id: Some(creature_id),
                count: new_count,
            }],
            Vec::new(),
        )
        .expect("hero army edit applies");
    state
        .apply_town_request(id, &build(16, false))
        .expect("town edit applies");
    let after = state.get_town_edit_state(id).unwrap();
    let army = after.garrison_hero_army.as_ref().unwrap();
    assert_eq!(army.slots[slot].count, new_count);
    assert_eq!(army.dirty_fields, vec![format!("Army slot {}", slot + 1)]);
    assert_eq!(after.dirty_fields, vec!["Buildings"]);
    let hero_view = state.hero_edit_state(army.hero_id).unwrap();
    assert_eq!(hero_view.army[slot].count, new_count);
    let pending = state.pending_edits().unwrap();
    assert_eq!(pending.len(), 2);
    assert!(
        pending
            .iter()
            .any(|edit| edit.record_kind == "hero" && edit.record_id == army.hero_id)
    );
    assert!(
        pending
            .iter()
            .any(|edit| edit.record_kind == "town" && edit.record_id == id)
    );
    // A direct town garrison write is still refused while the hero is inside.
    assert!(
        state
            .apply_town_edit(
                id,
                vec![h3forge_save::TownEdit::SetGarrisonSlot {
                    slot: 0,
                    creature_id: Some(0),
                    count: 1
                }]
            )
            .is_err()
    );
    assert_eq!(state.pending_edits().unwrap().len(), 2);

    // OldtownLibrary: only a visiting hero. The garrison is the town's own
    // and stays directly editable; no hero army is substituted.
    let visited = town_id(&state, "OldtownLibrary");
    let visited_state = state.get_town_edit_state(visited).unwrap();
    assert!(visited_state.visiting_hero.is_some() && visited_state.garrisoned_hero.is_none());
    assert!(visited_state.garrison_hero_army.is_none());
    assert_eq!(visited_state.garrison_read_only_reason, None);
    let response = state
        .apply_town_edit(
            visited,
            vec![h3forge_save::TownEdit::SetGarrisonSlot {
                slot: 0,
                creature_id: Some(0),
                count: 9,
            }],
        )
        .expect("independent garrison beside a visiting hero is editable");
    assert_eq!(response.garrison[0].count, 9);

    // Port Balloon: no hero at all, independent garrison as before.
    let independent = state
        .get_town_edit_state(town_id(&state, "Port Balloon"))
        .unwrap();
    assert!(independent.garrison_hero_army.is_none());
    assert_eq!(independent.garrison_read_only_reason, None);
    assert!(independent.garrison[0].editable);
    assert_eq!(
        std::fs::read(&copy).unwrap(),
        original_bytes,
        "nothing written"
    );
}
