//! Town editing flow through the plain session state (no Tauri runtime):
//! open a private sample copy, apply a proven garrison edit, Save As into a
//! temp directory, verify the written payload diff, the recovery backup, the
//! untouched original, and the garrison-hero blocking rule. Opt-in via
//! `H3FORGE_FIXTURE_ROOT`; skips cleanly when the sample is absent.

use std::path::{Path, PathBuf};

use h3forge_desktop::session::AppState;
use h3forge_save::SaveDocument;
use h3forge_save::definitions::towns;

fn sample_copy(tag: &str) -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from)?;
    let source = root.join("SaAndVla.GM2");
    if !source.is_file() {
        return None;
    }
    let destination = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.local/fixtures");
    std::fs::create_dir_all(&destination).ok()?;
    // Per-test destination: the tests run in parallel threads and must not
    // share one copy.
    let destination = destination.join(format!("SaAndVla-town-flow-{tag}.GM2"));
    let _ = std::fs::remove_file(&destination);
    std::fs::copy(&source, &destination).ok()?;
    Some(destination)
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("h3forge-town-flow-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn repo(relative: &str) -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(relative)
        .canonicalize()
        .ok()
}

fn open_sample(state: &mut AppState, copy: &Path) {
    // Map discovery needs the repository installation (its Maps folder);
    // without the private Data/ directory the link stays absent and the
    // map-info assertions below are skipped by the caller's guard.
    if let Some(data) = repo("Data") {
        let _ = state.set_manual_installation(data);
    }
    let grant = state.issue_grant(copy.to_path_buf());
    state
        .open_granted_save(&grant.grant_id)
        .expect("sample opens");
}

fn town_id_by_name(state: &AppState, name: &str) -> usize {
    let towns = state.list_towns().expect("towns listed");
    towns
        .iter()
        .find(|town| town.name.lossy == name)
        .map(|town| town.id)
        .unwrap_or_else(|| panic!("town {name} present"))
}

#[test]
fn garrison_edit_save_as_roundtrip() {
    let Some(copy) = sample_copy("ok") else {
        eprintln!("no private sample copy; skipping town flow test");
        return;
    };
    let dir = temp_dir("ok");
    let original_bytes = std::fs::read(&copy).unwrap();

    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let base_document = state.session_ref().unwrap().document.clone();

    // Port Balloon: proven unoccupied garrison (Gnoll x192 in slot 1).
    let id = town_id_by_name(&state, "Port Balloon");
    let detail = state.get_town(id).expect("town detail");
    assert_eq!(detail.garrison[0].creature_id, 98);
    assert_eq!(detail.garrison[0].count, 192);
    assert!(detail.garrisoned_hero.is_none());
    assert!(detail.map.is_some(), "map linked automatically");
    assert!(
        detail.map.unwrap().matched,
        "matched to the map by coordinates"
    );

    let response = state
        .apply_town_edit(
            id,
            vec![h3forge_save::TownEdit::SetGarrisonSlot {
                slot: 0,
                creature_id: Some(13), // Archangel
                count: 42,
            }],
        )
        .expect("garrison edit applies");
    assert!(response.editable);
    assert_eq!(response.dirty_fields, vec!["Garrison slot 1"]);
    assert_eq!(response.garrison[0].creature_id, 13);
    assert_eq!(response.garrison[0].count, 42);
    // Unedited slot stays as stored (Lizard Warrior x55).
    assert_eq!(response.garrison[1].creature_id, 101);
    assert_eq!(response.garrison[1].count, 55);

    let pending = state.pending_edits().expect("pending listed");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].record_kind, "town");
    assert!(pending[0].field.contains("Garrison slot 1"));

    // Save As: verified write, backup, untouched original.
    let dest = dir.join("SaAndVla towns (edited).GM2");
    let summary = state.save_save_as(&dest).expect("save succeeds");
    assert_eq!(summary.file_name, "SaAndVla towns (edited).GM2");
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

    // Reopen: the edit is present, the payload diff stays inside the two
    // garrison words, and the pending set is clean.
    let reopened = SaveDocument::open(written).expect("written save reopens");
    let edited = reopened
        .towns
        .iter()
        .find(|town| town.name_lossy == "Port Balloon")
        .expect("town still located");
    assert_eq!(edited.garrison[0].creature_id, 13);
    assert_eq!(edited.garrison[0].count, 42);
    assert_eq!(reopened.towns.len(), 37);
    let mut changed = Vec::new();
    for (index, (before, after)) in base_document
        .payload()
        .iter()
        .zip(reopened.payload())
        .enumerate()
    {
        if before != after {
            changed.push(index);
        }
    }
    let word1 = edited.span.start + towns::GARRISON_IDS;
    let word2 = edited.span.start + towns::GARRISON_COUNTS;
    assert!(
        changed.iter().all(
            |offset| (word1..word1 + 4).contains(offset) || (word2..word2 + 4).contains(offset)
        ),
        "diff confined to the garrison words: {changed:?}"
    );
    assert!(state.pending_edits().unwrap().is_empty());
}

#[test]
fn garrison_hero_ownership_blocks_town_edit() {
    let Some(copy) = sample_copy("blocked") else {
        eprintln!("no private sample copy; skipping town flow test");
        return;
    };
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    // Brill carries a garrisoned hero: the town garrison redirects to it.
    let id = town_id_by_name(&state, "Brill");
    let detail = state.get_town(id).expect("town detail");
    let hero = detail.garrisoned_hero.expect("garrisoned hero linked");
    let error = state
        .apply_town_edit(
            id,
            vec![h3forge_save::TownEdit::SetGarrisonSlot {
                slot: 0,
                creature_id: Some(0),
                count: 1,
            }],
        )
        .expect_err("garrison owned by a hero is rejected");
    assert_eq!(error.code, "edit-failed");
    assert!(
        error.message.contains("garrisoned hero"),
        "{}",
        error.message
    );
    // The effective state still names the hero for the redirect button.
    let edit_state = state.get_town_edit_state(id).expect("edit state");
    assert!(
        edit_state
            .garrison_read_only_reason
            .unwrap()
            .contains("hero")
    );
    assert_eq!(edit_state.garrisoned_hero.unwrap().id, hero.id);
    assert!(state.pending_edits().unwrap().is_empty());

    // Discard of a town with no edits stays a no-op success.
    let remaining = state.discard_town_edits(id).expect("discard");
    assert!(remaining.is_empty());
}

#[test]
fn town_list_reflects_pending_garrison_override() {
    let Some(copy) = sample_copy("list") else {
        eprintln!("no private sample copy; skipping town flow test");
        return;
    };
    let mut state = AppState::new();
    open_sample(&mut state, &copy);
    let id = town_id_by_name(&state, "Tabor Island");
    let before = state
        .list_towns()
        .unwrap()
        .into_iter()
        .find(|town| town.id == id)
        .unwrap()
        .garrison_creature_ids;
    assert!(!before.is_empty());
    // Clear every occupied slot.
    let detail = state.get_town(id).unwrap();
    let mut edits = Vec::new();
    for (slot, stack) in detail.garrison.iter().enumerate() {
        if !stack.empty {
            edits.push(h3forge_save::TownEdit::SetGarrisonSlot {
                slot: slot as u8,
                creature_id: None,
                count: 0,
            });
        }
    }
    state.apply_town_edit(id, edits).expect("clears apply");
    let after = state
        .list_towns()
        .unwrap()
        .into_iter()
        .find(|town| town.id == id)
        .unwrap()
        .garrison_creature_ids;
    assert!(after.is_empty(), "pending clears show in the list preview");
    // Revert via discard restores the preview.
    state.discard_town_edits(id).expect("discard");
    let restored = state
        .list_towns()
        .unwrap()
        .into_iter()
        .find(|town| town.id == id)
        .unwrap()
        .garrison_creature_ids;
    assert_eq!(restored, before);
}
