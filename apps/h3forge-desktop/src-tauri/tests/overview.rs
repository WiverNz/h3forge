//! App-level integration tests for the read-only hero overview: opening via
//! grant, duplicate/malformed record handling, selection by opaque ID, and
//! the absence of any write path. All saves are synthetic and live only in
//! the OS temporary directory.

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use common::{craft_record, gzip_stored, save_payload};
use h3forge_desktop::session::AppState;

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

const AT_A: usize = 30_000;
// Dense 38-byte inter-record gaps like the real heroes section.
const AT_B: usize = AT_A + 1122 + 38;
const AT_C: usize = AT_B + 1122 + 38;

fn temp_save(name: &str, bytes: &[u8]) -> PathBuf {
    // Unique directory per call: integration tests run in parallel threads.
    let dir = std::env::temp_dir().join(format!(
        "h3forge-overview-{}-{}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).expect("temp dir creates");
    let path = dir.join(name);
    fs::write(&path, bytes).expect("fixture writes");
    path
}

fn open_fixture() -> (AppState, h3forge_desktop::dto::SaveSummaryDto) {
    // A: owned Orange hero with spellbook and learned spells.
    let a = craft_record(4, b"Aiden", 2, 1064, 73, false);
    // B: neutral record with the SAME name and a level/experience mismatch.
    let b = craft_record(0xFF, b"Aiden", 5, 0, 0, true);
    // C: structurally valid but owner 9 fails secondary verification.
    let c = craft_record(9, b"Caitlin", 1, 0, 0, true);
    let payload = save_payload(b"Test Map", b"A map", &[(AT_A, a), (AT_B, b), (AT_C, c)]);
    let path = temp_save("overview.GM2", &gzip_stored(&payload));

    let mut state = AppState::new();
    let grant = state.issue_grant(path);
    let summary = state
        .open_granted_save(&grant.grant_id)
        .expect("fixture opens");
    (state, summary)
}

#[test]
fn summary_reports_format_map_and_counts() {
    let (_state, summary) = open_fixture();
    assert_eq!(summary.file_name, "overview.GM2");
    assert_eq!(summary.family_title, Some("Horn of the Abyss"));
    assert_eq!((summary.major, summary.minor), (44, 11));
    assert_eq!(summary.container_kind, "regular");
    assert_eq!(summary.compatibility, "readable");
    assert!(summary.heroes_supported);
    let map = summary.map_name.expect("map name present");
    assert_eq!(map.raw, b"Test Map");
    assert_eq!(summary.map_description.expect("description").raw, b"A map");
    assert_eq!(summary.hero_count, 3);
    assert_eq!(summary.confirmed_count, 2, "record C stays a candidate");
    assert!(summary.diagnostics.is_empty());
}

#[test]
fn duplicate_names_select_distinct_spans_by_id() {
    let (state, _summary) = open_fixture();
    let heroes = state.list_heroes().expect("list");
    assert_eq!(heroes.len(), 3);

    // Same decoded name, different records.
    assert_eq!(heroes[0].name.lossy, "Aiden");
    assert_eq!(heroes[1].name.lossy, "Aiden");
    assert!(heroes[0].duplicate_name);
    assert!(heroes[1].duplicate_name);
    assert!(!heroes[2].duplicate_name);

    // IDs select the correct span even with identical names.
    let first = state.get_hero(0).expect("hero 0");
    let second = state.get_hero(1).expect("hero 1");
    assert_eq!((first.span_start, first.span_end), (AT_A, AT_A + 1122));
    assert_eq!((second.span_start, second.span_end), (AT_B, AT_B + 1122));

    // Owned vs neutral is distinguishable in the list.
    assert_eq!(heroes[0].owner.kind, "player");
    assert_eq!(heroes[0].owner.player, Some(4));
    assert_eq!(heroes[0].owner.label, "Orange");
    assert!(heroes[0].has_spellbook);
    assert_eq!(heroes[1].owner.kind, "neutral");
    assert!(!heroes[1].has_spellbook);

    // Duplicate-name and mismatch notes are explained on the detail view.
    assert!(
        first
            .notes
            .iter()
            .any(|note| note.kind == "duplicate-name" && note.message.contains("#1"))
    );
    assert!(
        second
            .notes
            .iter()
            .any(|note| note.kind == "level-experience-mismatch")
    );
}

#[test]
fn owned_spellbook_hero_shows_actual_values() {
    let (state, _summary) = open_fixture();
    let hero = state.get_hero(0).expect("hero 0");
    assert_eq!(hero.confidence, "high");
    assert_eq!(hero.level, 2);
    assert_eq!(hero.level_from_experience, 2);
    assert_eq!(hero.experience, 1064);
    assert_eq!(hero.spell_points, 73);
    assert_eq!(hero.movement_total, 1500);
    assert_eq!(hero.movement_left, 1500);
    assert_eq!((hero.primary.attack, hero.primary.defense), (0, 0));
    let learned: Vec<usize> = hero.spells_learned.iter().map(|spell| spell.id).collect();
    assert_eq!(learned, vec![3, 15]);
    assert_eq!(
        hero.spells_learned
            .iter()
            .map(|spell| spell.name)
            .collect::<Vec<_>>(),
        vec![Some("View Earth"), Some("Magic Arrow")]
    );
    let skill = &hero.skills[0];
    assert_eq!(skill.name, Some("Wisdom"));
    assert_eq!(skill.mastery_name, Some("Advanced"));
    // Unsupported fields stay raw but carry labeled slot names.
    assert_eq!(hero.equipment.len(), 19);
    assert_eq!(hero.equipment[17].slot, "spellbook");
    assert_eq!(hero.inventory.len(), 64 * 8);
}

#[test]
fn malformed_candidate_is_listed_but_flagged() {
    let (state, _summary) = open_fixture();
    let heroes = state.list_heroes().expect("list");
    assert_eq!(heroes[2].confidence, "low");
    assert!(heroes[2].has_notes);

    let detail = state.get_hero(2).expect("candidate detail");
    assert_eq!(detail.confidence, "low");
    assert_eq!(detail.span_start, AT_C);
    let rejection = detail
        .notes
        .iter()
        .find(|note| note.kind == "rejected-owner")
        .expect("owner rejection explained");
    assert!(rejection.message.contains("low-confidence"));
}

#[test]
fn errors_are_structured_and_no_session_survives_failure() {
    // Unknown grant without a picker selection.
    let mut state = AppState::new();
    let error = state.open_granted_save("grant-1").unwrap_err();
    assert_eq!(error.code, "grant-not-found");

    let (_state, summary) = open_fixture();
    assert_eq!(summary.hero_count, 3);

    // A mismatched grant ID is rejected and the real grant stays usable.
    let payload = save_payload(b"T", b"", &[]);
    let path = temp_save("second.GM2", &gzip_stored(&payload));
    let mut state = AppState::new();
    let grant = state.issue_grant(path);
    let error = state.open_granted_save("grant-999").unwrap_err();
    assert_eq!(error.code, "grant-not-found");
    let summary = state
        .open_granted_save(&grant.grant_id)
        .expect("matching grant still opens");
    assert_eq!(summary.hero_count, 0);

    // Malformed file: open fails with the core error surfaced, no session.
    let bad = temp_save("bad.GM2", b"not a gzip save at all");
    let mut state = AppState::new();
    let grant = state.issue_grant(bad);
    let error = state.open_granted_save(&grant.grant_id).unwrap_err();
    assert_eq!(error.code, "open-failed");
    assert!(error.message.contains("gzip"));
    let error = state.list_heroes().unwrap_err();
    assert_eq!(error.code, "no-open-save");
    let error = state.get_hero(0).unwrap_err();
    assert_eq!(error.code, "no-open-save");

    // Unknown hero ID on a live session.
    let (state, _summary) = open_fixture();
    let error = state.get_hero(99).unwrap_err();
    assert_eq!(error.code, "hero-not-found");
}
