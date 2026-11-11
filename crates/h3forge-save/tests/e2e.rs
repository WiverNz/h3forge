//! End-to-end opening of synthetic saves through `SaveDocument`: container
//! validation, detection, map text and hero decoding. Unit tests inside the
//! crate cover each failure mode; these tests cover the composed flow.

mod common;

use common::{craft_record, gzip_stored, save_payload};
use h3forge_save::{
    Compatibility, ContainerKind, DiagnosticCode, GameFamily, HeroConfidence, HeroOwner,
    SaveDocument,
};

fn open_synthetic(
    major: u32,
    minor: u32,
    name: &[u8],
    desc: &[u8],
    records: &[(usize, Vec<u8>)],
) -> SaveDocument {
    let payload = save_payload(major, minor, name, desc, records);
    let compressed = gzip_stored(&payload);
    SaveDocument::open(compressed).expect("synthetic save opens")
}

#[test]
fn opens_full_synthetic_save() {
    let record = craft_record(4, b"Aiden___", 2, 1064, 73);
    let at = 30_000 + 500;
    let document = open_synthetic(44, 11, b"Test Map", b"A test map.", &[(at, record.clone())]);

    assert!(document.original_bytes().len() > record.len());
    assert_eq!(document.format.family, Some(GameFamily::HornOfTheAbyss));
    assert_eq!((document.format.major, document.format.minor), (44, 11));
    assert_eq!(document.format.container_kind, ContainerKind::Regular);
    assert_eq!(document.format.compatibility, Compatibility::Readable);

    assert_eq!(document.map.name_raw, b"Test Map");
    assert_eq!(
        document.map.description_raw.as_deref(),
        Some(&b"A test map."[..])
    );

    assert_eq!(document.heroes.len(), 1);
    let hero = &document.heroes[0];
    assert_eq!(hero.confidence, HeroConfidence::High);
    assert_eq!(hero.owner, HeroOwner::Player(4));
    assert_eq!(hero.name_raw, b"Aiden___");
    assert_eq!(hero.level, 2);
    assert_eq!(hero.level_from_experience, 2); // 1064 XP derives level 2: consistent.
    assert_eq!(hero.experience, 1064);
    assert_eq!(hero.spell_points, 73);
    assert_eq!(hero.movement_total, 1500);
    assert_eq!(hero.movement_left, 1500);
    assert_eq!(hero.primary.attack, 3);
    assert_eq!(hero.primary.defense, 7);
    assert_eq!(hero.primary.spell_power, 6);
    assert_eq!(hero.primary.knowledge, 6);
    assert_eq!(hero.skills.len(), 1);
    assert_eq!(
        (
            hero.skills[0].skill_id,
            hero.skills[0].mastery,
            hero.skills[0].slot
        ),
        (7, 2, 1)
    );
    let learned: Vec<usize> = hero
        .spells_learned
        .iter()
        .enumerate()
        .filter(|(_, flag)| **flag == 1)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(learned, vec![3, 15]);
    // The stored record bytes are retrievable for future read-only milestones.
    assert_eq!(
        document.hero_record_bytes(0).expect("record bytes"),
        &record[..]
    );
    assert!(document.hero_record_bytes(99).is_none());
    // Stored level agrees with derived level: no mismatch note for this hero.
    assert!(hero.notes.is_empty());
}

#[test]
fn unknown_magic_is_a_typed_error() {
    let payload = vec![0u8; 256];
    let mut compressed = gzip_stored(&payload);
    // Fix the CRC after mangling is not needed: the payload magic is wrong.
    let error = SaveDocument::open(compressed.clone()).unwrap_err();
    assert!(matches!(
        error,
        h3forge_save::SaveError::UnknownFormat { .. }
    ));
    // A valid gzip of a non-save also errors, and the compressed input is intact.
    compressed[5] = 0;
    assert!(SaveDocument::open(compressed).is_err());
}

#[test]
fn unknown_version_is_unsupported_with_diagnostic() {
    let document = open_synthetic(1, 0, b"Old Map", b"", &[]);
    assert_eq!(document.format.compatibility, Compatibility::Unsupported);
    assert_eq!(document.heroes.len(), 0);
    assert!(
        document
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::UnrecognizedVersion)
    );
}

#[test]
fn recognized_non_hota_skips_hero_scan_with_diagnostic() {
    let document = open_synthetic(42, 0, b"SoD Map", b"", &[]);
    assert_eq!(document.format.family, Some(GameFamily::ShadowOfDeath));
    assert_eq!(document.format.compatibility, Compatibility::Readable);
    assert_eq!(document.heroes.len(), 0);
    assert!(
        document
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::HeroScanNotImplemented)
    );
}

#[test]
fn low_confidence_hero_still_lists_with_notes() {
    let mut record = craft_record(4, b"Aiden___", 2, 1064, 73);
    // Passes the loose structure check but fails the strict equipment
    // grammar: tail word is neither 0000, FFFFFFFF, nor ?? ?? 00 00.
    let at = h3forge_save::definitions::hero::EQUIPMENT + 2 * 8;
    record[at..at + 8].copy_from_slice(&[9, 0, 0, 0, 5, 6, 7, 8]);
    let document = open_synthetic(44, 11, b"Test Map", b"", &[(30_500, record)]);
    assert_eq!(document.heroes.len(), 1);
    let hero = &document.heroes[0];
    assert_eq!(hero.confidence, HeroConfidence::Low);
    assert!(hero.notes.iter().any(|note| matches!(
        note,
        h3forge_save::HeroNote::Rejected(h3forge_save::RejectReason::EquipmentInvalid)
    )));
    assert_eq!(document.confirmed_heroes().count(), 0);
}
