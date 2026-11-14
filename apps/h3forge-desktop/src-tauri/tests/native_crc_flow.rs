//! User-supplied native team-research saves, copied before opening or editing.
use h3forge_desktop::session::AppState;
use h3forge_save::{DiagnosticCode, HeroEdit, HeroOwner, SaveDocument};

#[test]
fn native_team_fixtures_open_edit_save_and_reopen_without_changing_team_bytes() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping native CRC fixtures: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = repo.join(format!(".local/fixtures/native-crc-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in [
        "same_teams",
        "same_teams_restart",
        "separate_teams",
        "separate_team_map_2",
    ] {
        let original =
            std::fs::read(std::path::PathBuf::from(&root).join(format!("{name}.GM2"))).unwrap();
        let source = dir.join(format!("{name}.GM2"));
        std::fs::write(&source, &original).unwrap();
        let document = SaveDocument::open(std::fs::read(&source).unwrap()).unwrap();
        assert!(
            document
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::NativeGzipCrcMismatch)
        );
        eprintln!(
            "{name}: heroes={}, towns={}, players={}",
            document.heroes.len(),
            document.towns.len(),
            document.players.len()
        );
        assert_eq!(document.towns.len(), 6);
        assert_eq!(document.players.len(), 8);
        assert!(
            document
                .players
                .iter()
                .all(|p| p.read_only_reason.is_none())
        );
        let id = document
            .heroes
            .iter()
            .position(|h| {
                h.owner == HeroOwner::Player(0)
                    && document
                        .plan_hero_edits(h.id, &[HeroEdit::SetSpellPoints(99)])
                        .is_ok()
            })
            .expect("editable Red hero");
        let mut state = AppState::new();
        let grant = state.issue_grant(source.clone());
        state.open_granted_save(&grant.grant_id).unwrap();
        let patches = document
            .plan_hero_edits(id, &[HeroEdit::SetSpellPoints(99)])
            .unwrap();
        let mut expected = document.payload().to_vec();
        h3forge_save::apply_patches(&mut expected, &patches).unwrap();
        state
            .apply_hero_edit(id, vec![HeroEdit::SetSpellPoints(99)], vec![])
            .unwrap();
        state.discard_hero_edits(id).unwrap();
        assert!(state.pending_edits().unwrap().is_empty());
        state
            .apply_hero_edit(id, vec![HeroEdit::SetSpellPoints(99)], vec![])
            .unwrap();
        state.discard_all_edits().unwrap();
        assert!(state.pending_edits().unwrap().is_empty());
        state
            .apply_hero_edit(id, vec![HeroEdit::SetSpellPoints(99)], vec![])
            .unwrap();
        let dest = dir.join(format!("{name}-edited.GM2"));
        state.save_save_as(&dest).unwrap();
        let reopened = SaveDocument::open_strict(std::fs::read(&dest).unwrap()).unwrap();
        assert_eq!(reopened.payload(), expected);
        assert_eq!(reopened.heroes[id].spell_points, 99);
        assert_eq!(reopened.players, document.players);
        let changed: Vec<_> = document
            .payload()
            .iter()
            .zip(reopened.payload())
            .enumerate()
            .filter_map(|(i, (a, b))| (a != b).then_some(i))
            .collect();
        assert!(!changed.is_empty());
        assert!(
            changed
                .iter()
                .all(|i| patches.iter().any(|p| (p.offset..p.end()).contains(i)))
        );
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(dir.read_dir().unwrap().filter_map(Result::ok).any(|e| {
            e.file_name().to_string_lossy().contains(".backup-")
                && std::fs::read(e.path()).unwrap() == original
        }));
        state
            .apply_hero_edit(id, vec![HeroEdit::SetSpellPoints(100)], vec![])
            .unwrap();
        state.save().unwrap();
        assert_eq!(
            SaveDocument::open_strict(std::fs::read(&dest).unwrap())
                .unwrap()
                .heroes[id]
                .spell_points,
            100
        );
        assert!(state.pending_edits().unwrap().is_empty());
        eprintln!(
            "{name}: native open, Revert/Discard, exact mana-only patch, Save As/Save and strict reopen passed; changed offsets {changed:?}"
        );
    }
}
