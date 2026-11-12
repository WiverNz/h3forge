//! Read-only coordinates survive existing pending-edit and persistence flows.
use h3forge_desktop::session::AppState;
use h3forge_save::{HeroEdit, SaveDocument};

#[test]
fn coordinates_preserved_through_pending_revert_discard_and_save() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping private coordinate flow: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let source = std::path::PathBuf::from(root).join("SaAndVla_before_movement.GM2");
    let original = std::fs::read(&source).unwrap();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../.local/fixtures/coordinate-flow-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let copy = dir.join("source.GM2");
    std::fs::write(&copy, &original).unwrap();
    let base = SaveDocument::open(original.clone()).unwrap();
    let mut state = AppState::new();
    let grant = state.issue_grant(copy.clone());
    state.open_granted_save(&grant.grant_id).unwrap();
    assert_eq!(state.get_hero(66).unwrap().coordinates, Some([231, 52, 0]));
    for discard_all in [false, true] {
        state
            .apply_hero_edit(66, vec![HeroEdit::SetAttack(9)], Vec::new())
            .unwrap();
        assert_eq!(state.get_hero(66).unwrap().coordinates, Some([231, 52, 0]));
        if discard_all {
            state.discard_all_edits().unwrap();
        } else {
            state.discard_hero_edits(66).unwrap();
        }
        assert!(state.pending_edits().unwrap().is_empty());
    }
    state
        .apply_hero_edit(66, vec![HeroEdit::SetAttack(9)], Vec::new())
        .unwrap();
    let dest = dir.join("edited.GM2");
    state.save_save_as(&dest).unwrap();
    assert_eq!(std::fs::read(&copy).unwrap(), original);
    state
        .apply_hero_edit(66, vec![HeroEdit::SetAttack(10)], Vec::new())
        .unwrap();
    state.save().unwrap();
    let reopened = SaveDocument::open(std::fs::read(dest).unwrap()).unwrap();
    assert_eq!(reopened.hero_coordinates(66), Some([231, 52, 0]));
    assert_eq!(reopened.hero_coordinates(75), Some([76, 41, 0]));
    let changed: Vec<_> = base
        .payload()
        .iter()
        .zip(reopened.payload())
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert_eq!(changed, [base.heroes[66].span.start + 238]);
    assert_eq!(reopened.heroes[66].primary.attack, 10);
    assert!(state.pending_edits().unwrap().is_empty());
    assert_eq!(std::fs::read(source).unwrap(), original);

    // Coordinates use the same pending/edit/save lifecycle as attributes.
    state
        .apply_hero_edit(66, vec![HeroEdit::SetCoordinates([108, 77, 0])], Vec::new())
        .unwrap();
    assert_eq!(
        state.hero_edit_state(66).unwrap().coordinates,
        Some([108, 77, 0])
    );
    assert!(state.set_hero_dismissal(66, true).is_err());
    state.discard_hero_edits(66).unwrap();
    assert_eq!(
        state.hero_edit_state(66).unwrap().coordinates,
        Some([231, 52, 0])
    );
    state
        .apply_hero_edit(75, vec![HeroEdit::SetCoordinates([118, 71, 0])], Vec::new())
        .unwrap();
    state.discard_all_edits().unwrap();
    assert_eq!(
        state.hero_edit_state(75).unwrap().coordinates,
        Some([76, 41, 0])
    );
    state.set_hero_dismissal(75, true).unwrap();
    assert!(
        state
            .apply_hero_edit(75, vec![HeroEdit::SetCoordinates([118, 71, 0])], Vec::new())
            .is_err()
    );
    state.discard_all_edits().unwrap();
    state
        .apply_hero_edit(
            66,
            vec![
                HeroEdit::SetCoordinates([108, 77, 0]),
                HeroEdit::SetAttack(11),
            ],
            Vec::new(),
        )
        .unwrap();
    state
        .apply_hero_edit(75, vec![HeroEdit::SetCoordinates([118, 71, 0])], Vec::new())
        .unwrap();
    assert!(
        state
            .apply_hero_edit(66, vec![HeroEdit::SetCoordinates([252, 52, 0])], Vec::new())
            .is_err()
    );
    assert_eq!(
        state.hero_edit_state(66).unwrap().coordinates,
        Some([108, 77, 0])
    );
    let moved = dir.join("relocated.GM2");
    // The base tile is empty and legal for another boarded hero, but is now
    // reserved by Nimbus's pending relocation.
    base.plan_hero_relocation(3, [118, 71, 0]).unwrap();
    let collision = state
        .apply_hero_edit(3, vec![HeroEdit::SetCoordinates([118, 71, 0])], Vec::new())
        .unwrap_err();
    assert!(
        collision
            .message
            .contains("Another pending hero relocation")
    );
    state.save_save_as(&moved).unwrap();
    let after = SaveDocument::open(std::fs::read(&moved).unwrap()).unwrap();
    assert_eq!(after.hero_coordinates(66), Some([108, 77, 0]));
    assert_eq!(after.hero_coordinates(75), Some([118, 71, 0]));
    assert_eq!(after.heroes[66].primary.attack, 11);
    assert_eq!(
        after.heroes[66].movement_left,
        base.heroes[66].movement_left
    );
    assert!(state.pending_edits().unwrap().is_empty());
    state
        .apply_hero_edit(66, vec![HeroEdit::SetCoordinates([231, 52, 0])], Vec::new())
        .unwrap();
    state.save().unwrap();
    let after = SaveDocument::open(std::fs::read(moved).unwrap()).unwrap();
    assert_eq!(after.hero_coordinates(66), Some([231, 52, 0]));
    assert_eq!(std::fs::read(&copy).unwrap(), original);
}
