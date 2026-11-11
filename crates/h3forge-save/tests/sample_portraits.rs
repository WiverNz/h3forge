//! Opt-in portrait identity verification against the real sample save.
//!
//! Evidence chain validated here (H3F-005): the runtime hero record carries
//! `hero_type` / `hero_class` / `hero_portrait` at offsets 10/11/12 (h3svg
//! specification), renamed heroes keep their type, and unchanged CP1251
//! roster names sit at type-consistent classes. Without the fixture the test
//! passes without touching anything.

use std::path::{Path, PathBuf};

use h3forge_save::SaveDocument;

fn fixture_copy() -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from)?;
    let source = root.join("SaAndVla.GM2");
    if !source.is_file() {
        return None;
    }
    let destination = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/fixtures");
    std::fs::create_dir_all(&destination).ok()?;
    let destination = destination.join("SaAndVla.GM2");
    std::fs::copy(&source, &destination).ok()?;
    Some(destination)
}

#[test]
fn sample_portrait_identity_matches_research() {
    let Some(copy) = fixture_copy() else {
        eprintln!("no private sample copy; skipping portrait identity test");
        return;
    };
    let document = SaveDocument::open(std::fs::read(&copy).expect("sample reads")).expect("opens");

    // Every located HotA record carries the three identity bytes.
    assert_eq!(document.heroes.len(), 215);
    for hero in &document.heroes {
        let identity = hero.portrait.expect("located records carry identity bytes");
        assert!(identity.hero_class < 24, "class byte in documented range");
    }

    // Known identities: type 0 = Orrin (CP1251  ce f0 f0 e8 ed), unchanged
    // roster names at type-consistent classes, and renames that keep type.
    let identity_at = |index: usize| document.heroes[index].portrait.unwrap();
    assert_eq!(
        identity_at(0),
        h3forge_save::PortraitIdentity {
            hero_type: 0,
            hero_class: 0, // Knight
            portrait_id: 0,
        },
        "record 0 is Orrin (knight), portrait 0"
    );
    assert_eq!(
        identity_at(1).hero_type,
        1,
        "renamed 'AbsoluteZero' keeps hero type 1"
    );
    assert_eq!(
        identity_at(16).hero_class,
        2,
        "record 16 (Мефала/Mephala) is a Ranger"
    );

    // Documented sample statistics: 147 default portraits (portrait == type)
    // and 68 customized ones; custom portrait IDs exceed the 130 standard
    // files, so those must fall back to placeholders upstream.
    let default_portraits = document
        .heroes
        .iter()
        .filter(|hero| {
            let identity = hero.portrait.unwrap();
            identity.portrait_id == identity.hero_type
        })
        .count();
    assert_eq!(default_portraits, 147);
    let max_portrait = document
        .heroes
        .iter()
        .map(|hero| hero.portrait.unwrap().portrait_id)
        .max()
        .unwrap();
    assert_eq!(max_portrait, 244);
}
