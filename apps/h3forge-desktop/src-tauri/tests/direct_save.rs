//! Opt-in real-save regression. Only unique ignored copies are writable.
use h3forge_desktop::session::AppState;
use h3forge_save::{HeroEdit, SaveDocument};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn fixture(tag: &str) -> Option<(PathBuf, PathBuf)> {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("H3FORGE_FIXTURE_ROOT absent; skipping {tag}");
        return None;
    };
    let source = PathBuf::from(root).join("SaAndVla.GM2");
    assert!(source.is_file(), "configured SaAndVla fixture must exist");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../../.local/fixtures/direct-{tag}-{nonce}"));
    std::fs::create_dir_all(&dir).unwrap();
    let copy = dir.join("working.GM2");
    std::fs::copy(&source, &copy).unwrap();
    Some((source, copy))
}
fn open(state: &mut AppState, path: &std::path::Path) {
    let grant = state.issue_grant(path.to_owned());
    state.open_granted_save(&grant.grant_id).unwrap();
}
fn hero(doc: &SaveDocument) -> usize {
    doc.heroes
        .iter()
        .position(|h| h.span.start == 4_727_078)
        .expect("verified hero span")
}
fn backups(path: &std::path::Path) -> Vec<Vec<u8>> {
    std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(Result::unwrap)
        .filter(|e| e.file_name().to_string_lossy().contains(".backup-"))
        .map(|e| std::fs::read(e.path()).unwrap())
        .collect()
}
fn assert_payload(before: &SaveDocument, after: &SaveDocument, id: usize, edits: &[HeroEdit]) {
    let plan = before.plan_hero_edits(id, edits).unwrap();
    let mut expected = before.payload().to_vec();
    h3forge_save::apply_patches(&mut expected, &plan).unwrap();
    assert_eq!(
        expected,
        after.payload(),
        "full payload equals explicit patches"
    );
    // Independent allowlist: primary attack, two spell flags (and available),
    // and the first two army ID/count words. No other offsets may differ.
    let base = before.heroes[id].span.start;
    for (offset, (a, b)) in before.payload().iter().zip(after.payload()).enumerate() {
        if a != b {
            let local = offset.checked_sub(base).unwrap();
            assert!(
                (113..121).contains(&local)
                    || (141..149).contains(&local)
                    || [238, 242, 306, 312, 376].contains(&local),
                "unexpected offset {offset}"
            );
        }
    }
}

#[test]
fn real_save_twice_and_save_as_rebases_with_verified_backups() {
    let Some((source, copy)) = fixture("roundtrip") else {
        return;
    };
    let original = std::fs::read(&source).unwrap();
    let hash = Sha256::digest(&original);
    let before = SaveDocument::open(std::fs::read(&copy).unwrap()).unwrap();
    assert_eq!(before.heroes.len(), 215);
    let noop = SaveDocument::open(before.serialize_with_patches(&[]).unwrap()).unwrap();
    assert_eq!(before.payload(), noop.payload());
    let id = hero(&before);
    let mut state = AppState::new();
    open(&mut state, &copy);
    assert!(state.summary().unwrap().save_disabled_reason.is_none());
    assert_eq!(state.save().unwrap_err().code, "nothing-to-save");
    let edits = vec![
        HeroEdit::SetAttack(9),
        HeroEdit::SetSpellLearned {
            spell_id: 0,
            learned: true,
        },
        HeroEdit::SetSpellLearned {
            spell_id: 64,
            learned: false,
        },
        HeroEdit::SetArmySlot {
            slot: 0,
            creature_id: Some(153),
            count: 1234,
        },
        HeroEdit::SetArmySlot {
            slot: 1,
            creature_id: None,
            count: 0,
        },
    ];
    state.apply_hero_edit(id, edits.clone(), vec![]).unwrap();
    state.save().unwrap();
    let first_bytes = std::fs::read(&copy).unwrap();
    let first = SaveDocument::open(first_bytes.clone()).unwrap();
    assert_payload(&before, &first, id, &edits);
    assert_eq!(first.heroes[id].primary.attack, 9);
    assert_eq!(first.heroes[id].spells_learned[0], 1);
    assert_eq!(first.heroes[id].spells_learned[64], 0);
    assert_eq!(first.heroes[id].army[0].creature_id, 153);
    assert_eq!(first.heroes[id].army[0].count, 1234);
    assert!(first.heroes[id].army[1].is_empty());
    assert_eq!(backups(&copy), [original.clone()]);
    assert!(state.pending_edits().unwrap().is_empty());

    let second_edits = vec![
        HeroEdit::SetSpellLearned {
            spell_id: 0,
            learned: false,
        },
        HeroEdit::SetSpellLearned {
            spell_id: 64,
            learned: true,
        },
        HeroEdit::SetArmySlot {
            slot: 0,
            creature_id: Some(153),
            count: 4321,
        },
        HeroEdit::SetArmySlot {
            slot: 1,
            creature_id: Some(0),
            count: 10,
        },
    ];
    state
        .apply_hero_edit(id, second_edits.clone(), vec![])
        .unwrap();
    state.save().unwrap();
    let second_bytes = std::fs::read(&copy).unwrap();
    let second = SaveDocument::open(second_bytes.clone()).unwrap();
    assert_payload(&first, &second, id, &second_edits);
    assert_eq!(second.heroes[id].army[1].creature_id, 0);
    assert_eq!(second.heroes[id].army[1].count, 10);
    assert_eq!(second.heroes[id].army[0].count, 4321);
    assert_eq!(second.heroes[id].spells_learned[0], 0);
    assert_eq!(second.heroes[id].spells_learned[64], 1);
    let recovery = backups(&copy);
    assert_eq!(recovery.len(), 2);
    assert!(recovery.contains(&original) && recovery.contains(&first_bytes));

    // Save As must switch the source; subsequent direct Save uses only that file.
    let new_path = copy.with_file_name("new.GM2");
    state
        .apply_hero_edit(id, vec![HeroEdit::SetAttack(10)], vec![])
        .unwrap();
    state.save_save_as(&new_path).unwrap();
    let new_baseline = std::fs::read(&new_path).unwrap();
    state
        .apply_hero_edit(id, vec![HeroEdit::SetAttack(11)], vec![])
        .unwrap();
    state.save().unwrap();
    assert_eq!(std::fs::read(&copy).unwrap(), second_bytes);
    assert!(backups(&new_path).contains(&new_baseline));
    assert_eq!(
        SaveDocument::open(std::fs::read(&new_path).unwrap())
            .unwrap()
            .heroes[id]
            .primary
            .attack,
        11
    );
    assert_eq!(Sha256::digest(std::fs::read(source).unwrap()), hash);
    eprintln!("SaAndVla SHA-256 unchanged: {hash:x}; 215 heroes; repeated Save + Save As verified");
}

#[test]
fn real_save_rejects_protected_invalid_changed_and_locked_sources() {
    let Some((source, copy)) = fixture("rejections") else {
        return;
    };
    let hash = Sha256::digest(std::fs::read(&source).unwrap());
    let baseline = std::fs::read(&copy).unwrap();
    let doc = SaveDocument::open(baseline.clone()).unwrap();
    let id = hero(&doc);
    let mut state = AppState::new();
    open(&mut state, &copy);
    for edit in [
        HeroEdit::SetAttack(100),
        HeroEdit::SetArmySlot {
            slot: 0,
            creature_id: Some(999),
            count: 1,
        },
        HeroEdit::SetArmySlot {
            slot: 0,
            creature_id: Some(0),
            count: 0,
        },
    ] {
        assert!(state.apply_hero_edit(id, vec![edit], vec![]).is_err());
    }
    assert!(state.pending_edits().unwrap().is_empty());
    state
        .apply_hero_edit(id, vec![HeroEdit::SetAttack(9)], vec![])
        .unwrap();
    assert_eq!(state.save_save_as(&copy).unwrap_err().code, "same-file");
    let existing = copy.with_file_name("existing.GM2");
    std::fs::write(&existing, b"keep me").unwrap();
    assert_eq!(
        state.save_save_as(&existing).unwrap_err().code,
        "file-exists"
    );
    assert_eq!(std::fs::read(existing).unwrap(), b"keep me");

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Allow reads/writes, but deny DELETE sharing: preparation works,
        // Windows replacement fails, and the verified backup must remain.
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1 | 2)
            .open(&copy)
            .unwrap();
        assert_eq!(state.save().unwrap_err().code, "write-failed");
        assert_eq!(std::fs::read(&copy).unwrap(), baseline);
        assert_eq!(backups(&copy), [baseline.clone()]);
        assert_eq!(state.pending_edits().unwrap().len(), 1);
        drop(lock);
    }
    let modified_time = std::fs::metadata(&copy).unwrap().modified().unwrap();
    let mut changed = baseline.clone();
    changed[20] ^= 1;
    std::fs::write(&copy, &changed).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&copy)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified_time))
        .unwrap();
    assert_eq!(state.save().unwrap_err().code, "source-changed");
    assert_eq!(std::fs::read(&copy).unwrap(), changed);
    assert_eq!(state.pending_edits().unwrap().len(), 1);

    // Open the original read-only: repository games/Data guards must reject Save.
    let repo_source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../games/SaAndVla.GM2");
    if repo_source.canonicalize().ok().as_ref() == Some(&source.canonicalize().unwrap()) {
        open(&mut state, &source);
        assert!(
            state
                .summary()
                .unwrap()
                .save_disabled_reason
                .unwrap()
                .contains("protected")
        );
        state
            .apply_hero_edit(id, vec![HeroEdit::SetAttack(9)], vec![])
            .unwrap();
        assert_eq!(state.save().unwrap_err().code, "forbidden-destination");
        let data = repo_source
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Data/forbidden.GM2");
        assert_eq!(
            state.save_save_as(&data).unwrap_err().code,
            "forbidden-destination"
        );
    }
    assert_eq!(Sha256::digest(std::fs::read(source).unwrap()), hash);
}
