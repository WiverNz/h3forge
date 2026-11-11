//! Copy-only hero dismissal (H3F-026) through the persistence path the UI
//! uses: pending state, Revert, Discard, Save As, Save and reopen.
//!
//! `H3FORGE_FIXTURE_ROOT` must be the absolute path of the `games` folder
//! holding the private `SaAndVla_before.GM2` sample.
use h3forge_desktop::session::AppState;
use h3forge_save::{HeroOwner, SaveDocument};

const GALTHRAN: usize = 71;
const OTHER_PINK_HERO: usize = 78;
const GARRISONED: usize = 69;

#[test]
fn dismissal_session_revert_discard_save_as_and_save() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping private dismissal flow: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let sample = std::path::PathBuf::from(root).join("SaAndVla_before.GM2");
    if !sample.exists() {
        eprintln!("skipping private dismissal flow: SaAndVla_before.GM2 is missing");
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../.local/fixtures/dismissal-flow-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let original = std::fs::read(sample).unwrap();
    let source = dir.join("source.GM2");
    std::fs::write(&source, &original).unwrap();
    let mut state = AppState::new();
    let grant = state.issue_grant(source.clone());
    state.open_granted_save(&grant.grant_id).unwrap();

    // Availability and explanations come from the backend.
    let base = state.hero_edit_state(GALTHRAN).unwrap();
    assert!(!base.dismissal_pending);
    assert!(base.dismissal_blocked_reason.is_none());
    let blocked = state.hero_edit_state(GARRISONED).unwrap();
    assert!(
        blocked
            .dismissal_blocked_reason
            .unwrap()
            .contains("garrisoned")
    );
    assert!(state.set_hero_dismissal(GARRISONED, true).is_err());
    assert!(state.set_hero_dismissal(0, true).is_err());
    assert!(state.pending_edits().unwrap().is_empty());
    assert!(state.save().is_err());

    // Mark, unmark (Revert), per-hero discard and Discard all.
    let marked = state.set_hero_dismissal(GALTHRAN, true).unwrap();
    assert!(marked.dismissal_pending);
    assert!(marked.dirty_fields.contains(&"Dismissal".to_string()));
    let pending = state.pending_edits().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(
        (
            pending[0].record_kind,
            pending[0].record_id,
            pending[0].field.as_str()
        ),
        ("hero", GALTHRAN, "Dismissal")
    );
    assert_eq!(
        (pending[0].before.as_str(), pending[0].after.as_str()),
        ("Pink", "Dismissed")
    );
    let reverted = state.set_hero_dismissal(GALTHRAN, false).unwrap();
    assert!(!reverted.dismissal_pending && reverted.dirty_fields.is_empty());
    assert!(state.pending_edits().unwrap().is_empty());
    state.set_hero_dismissal(GALTHRAN, true).unwrap();
    assert!(state.discard_hero_edits(GALTHRAN).unwrap().is_empty());
    state.set_hero_dismissal(GALTHRAN, true).unwrap();
    state.set_hero_dismissal(OTHER_PINK_HERO, true).unwrap();
    assert_eq!(state.pending_edits().unwrap().len(), 2);
    assert!(state.discard_all_edits().unwrap().is_empty());
    assert!(!state.hero_edit_state(GALTHRAN).unwrap().dismissal_pending);

    // Dismissal resets army and spell points, so pending edits of those
    // bytes cannot be combined with it, in either order.
    let spell_points = h3forge_save::HeroEdit::SetSpellPoints(7);
    state
        .apply_hero_edit(GALTHRAN, vec![spell_points.clone()], Vec::new())
        .unwrap();
    assert!(state.set_hero_dismissal(GALTHRAN, true).is_err());
    assert!(!state.hero_edit_state(GALTHRAN).unwrap().dismissal_pending);
    state.discard_hero_edits(GALTHRAN).unwrap();
    state.set_hero_dismissal(GALTHRAN, true).unwrap();
    assert!(
        state
            .apply_hero_edit(GALTHRAN, vec![spell_points], Vec::new())
            .is_err()
    );
    assert_eq!(state.pending_edits().unwrap().len(), 1);
    state.discard_all_edits().unwrap();

    // Save As together with an ordinary hero edit; the source stays intact.
    state.set_hero_dismissal(GALTHRAN, true).unwrap();
    state
        .apply_hero_edit(
            GALTHRAN,
            vec![h3forge_save::HeroEdit::SetAttack(9)],
            Vec::new(),
        )
        .unwrap();
    let dest = dir.join("dismissed.GM2");
    state.save_save_as(&dest).unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert!(state.pending_edits().unwrap().is_empty());
    let doc = SaveDocument::open(std::fs::read(&dest).unwrap()).unwrap();
    assert_eq!(doc.heroes[GALTHRAN].owner, HeroOwner::Neutral);
    assert_eq!(doc.heroes[GALTHRAN].primary.attack, 9);
    assert_eq!(doc.heroes[GALTHRAN].army[0].count, 1);
    assert_eq!(doc.players[7].hero_count, 2);
    assert!(doc.players[7].read_only_reason.is_none());

    // The session re-based onto the written file: the hero is now unowned.
    let after = state.hero_edit_state(GALTHRAN).unwrap();
    assert!(!after.dismissal_pending);
    assert!(after.dismissal_blocked_reason.is_some());
    assert!(state.set_hero_dismissal(GALTHRAN, true).is_err());

    // Direct Save of a second dismissal over the generated copy.
    state.set_hero_dismissal(OTHER_PINK_HERO, true).unwrap();
    state.save().unwrap();
    let doc = SaveDocument::open(std::fs::read(&dest).unwrap()).unwrap();
    assert_eq!(doc.heroes[OTHER_PINK_HERO].owner, HeroOwner::Neutral);
    assert_eq!(doc.players[7].hero_count, 1);
    assert!(state.pending_edits().unwrap().is_empty());
}
