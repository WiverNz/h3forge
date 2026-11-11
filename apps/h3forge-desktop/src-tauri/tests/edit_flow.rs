//! End-to-end editing flow through the plain session state (no Tauri
//! runtime): open a private sample copy, apply semantic hero edits, Save As
//! into a temp directory, and verify the written file, the recovery backup,
//! the untouched original and the failure paths. Opt-in via
//! `H3FORGE_FIXTURE_ROOT`; skips cleanly when the sample is absent.

use std::path::{Path, PathBuf};

use h3forge_desktop::session::AppState;
use h3forge_save::{HeroEdit, SaveDocument};

fn sample_copy(tag: &str) -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from)?;
    let source = root.join("SaAndVla.GM2");
    if !source.is_file() {
        return None;
    }
    let destination = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.local/fixtures");
    std::fs::create_dir_all(&destination).ok()?;
    // Per-test destination: the three tests run in parallel threads and must
    // not share one copy.
    let destination = destination.join(format!("SaAndVla-edit-flow-{tag}.GM2"));
    std::fs::copy(&source, &destination).ok()?;
    Some(destination)
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("h3forge-edit-flow-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn open_sample(state: &mut AppState, copy: &Path) {
    let grant = state.issue_grant(copy.to_path_buf());
    state
        .open_granted_save(&grant.grant_id)
        .expect("sample opens");
}

/// Research candidate: the owned, spellbook-bearing record at span 4,727,078.
fn candidate_id(state: &AppState) -> usize {
    (0..)
        .find(|&id| {
            state
                .get_hero(id)
                .map(|hero| hero.span_start == 4_727_078)
                .unwrap_or(false)
        })
        .expect("research candidate present")
}

#[test]
fn save_as_roundtrip_backup_and_original_intact() {
    let Some(copy) = sample_copy("ok") else {
        eprintln!("no private sample copy; skipping edit flow test");
        return;
    };
    let dir = temp_dir("ok");
    let original_bytes = std::fs::read(&copy).unwrap();

    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let id = candidate_id(&state);
    let response = state
        .apply_hero_edit(
            id,
            vec![
                HeroEdit::SetAttack(9),
                HeroEdit::SetSpellPoints(99),
                HeroEdit::SetSpellLearned {
                    spell_id: 0,
                    learned: true,
                },
            ],
            Vec::new(),
        )
        .expect("edits apply");
    assert!(response.editable);
    assert_eq!(response.dirty_fields.len(), 3);
    assert!(response.spells_learned.iter().any(|spell| spell.id == 0));

    let dest = dir.join("SaAndVla (edited).GM2");
    let summary = state.save_save_as(&dest).expect("save succeeds");
    assert_eq!(summary.file_name, "SaAndVla (edited).GM2");

    // Original untouched; output written; backup holds the original bytes.
    assert_eq!(std::fs::read(&copy).unwrap(), original_bytes);
    let written = std::fs::read(&dest).expect("output exists");
    let backup = dir
        .read_dir()
        .unwrap()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(".backup-"))
        })
        .expect("recovery backup written");
    assert_eq!(std::fs::read(&backup).unwrap(), original_bytes);
    assert!(!dir.join(".h3forge-save.tmp").exists());

    // The written file reopens with the modified values and a payload diff
    // limited to the planned offsets.
    let base = SaveDocument::open(original_bytes.clone()).unwrap();
    let reopened = SaveDocument::open(written).unwrap();
    assert_eq!(reopened.payload().len(), base.payload().len());
    let edited = reopened
        .heroes
        .iter()
        .find(|hero| hero.span.start == 4_727_078)
        .unwrap();
    assert_eq!(edited.primary.attack, 9);
    assert_eq!(edited.spell_points, 99);
    assert_eq!(edited.spells_learned[0], 1);
    assert_eq!(edited.spells_available[0], 1);
    let mut changed = Vec::new();
    for (index, (before, after)) in base.payload().iter().zip(reopened.payload()).enumerate() {
        if before != after {
            changed.push(index);
        }
    }
    // Four patched offsets: attack (1 byte), spell points (one u16 patch),
    // learned flag and available flag for spell 0.
    assert_eq!(changed.len(), 4, "diff is exactly the planned patch union");

    // Session re-based: pending clean, further save is a no-op.
    assert!(state.pending_edits().unwrap().is_empty());
    let error = state.save_save_as(&dir.join("again.GM2")).unwrap_err();
    assert_eq!(error.code, "nothing-to-save");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn save_as_failure_paths_keep_edits_and_original() {
    let Some(copy) = sample_copy("fail") else {
        eprintln!("no private sample copy; skipping edit flow test");
        return;
    };
    let dir = temp_dir("fail");
    let original_bytes = std::fs::read(&copy).unwrap();

    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let id = candidate_id(&state);
    state
        .apply_hero_edit(id, vec![HeroEdit::SetAttack(9)], Vec::new())
        .unwrap();

    // Existing destination is never overwritten.
    let existing = dir.join("taken.GM2");
    std::fs::write(&existing, b"occupied").unwrap();
    let error = state.save_save_as(&existing).unwrap_err();
    assert_eq!(error.code, "file-exists");
    assert_eq!(std::fs::read(&existing).unwrap(), b"occupied");
    assert_eq!(state.pending_edits().unwrap().len(), 1, "edits retained");

    // Changed source is detected before anything is written.
    std::fs::write(&copy, &original_bytes).unwrap(); // same bytes, new mtime
    let error = state
        .save_save_as(&dir.join("after-change.GM2"))
        .unwrap_err();
    assert_eq!(error.code, "source-changed");
    assert!(!dir.join("after-change.GM2").exists());
    assert!(
        !dir.read_dir()
            .unwrap()
            .filter_map(|entry| entry.ok())
            .any(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.contains(".backup-"))
            })
    );

    // The open source file itself can never be the destination.
    let error = state.save_save_as(&copy).unwrap_err();
    assert_eq!(error.code, "same-file");

    // Repository games/ and Data/ are hard-guarded destinations.
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let error = state
        .save_save_as(&repo.join("games").join("out.GM2"))
        .unwrap_err();
    assert_eq!(error.code, "forbidden-destination");
    let error = state
        .save_save_as(&repo.join("Data").join("out.GM2"))
        .unwrap_err();
    assert_eq!(error.code, "forbidden-destination");

    // Original still intact; edits still pending and usable after re-open.
    assert_eq!(std::fs::read(&copy).unwrap(), original_bytes);
    assert_eq!(state.pending_edits().unwrap().len(), 1);
    std::fs::write(&copy, &original_bytes).unwrap();
    let grant = state.issue_grant(copy.clone());
    state.open_granted_save(&grant.grant_id).unwrap();
    let id = candidate_id(&state);
    state
        .apply_hero_edit(id, vec![HeroEdit::SetAttack(9)], Vec::new())
        .unwrap();
    let dest = dir.join("recovered.GM2");
    state
        .save_save_as(&dest)
        .expect("save succeeds after re-open");
    assert_eq!(std::fs::read(&copy).unwrap(), original_bytes);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_spellbook_learn_warns_but_records() {
    let Some(copy) = sample_copy("warn") else {
        eprintln!("no private sample copy; skipping edit flow test");
        return;
    };
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let id = (0..)
        .find(|&id| {
            state
                .get_hero(id)
                .map(|hero| hero.confidence == "high" && !hero.has_spellbook)
                .unwrap_or(false)
        })
        .expect("a confirmed hero without a spellbook exists");
    let response = state
        .apply_hero_edit(
            id,
            vec![HeroEdit::SetSpellLearned {
                spell_id: 5,
                learned: true,
            }],
            Vec::new(),
        )
        .unwrap();
    assert_eq!(response.warnings.len(), 1);
    assert!(response.warnings[0].contains("no spellbook"));
    assert!(response.spells_learned.iter().any(|spell| spell.id == 5));
    // Discarding returns the hero to its base state.
    let pending = state.discard_hero_edits(id).unwrap();
    assert!(pending.is_empty());
    let response = state.hero_edit_state(id).unwrap();
    assert!(!response.spells_learned.iter().any(|spell| spell.id == 5));
    assert!(response.dirty_fields.is_empty());
}

/// Army edits share the existing safe Save As, dirty-state and discard path.
#[test]
fn army_sample_roundtrip_and_discard() {
    if std::env::var_os("H3FORGE_FIXTURE_ROOT").is_none() {
        eprintln!("no fixture root; skipping army sample roundtrip");
        return;
    }
    let copy = sample_copy("army").expect("configured SaAndVla sample must copy successfully");
    let original_bytes = std::fs::read(&copy).unwrap();
    let base = SaveDocument::open(original_bytes.clone()).unwrap();
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let id = candidate_id(&state);
    let original_slot = base.heroes[id].army[0];
    let change = HeroEdit::SetArmySlot {
        slot: 0,
        creature_id: Some(153),
        count: 1234,
    };
    let response = state
        .apply_hero_edit(id, vec![change.clone()], vec![])
        .unwrap();
    assert_eq!(response.army[0].name, Some("Nymph"));
    assert_eq!(response.army[0].count, 1234);
    assert_eq!(response.dirty_fields, ["Army slot 1"]);
    assert!(
        state
            .apply_hero_edit(
                id,
                vec![HeroEdit::SetArmySlot {
                    slot: 9,
                    creature_id: None,
                    count: 0
                }],
                vec![]
            )
            .is_err()
    );
    assert_eq!(
        state.hero_edit_state(id).unwrap().army[0].count,
        1234,
        "invalid request leaves pending state intact"
    );
    state
        .apply_hero_edit(
            id,
            vec![HeroEdit::SetArmySlot {
                slot: 0,
                creature_id: Some(original_slot.creature_id),
                count: original_slot.count,
            }],
            vec![],
        )
        .unwrap();
    assert!(
        state.pending_edits().unwrap().is_empty(),
        "reverting original removes dirty state"
    );
    state
        .apply_hero_edit(id, vec![change.clone()], vec![])
        .unwrap();
    state.discard_all_edits().unwrap();
    assert_eq!(
        state.hero_edit_state(id).unwrap().army[0].count,
        original_slot.count
    );
    state
        .apply_hero_edit(id, vec![change.clone()], vec![])
        .unwrap();
    state.discard_hero_edits(id).unwrap();
    assert!(state.pending_edits().unwrap().is_empty());

    let edits = vec![
        change.clone(),
        HeroEdit::SetArmySlot {
            slot: 1,
            creature_id: None,
            count: 0,
        },
        HeroEdit::SetArmySlot {
            slot: 6,
            creature_id: Some(199),
            count: 7,
        },
        HeroEdit::SetAttack(9),
        HeroEdit::SetSpellLearned {
            spell_id: 0,
            learned: true,
        },
    ];
    state.apply_hero_edit(id, edits.clone(), vec![]).unwrap();
    let dir = temp_dir("army");
    let dest = dir.join("SaAndVla-army.GM2");
    state.save_save_as(&dest).unwrap();
    let reopened = SaveDocument::open(std::fs::read(&dest).unwrap()).unwrap();
    let edited = &reopened.heroes[id];
    assert_eq!(edited.army[0].creature_id, 153);
    assert_eq!(edited.army[0].count, 1234);
    assert_eq!(edited.army[1].creature_id, u32::MAX);
    assert_eq!(edited.army[1].count, 0);
    assert_eq!(edited.army[6].creature_id, 199);
    assert_eq!(edited.army[6].count, 7);
    assert_eq!(edited.primary.attack, 9);
    assert_eq!(edited.spells_learned[0], 1);
    assert_eq!(base.payload().len(), reopened.payload().len());
    let plan = base.plan_hero_edits(id, &edits).unwrap();
    let mut expected = base.payload().to_vec();
    h3forge_save::apply_patches(&mut expected, &plan).unwrap();
    assert_eq!(
        expected,
        reopened.payload(),
        "exact planned payload and no unrelated changes"
    );
    assert_eq!(std::fs::read(&copy).unwrap(), original_bytes);
    let backup = dir
        .read_dir()
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.to_string_lossy().contains(".backup-"))
        .expect("backup exists");
    assert_eq!(std::fs::read(backup).unwrap(), original_bytes);
    assert!(state.pending_edits().unwrap().is_empty());
    eprintln!(
        "army sample verified: {} heroes, output {}",
        reopened.heroes.len(),
        dest.display()
    );
}

#[test]
fn army_request_bounds_rejected_at_typed_boundary() {
    use h3forge_desktop::dto::HeroEditRequestDto;
    for json in [
        r#"{"army":{"slot":0,"creatureId":0,"count":4294967296}}"#,
        r#"{"army":{"slot":0,"creatureId":0,"count":-1}}"#,
        r#"{"army":{"slot":0,"creatureId":0,"count":1.5}}"#,
        r#"{"army":{"slot":0,"creatureId":4294967296,"count":1}}"#,
    ] {
        assert!(serde_json::from_str::<HeroEditRequestDto>(json).is_err());
    }
}

#[test]
fn backpack_pending_and_sample_roundtrip() {
    use h3forge_save::{BackpackEdit, BackpackItem};
    let Some(copy) = sample_copy("backpack") else {
        eprintln!("no private sample copy; skipping backpack roundtrip");
        return;
    };
    let original = std::fs::read(&copy).unwrap();
    let doc = SaveDocument::open(original.clone()).unwrap();
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let id = candidate_id(&state);
    let slots: Vec<u8> = doc.heroes[id]
        .backpack
        .iter()
        .enumerate()
        .filter(|(_, item)| **item == BackpackItem::Empty)
        .take(2)
        .map(|(i, _)| i as u8)
        .collect();
    assert_eq!(slots.len(), 2);
    let request: h3forge_desktop::dto::HeroEditRequestDto = serde_json::from_str(&format!(
        r#"{{"backpackSlot":{{"slot":{},"item":{{"kind":"artifact","value":7}}}}}}"#,
        slots[0]
    ))
    .unwrap();
    let edits = request.to_edits();
    let response = state.apply_hero_edit(id, edits.clone(), vec![]).unwrap();
    assert_eq!(
        response.backpack[slots[0] as usize].name.as_deref(),
        Some("Centaur's Axe")
    );
    assert_eq!(response.dirty_fields.len(), 1);
    state.discard_hero_edits(id).unwrap();
    assert!(state.hero_edit_state(id).unwrap().dirty_fields.is_empty());
    state.apply_hero_edit(id, edits.clone(), vec![]).unwrap();
    state.discard_all_edits().unwrap();
    assert!(state.hero_edit_state(id).unwrap().dirty_fields.is_empty());
    let mut edits = edits;
    edits.push(HeroEdit::SetBackpackSlot {
        slot: slots[1],
        item: BackpackEdit::Scroll { spell_id: 15 },
    });
    state.apply_hero_edit(id, edits.clone(), vec![]).unwrap();
    state.get_hero(id + 1).unwrap();
    assert_eq!(state.hero_edit_state(id).unwrap().dirty_fields.len(), 2);
    let dir = temp_dir("backpack");
    let dest = dir.join("backpack.GM2");
    state.save_save_as(&dest).unwrap();
    let reopened = SaveDocument::open(std::fs::read(&dest).unwrap()).unwrap();
    let plan = doc.plan_hero_edits(id, &edits).unwrap();
    assert_eq!(plan.len(), 2);
    assert_eq!(
        reopened.heroes[id].backpack[slots[1] as usize],
        BackpackItem::Scroll { spell_id: 15 }
    );
    assert_eq!(doc.heroes[id].equipment, reopened.heroes[id].equipment);
    let changed: Vec<_> = doc
        .payload()
        .iter()
        .zip(reopened.payload())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert!(!changed.is_empty());
    assert!(
        changed
            .iter()
            .all(|i| plan.iter().any(|p| (p.offset..p.end()).contains(i)))
    );
    eprintln!(
        "Backpack slots {:?}; relative patches {:?}; changed relative bytes {:?}",
        slots,
        plan.iter()
            .map(|p| (p.offset - doc.heroes[id].span.start, p.expected.len()))
            .collect::<Vec<_>>(),
        changed
            .iter()
            .map(|i| i - doc.heroes[id].span.start)
            .collect::<Vec<_>>()
    );
    assert_eq!(std::fs::read(&copy).unwrap(), original);
    assert!(state.hero_edit_state(id).unwrap().dirty_fields.is_empty());
}

#[test]
fn artifact_visual_editor_pending_transfers_and_replacements() {
    use h3forge_save::{BackpackItem, EquipmentItem, HeroConfidence};
    let Some(copy) = sample_copy("artifact-visual") else {
        eprintln!("no sample; skipping artifact visual flow");
        return;
    };
    let original = std::fs::read(&copy).unwrap();
    let doc = SaveDocument::open(original.clone()).unwrap();
    let hero = doc
        .heroes
        .iter()
        .find(|hero| {
            hero.confidence == HeroConfidence::High
                && hero.equipment_gating.is_clear()
                && hero.equipment_items[3] == EquipmentItem::Empty
                && hero
                    .backpack
                    .iter()
                    .filter(|item| **item == BackpackItem::Empty)
                    .count()
                    >= 2
        })
        .unwrap();
    let id = hero.id;
    let empty: Vec<usize> = hero
        .backpack
        .iter()
        .enumerate()
        .filter(|(_, item)| **item == BackpackItem::Empty)
        .take(2)
        .map(|(i, _)| i)
        .collect();
    let (a, b) = (empty[0], empty[1]);
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let apply = |state: &mut AppState, request: serde_json::Value| {
        let dto: h3forge_desktop::dto::HeroEditRequestDto =
            serde_json::from_value(request).unwrap();
        state.apply_hero_edit(id, dto.to_edits(), vec![]).unwrap()
    };
    use serde_json::json;
    apply(
        &mut state,
        json!({"backpackSlot":{"slot":a,"item":{"kind":"artifact","value":7}}}),
    );
    let response = apply(
        &mut state,
        json!({"backpackSlot":{"slot":a,"item":{"kind":"artifact","value":8}}}),
    );
    assert_eq!(response.backpack[a].artifact_id, Some(8));
    // Two positions are committed to pending state atomically.
    let response = apply(
        &mut state,
        json!({"backpackSlot":{"slot":a,"item":{"kind":"empty"}},"backpackDestination":{"slot":b,"item":{"kind":"artifact","value":8}}}),
    );
    assert_eq!(response.backpack[a].kind, "empty");
    assert_eq!(response.backpack[b].artifact_id, Some(8));
    apply(
        &mut state,
        json!({"equipmentSlot":{"slot":3,"item":{"kind":"artifact","value":8}},"backpackSlot":{"slot":b,"item":{"kind":"empty"}}}),
    );
    apply(
        &mut state,
        json!({"backpackSlot":{"slot":a,"item":{"kind":"artifact","value":7}}}),
    );
    let response = apply(
        &mut state,
        json!({"equipmentSlot":{"slot":3,"item":{"kind":"artifact","value":7}},"backpackSlot":{"slot":a,"item":{"kind":"artifact","value":8}}}),
    );
    assert_eq!(response.equipment[3].item.artifact_id, Some(7));
    assert_eq!(response.backpack[a].artifact_id, Some(8));
    assert_eq!(response.primary.attack, hero.primary.attack.wrapping_add(2));
    assert_eq!(
        std::fs::read(&copy).unwrap(),
        original,
        "pending edits never write source"
    );
    let dest = temp_dir("artifact-visual").join("visual.GM2");
    state.save_save_as(&dest).unwrap();
    let reopened = SaveDocument::open(std::fs::read(&dest).unwrap()).unwrap();
    assert_eq!(
        reopened.heroes[id].equipment_items[3],
        EquipmentItem::Artifact(7)
    );
    assert_eq!(reopened.heroes[id].backpack[a], BackpackItem::Artifact(8));
    assert_eq!(reopened.heroes[id].backpack[b], BackpackItem::Empty);
    assert_eq!(
        reopened.heroes[id].equipment_gating.reservation_bytes,
        hero.equipment_gating.reservation_bytes
    );
    apply(
        &mut state,
        json!({"backpackSlot":{"slot":a,"item":{"kind":"scroll","value":15}}}),
    );
    state.discard_hero_edits(id).unwrap();
    assert_eq!(
        state.hero_edit_state(id).unwrap().backpack[a].artifact_id,
        Some(8)
    );
    apply(
        &mut state,
        json!({"equipmentSlot":{"slot":3,"item":{"kind":"empty"}},"backpackSlot":{"slot":b,"item":{"kind":"artifact","value":7}}}),
    );
    state.discard_all_edits().unwrap();
    assert_eq!(
        state.hero_edit_state(id).unwrap().equipment[3]
            .item
            .artifact_id,
        Some(7)
    );
    assert_eq!(std::fs::read(&copy).unwrap(), original);
}

/// In-place equipment changes, HotA artifacts and combination artifacts
/// through the session: a worn combination shows which slots it takes,
/// reserved slots refuse items with a reason, the combination can be taken
/// off and another equipped across separate pending edits, and Save As
/// reopens with consistent reservation counters.
#[test]
fn combination_and_in_place_equipment_flow() {
    use h3forge_save::{EquipmentItem, HeroConfidence};
    let Some(copy) = sample_copy("combination") else {
        eprintln!("no sample; skipping combination equipment flow");
        return;
    };
    let original = std::fs::read(&copy).unwrap();
    let doc = SaveDocument::open(original.clone()).unwrap();
    // Sample hero wearing Power of the Dragon Father (134) in the armor slot.
    let hero = doc
        .heroes
        .iter()
        .find(|hero| {
            hero.confidence == HeroConfidence::High
                && hero.equipment_items[5] == EquipmentItem::Artifact(134)
        })
        .expect("sample hero wearing Power of the Dragon Father");
    let id = hero.id;
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    use serde_json::json;
    let request = |value: serde_json::Value| -> Vec<HeroEdit> {
        serde_json::from_value::<h3forge_desktop::dto::HeroEditRequestDto>(value)
            .unwrap()
            .to_edits()
    };
    let view = state.hero_edit_state(id).unwrap();
    assert!(
        view.equipment_read_only_reason.is_none(),
        "editable with a worn combination"
    );
    for slot in [0usize, 1, 2, 3, 4, 6, 7, 8] {
        assert_eq!(
            view.equipment[slot].reserved_by,
            Some("Power of the Dragon Father"),
            "slot {slot}"
        );
        assert!(!view.equipment[slot].editable);
    }
    let choice = view.artifact_choices.iter().find(|c| c.id == 134).unwrap();
    assert!(choice.combination);
    assert!(
        choice
            .effect
            .as_deref()
            .unwrap()
            .contains("Dragon Scale Armor")
    );
    // HotA artifacts are ordinary choices now (no "unsupported" state).
    assert!(
        view.artifact_choices
            .iter()
            .any(|c| c.id == 153 && c.equip_kind == Some("side"))
    );

    // A reserved slot refuses an item with an actionable message.
    let refused = state
        .apply_hero_edit(
            id,
            request(json!({"equipmentSlot":{"slot":3,"item":{"kind":"artifact","value":7}}})),
            vec![],
        )
        .unwrap_err();
    assert!(
        refused.message.contains("Power of the Dragon Father"),
        "{}",
        refused.message
    );

    // Replace the combination in place with ordinary armor, then use the
    // freed weapon slot: two separate pending edits, planned together.
    let view = state
        .apply_hero_edit(
            id,
            request(json!({"equipmentSlot":{"slot":5,"item":{"kind":"artifact","value":25}}})),
            vec![],
        )
        .unwrap();
    assert!(view.equipment.iter().all(|slot| slot.reserved_by.is_none()));
    let view = state
        .apply_hero_edit(
            id,
            request(json!({"equipmentSlot":{"slot":3,"item":{"kind":"artifact","value":147}}})),
            vec![],
        )
        .unwrap();
    assert_eq!(view.equipment[3].item.artifact_id, Some(147));
    // -16 (set) and +7 (Trident of Dominion) Attack; -16 +1 Spell Power.
    assert_eq!(view.primary.attack, hero.primary.attack.wrapping_sub(9));
    assert_eq!(
        view.primary.spell_power,
        hero.primary.spell_power.wrapping_sub(15)
    );
    // Armor of the Damned needs the weapon slot the trident now holds.
    assert!(
        state
            .apply_hero_edit(
                id,
                request(json!({"equipmentSlot":{"slot":5,"item":{"kind":"artifact","value":132}}})),
                vec![],
            )
            .is_err()
    );

    let dest = temp_dir("combination").join("combination.GM2");
    state.save_save_as(&dest).unwrap();
    let saved = SaveDocument::open(std::fs::read(&dest).unwrap()).unwrap();
    let after = &saved.heroes[id];
    assert_eq!(after.equipment_items[5], EquipmentItem::Artifact(25));
    assert_eq!(after.equipment_items[3], EquipmentItem::Artifact(147));
    assert_eq!(after.equipment_gating.reservation_bytes, [0; 10]);
    assert!(after.equipment_gating.is_clear());
    assert_eq!(std::fs::read(&copy).unwrap(), original, "source untouched");
}

/// A combination artifact chosen through one of its extra slots: the set is
/// equipped in its primary slot and the clicked (occupied) slot is cleared
/// in the same request. Components are labeled with their combination.
#[test]
fn combination_equipped_through_secondary_slot() {
    use h3forge_save::{EquipmentItem, HeroConfidence};
    let Some(copy) = sample_copy("combination-secondary") else {
        eprintln!("no sample; skipping secondary-slot combination flow");
        return;
    };
    let doc = SaveDocument::open(std::fs::read(&copy).unwrap()).unwrap();
    // Necklace worn, cloak and feet free.
    let hero = doc
        .heroes
        .iter()
        .find(|hero| {
            hero.confidence == HeroConfidence::High
                && hero.equipment_gating.is_clear()
                && matches!(hero.equipment_items[2], EquipmentItem::Artifact(_))
                && hero.equipment_items[1] == EquipmentItem::Empty
                && hero.equipment_items[8] == EquipmentItem::Empty
        })
        .expect("sample hero with a necklace and free cloak/feet");
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let view = state.hero_edit_state(hero.id).unwrap();
    let cloak = view.artifact_choices.iter().find(|c| c.id == 130).unwrap();
    assert_eq!(cloak.also_takes, ["neck", "feet"]);
    let cowl = view.artifact_choices.iter().find(|c| c.id == 55).unwrap();
    assert_eq!(cowl.part_of, Some("Cloak of the Undead King"));
    assert!(
        cowl.effect
            .as_deref()
            .unwrap()
            .contains("Part of Cloak of the Undead King")
    );
    let request: h3forge_desktop::dto::HeroEditRequestDto =
        serde_json::from_value(serde_json::json!({
            "equipmentSlot": {"slot": 1, "item": {"kind": "artifact", "value": 130}},
            "equipmentSecond": {"slot": 2, "item": {"kind": "empty"}},
        }))
        .unwrap();
    let view = state
        .apply_hero_edit(hero.id, request.to_edits(), vec![])
        .unwrap();
    assert_eq!(view.equipment[1].item.artifact_id, Some(130));
    for slot in [2usize, 8] {
        assert_eq!(view.equipment[slot].item.kind, "empty");
        assert_eq!(
            view.equipment[slot].reserved_by,
            Some("Cloak of the Undead King")
        );
    }
}
