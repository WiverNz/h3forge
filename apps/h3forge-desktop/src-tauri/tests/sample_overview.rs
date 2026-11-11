//! Opt-in test opening the private sample save through the app session
//! layer (grant → open → list → detail). Set `H3FORGE_FIXTURE_ROOT` to a
//! directory containing `SaAndVla.GM2`; without it the test passes without
//! touching anything. The sample bytes are only read; the test works on a
//! copy under the ignored `.local/fixtures/` directory, never in `games/`
//! or `Data/`.

use std::path::{Path, PathBuf};

use h3forge_desktop::session::AppState;

const EXPECTED_HEROES: usize = 215;
const RESEARCH_CANDIDATE_OFFSET: usize = 4_727_078;

fn repo_root() -> PathBuf {
    // apps/h3forge-desktop/src-tauri -> repository root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn copy_destination() -> PathBuf {
    repo_root().join(".local/fixtures/SaAndVla.GM2")
}

#[test]
fn sample_overview_reproduces_research_baseline() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("H3FORGE_FIXTURE_ROOT not set; skipping private sample test");
        return;
    };
    let source = PathBuf::from(&root).join("SaAndVla.GM2");
    if !source.is_file() {
        eprintln!("no SaAndVla.GM2 under {root:?}; skipping private sample test");
        return;
    }

    // Work only on a copy in the ignored fixture directory; the fixed
    // destination can never resolve into games/ or Data/.
    let destination = copy_destination();
    std::fs::create_dir_all(destination.parent().expect("fixture dir"))
        .expect("fixture dir creates");
    std::fs::copy(&source, &destination).expect("sample copy writes");
    let canonical = destination.canonicalize().expect("copy canonicalizes");
    let repo = repo_root();
    for forbidden in ["games", "Data"] {
        assert!(
            !canonical.starts_with(repo.join(forbidden)),
            "test destination must stay out of repository {forbidden}/"
        );
    }

    let mut state = AppState::new();
    let grant = state.issue_grant(canonical);
    assert_eq!(grant.file_name, "SaAndVla.GM2");
    let summary = state
        .open_granted_save(&grant.grant_id)
        .expect("sample opens through the grant");

    assert_eq!(summary.family_title, Some("Horn of the Abyss"));
    assert_eq!((summary.major, summary.minor), (44, 11));
    assert_eq!(summary.compatibility, "readable");
    assert!(summary.heroes_supported);
    assert_eq!(
        summary.map_name.expect("map name").raw,
        b"- The Mysterious Island - COOP"
    );
    assert_eq!(summary.hero_count, EXPECTED_HEROES);
    assert_eq!(summary.confirmed_count, EXPECTED_HEROES);

    let heroes = state.list_heroes().expect("list");
    assert_eq!(heroes.len(), EXPECTED_HEROES);

    // Selection by opaque ID reaches the research candidate at its span.
    let mut candidate_id = None;
    for id in 0..heroes.len() {
        let detail = state.get_hero(id).expect("detail");
        if detail.span_start == RESEARCH_CANDIDATE_OFFSET {
            candidate_id = Some(id);
            assert_eq!(detail.span_end, RESEARCH_CANDIDATE_OFFSET + 1122);
            assert_eq!(detail.owner.kind, "player");
            assert_eq!(detail.owner.player, Some(4));
            assert_eq!(detail.owner.label, "Orange");
            assert_eq!(detail.confidence, "high");
            assert_eq!(detail.level, 2);
            assert_eq!(detail.level_from_experience, 2);
            assert_eq!(detail.experience, 1064);
            assert_eq!(detail.spell_points, 73);
            assert_eq!(detail.movement_total, detail.movement_left);
            assert_eq!(detail.primary.attack, 3);
            assert_eq!(detail.primary.defense, 7);
            assert_eq!(detail.primary.spell_power, 6);
            assert_eq!(detail.primary.knowledge, 6);
            assert!(detail.has_spellbook);
            let learned: Vec<usize> = detail.spells_learned.iter().map(|s| s.id).collect();
            assert_eq!(
                learned,
                vec![3, 15, 17, 20, 25, 33, 35, 37, 42, 43, 44, 47, 64]
            );
            let skills: Vec<_> = detail.skills.iter().map(|skill| skill.name).collect();
            assert_eq!(
                skills,
                vec![Some("Wisdom"), Some("Intelligence"), Some("Mysticism")]
            );
        }
    }
    assert!(
        candidate_id.is_some(),
        "research candidate record present at offset {RESEARCH_CANDIDATE_OFFSET}"
    );

    // CP1251 evidence: many names carry high bytes that are not valid UTF-8;
    // raw bytes must survive untouched for the frontend to re-decode.
    let non_utf8 = heroes
        .iter()
        .filter(|hero| std::str::from_utf8(&hero.name.raw).is_err())
        .count();
    assert!(
        non_utf8 >= 100,
        "expected a large Cyrillic-named population, got {non_utf8}"
    );
}
