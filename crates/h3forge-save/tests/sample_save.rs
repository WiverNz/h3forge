//! Opt-in test against a private copy of the real sample save.
//!
//! Set `H3FORGE_FIXTURE_ROOT` to a directory containing a copy of
//! `SaAndVla.GM2` (from the private `games/` reference set). Without the
//! variable this test passes without touching anything. The sample bytes are
//! only read; the test works on a copy written under the ignored
//! `.local/fixtures/` directory, never in `games/` or `Data/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use h3forge_save::{
    Compatibility, ContainerKind, GameFamily, HeroConfidence, HeroOwner, SaveDocument,
};
use sha2::{Digest, Sha256};

/// Recorded fingerprint from research (RESEARCH.md "Actual sample").
const SAMPLE_SHA256: &str = "f326b6ff04cb2b6d3fd7e44773dfa78eecca28f02277ff201b620c215b3856b5";
const SAMPLE_NAME: &[u8] = b"- The Mysterious Island - COOP";
const EXPECTED_PAYLOAD: usize = 5_271_294;
const EXPECTED_HEROES: usize = 215;

fn repo_root() -> PathBuf {
    // crates/h3forge-save -> repository root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn copy_destination() -> PathBuf {
    repo_root().join(".local/fixtures/SaAndVla.GM2")
}

#[test]
fn sample_copy_parses() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("H3FORGE_FIXTURE_ROOT not set; skipping private sample test");
        return;
    };
    let source = PathBuf::from(&root).join("SaAndVla.GM2");
    if !source.is_file() {
        eprintln!("no SaAndVla.GM2 under {root:?}; skipping private sample test");
        return;
    }

    let bytes = std::fs::read(&source).expect("sample copy reads");
    let digest = Sha256::digest(&bytes);
    assert_eq!(
        format!("{digest:x}"),
        SAMPLE_SHA256,
        "sample fingerprint must match the recorded research baseline"
    );

    // Work only on a copy in the ignored fixture directory. The destination
    // is fixed and canonical; it can never resolve into games/ or Data/.
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
    assert!(canonical.starts_with(repo.join(".local/fixtures")));

    let copy_bytes = std::fs::read(&canonical).expect("copied sample reads");
    let document = SaveDocument::open(copy_bytes).expect("sample opens");

    // Container facts from research.
    assert_eq!(document.payload().len(), EXPECTED_PAYLOAD);
    assert_eq!(document.format.family, Some(GameFamily::HornOfTheAbyss));
    assert_eq!((document.format.major, document.format.minor), (44, 11));
    assert_eq!(document.format.container_kind, ContainerKind::Regular);
    assert_eq!(document.format.compatibility, Compatibility::Readable);

    // Map text.
    assert_eq!(document.map.name_raw, SAMPLE_NAME);
    assert!(document.map.description_raw.is_some());

    // Hero records reproduce the research baseline exactly.
    assert_eq!(document.heroes.len(), EXPECTED_HEROES);
    assert!(
        document
            .heroes
            .iter()
            .all(|hero| hero.confidence == HeroConfidence::High)
    );
    assert_eq!(document.heroes[0].span.start, 4_657_602);
    assert_eq!(document.heroes[EXPECTED_HEROES - 1].span.start, 4_921_020);

    // Owner distribution recorded by research.
    let mut owners = HashMap::new();
    for hero in &document.heroes {
        *owners.entry(hero.owner).or_insert(0usize) += 1;
    }
    let neutral = owners.get(&HeroOwner::Neutral).copied().unwrap_or(0);
    assert_eq!(neutral, 179, "neutral records");
    for (player, count) in [(0u8, 5usize), (4, 14), (6, 12), (7, 5)] {
        assert_eq!(
            owners.get(&HeroOwner::Player(player)).copied().unwrap_or(0),
            count,
            "owner {player} records"
        );
    }

    // Controlled-edit research candidate at offset 4,727,078.
    let target = document
        .heroes
        .iter()
        .find(|hero| hero.span.start == 4_727_078)
        .expect("research candidate record present");
    assert_eq!(target.owner, HeroOwner::Player(4));
    assert_eq!(target.level, 2);
    assert_eq!(target.experience, 1064);
    assert_eq!(target.spell_points, 73);
    assert_eq!(target.primary.attack, 3);
    assert_eq!(target.primary.defense, 7);
    assert_eq!(target.primary.spell_power, 6);
    assert_eq!(target.primary.knowledge, 6);
    let learned: Vec<usize> = target
        .spells_learned
        .iter()
        .enumerate()
        .filter(|(_, flag)| **flag == 1)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        learned,
        vec![3, 15, 17, 20, 25, 33, 35, 37, 42, 43, 44, 47, 64],
        "learned spell IDs of the owned research candidate"
    );

    // Every hero record decodes to a non-empty name and a plausible level.
    for hero in &document.heroes {
        assert!(!hero.name_raw.is_empty());
        assert!(hero.level >= 1, "hero {} level", hero.id);
    }
}
