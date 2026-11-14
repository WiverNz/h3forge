//! End-to-end town discovery and garrison editing on synthetic saves: no
//! fixture data needed. Covers the composed `SaveDocument` flow the unit
//! tests leave to integration: town parsing beside heroes, garrison-hero
//! linkage by hero type, edit planning, round-trip byte preservation, and the
//! format/confidence gates.

mod common;

use common::{craft_record, gzip_stored, save_payload};
use h3forge_save::definitions::towns::{self, GARRISON_COUNTS, GARRISON_IDS};
use h3forge_save::{
    EditError, HeroOwner, SaveDocument, TownConfidence, TownEdit, TownRejectReason,
};

/// Structurally valid town record with the given name/owner/type/coordinates.
fn craft_town(name: &[u8], owner: u8, type_: u8, x: u8, y: u8, z: u8) -> Vec<u8> {
    let mut record = vec![0u8; towns::RECORD_FIXED + name.len()];
    record[towns::OWNER] = owner;
    record[towns::TYPE] = type_;
    record[towns::X] = x;
    record[towns::Y] = y;
    record[towns::Z] = z;
    for slot in 0..towns::GARRISON_SLOTS {
        record[GARRISON_IDS + slot * 4..GARRISON_IDS + slot * 4 + 4].fill(0xFF);
        record[GARRISON_COUNTS + slot * 4..GARRISON_COUNTS + slot * 4 + 4].fill(0);
    }
    record[towns::GARRISONED_HERO] = 0xFF;
    record[towns::VISITING_HERO] = 0xFF;
    record[towns::MAGE_GUILD_LEVEL] = 3;
    record[towns::NAME_LEN..towns::NAME_LEN + 2]
        .copy_from_slice(&(name.len() as u16).to_le_bytes());
    record[towns::NAME..towns::NAME + name.len()].copy_from_slice(name);
    record
}

fn open(major: u32, minor: u32, records: &[(usize, Vec<u8>)]) -> SaveDocument {
    let payload = save_payload(major, minor, b"Test Map", b"A test map.", records);
    SaveDocument::open(gzip_stored(&payload)).expect("synthetic save opens")
}

const HERO_AT: usize = 4_000_000;
const TOWN_AT: usize = HERO_AT - towns::RECORD_FIXED - 20;

#[test]
fn town_parsed_beside_hero() {
    let town = craft_town(b"Testburg", 4, 2, 10, 20, 0);
    let hero = craft_record(4, b"Aiden___", 2, 1064, 73);
    let document = open(44, 11, &[(TOWN_AT, town), (HERO_AT, hero)]);
    assert_eq!(document.heroes.len(), 1);
    assert_eq!(document.towns.len(), 1);
    let town = &document.towns[0];
    assert_eq!(town.confidence, TownConfidence::High);
    assert_eq!(town.owner, HeroOwner::Player(4));
    assert_eq!(town.town_type, 2);
    assert_eq!((town.x, town.y, town.z), (10, 20, 0));
    assert_eq!(town.name_raw, b"Testburg");
    assert_eq!(town.name_lossy, "Testburg");
    assert_eq!(town.mage_guild_level, 3);
    assert!(town.garrison.iter().all(|slot| slot.is_empty()));
    assert_eq!(town.garrison_hero, None);
}

#[test]
fn garrison_edit_roundtrip_exact_diff() {
    let town = craft_town(b"Testburg", 4, 2, 10, 20, 0);
    let hero = craft_record(4, b"Aiden___", 2, 1064, 73);
    let document = open(44, 11, &[(TOWN_AT, town), (HERO_AT, hero)]);
    let plan = document
        .plan_town_edits(
            0,
            &[
                TownEdit::SetGarrisonSlot {
                    slot: 0,
                    creature_id: Some(0),
                    count: 300,
                },
                TownEdit::SetGarrisonSlot {
                    slot: 6,
                    creature_id: Some(199),
                    count: 12_345,
                },
            ],
        )
        .expect("plans");
    assert_eq!(plan.len(), 4);
    let bytes = document.serialize_with_patches(&plan).expect("serializes");
    let reopened = SaveDocument::open(bytes).expect("reopens");
    let town = &reopened.towns[0];
    assert_eq!(town.garrison[0].creature_id, 0);
    assert_eq!(town.garrison[0].count, 300);
    assert_eq!(town.garrison[6].creature_id, 199);
    assert_eq!(town.garrison[6].count, 12_345);
    assert!(town.garrison[1].is_empty());
    // Everything outside the four patched words is byte-identical.
    for (index, (before, after)) in document
        .payload()
        .iter()
        .zip(reopened.payload())
        .enumerate()
    {
        if before != after {
            let in_word = [0usize, 6].iter().any(|&slot| {
                (TOWN_AT + GARRISON_IDS + slot * 4..TOWN_AT + GARRISON_IDS + slot * 4 + 4)
                    .contains(&index)
                    || (TOWN_AT + GARRISON_COUNTS + slot * 4
                        ..TOWN_AT + GARRISON_COUNTS + slot * 4 + 4)
                        .contains(&index)
            });
            assert!(in_word, "unexpected diff at {index}");
        }
    }
    // No-op edits plan no patches; a clear writes the FF/0 empty form.
    let clear = reopened
        .plan_town_edits(
            0,
            &[TownEdit::SetGarrisonSlot {
                slot: 0,
                creature_id: None,
                count: 0,
            }],
        )
        .expect("plans clear");
    assert_eq!(clear.len(), 2);
    let cleared = SaveDocument::open(reopened.serialize_with_patches(&clear).unwrap()).unwrap();
    assert_eq!(cleared.towns[0].garrison[0].creature_id, u32::MAX);
    assert_eq!(cleared.towns[0].garrison[0].count, 0);
    assert!(
        cleared
            .plan_town_edits(
                0,
                &[TownEdit::SetGarrisonSlot {
                    slot: 1,
                    creature_id: None,
                    count: 0
                }]
            )
            .unwrap()
            .is_empty(),
        "clearing an already-empty slot is a no-op"
    );
}

#[test]
fn garrison_hero_linkage_and_block() {
    // Town names hero type 8 as its garrisoned hero; the hero record carries
    // type byte 8 and matching front-matter coordinates.
    let mut town = craft_town(b"Testburg", 4, 2, 10, 20, 0);
    town[towns::GARRISONED_HERO] = 8;
    let hero = craft_record(4, b"Aiden___", 2, 1064, 73);
    let payload = save_payload(
        44,
        11,
        b"Test Map",
        b"A test map.",
        &[(TOWN_AT, town), (HERO_AT, hero)],
    );
    // Hero front matter: x u16 @ -26, y @ -24, z @ -22, visible @ -20.
    let mut payload = payload;
    payload[HERO_AT - 26..HERO_AT - 24].copy_from_slice(&10u16.to_le_bytes());
    payload[HERO_AT - 24..HERO_AT - 22].copy_from_slice(&20u16.to_le_bytes());
    payload[HERO_AT - 22] = 0;
    payload[HERO_AT - 20] = 0;
    // Hero type byte at record offset 10.
    payload[HERO_AT + 10] = 8;
    let document = SaveDocument::open(gzip_stored(&payload)).expect("opens");
    let town = &document.towns[0];
    assert_eq!(town.garrison_hero, Some(0));
    assert!(
        !town.notes.iter().any(|note| matches!(
            note,
            h3forge_save::TownNote::GarrisonHeroLocationMismatch { .. }
        )),
        "matching coordinates raise no mismatch note"
    );
    // The garrison belongs to the hero: town edits are rejected, hero edits stand.
    assert!(matches!(
        document.plan_town_edits(
            0,
            &[TownEdit::SetGarrisonSlot {
                slot: 0,
                creature_id: Some(0),
                count: 1
            }]
        ),
        Err(EditError::InvalidValue { .. })
    ));
    assert!(
        document
            .plan_hero_edits(
                0,
                &[h3forge_save::HeroEdit::SetArmySlot {
                    slot: 0,
                    creature_id: Some(0),
                    count: 1
                }]
            )
            .is_ok()
    );
}

#[test]
fn unsupported_format_and_low_confidence_stay_read_only() {
    let town = craft_town(b"Testburg", 4, 2, 10, 20, 0);
    let hero = craft_record(4, b"Aiden___", 2, 1064, 73);
    // HotA 44/5: readable and scanned, but no validated edit support.
    let document = open(44, 5, &[(TOWN_AT, town.clone()), (HERO_AT, hero.clone())]);
    assert_eq!(document.towns.len(), 1);
    assert!(matches!(
        document.plan_town_edits(
            0,
            &[TownEdit::SetGarrisonSlot {
                slot: 0,
                creature_id: Some(0),
                count: 1
            }]
        ),
        Err(EditError::UnsupportedFormat { .. })
    ));

    // 44/11 with an unverified (candidate) town: no edits, still listed.
    let mut bad = craft_town(b"Testburg", 4, 2, 10, 20, 0);
    bad[GARRISON_COUNTS..GARRISON_COUNTS + 4].copy_from_slice(&9u32.to_le_bytes());
    let document = open(44, 11, &[(TOWN_AT, bad), (HERO_AT, hero)]);
    assert_eq!(document.towns.len(), 1);
    assert_eq!(document.towns[0].confidence, TownConfidence::Low);
    assert!(document.towns[0].notes.iter().any(|note| matches!(
        note,
        h3forge_save::TownNote::Rejected(TownRejectReason::GarrisonInconsistent)
    )));
    assert!(matches!(
        document.plan_town_edits(
            0,
            &[TownEdit::SetGarrisonSlot {
                slot: 1,
                creature_id: Some(0),
                count: 1
            }]
        ),
        Err(EditError::TownNotEditable { .. })
    ));
}

#[test]
fn unknown_garrison_creature_makes_town_read_only_but_listed() {
    let mut town = craft_town(b"Testburg", 4, 2, 10, 20, 0);
    // Slot 2: unknown creature 210 (word shape intact), count 7.
    town[GARRISON_IDS + 4..GARRISON_IDS + 8].copy_from_slice(&210u32.to_le_bytes());
    town[GARRISON_COUNTS + 4..GARRISON_COUNTS + 8].copy_from_slice(&7u32.to_le_bytes());
    let hero = craft_record(4, b"Aiden___", 2, 1064, 73);
    let document = open(44, 11, &[(TOWN_AT, town), (HERO_AT, hero)]);
    let town_view = &document.towns[0];
    // The record stays listed for inspection but fails verification: every
    // garrison write is blocked, not just the unknown stack.
    assert_eq!(town_view.confidence, TownConfidence::Low);
    assert!(town_view.notes.iter().any(|note| matches!(
        note,
        h3forge_save::TownNote::Rejected(TownRejectReason::UnknownCreature)
    )));
    assert!(!town_view.garrison[1].is_supported());
    for slot in [0u8, 1, 2] {
        assert!(
            matches!(
                document.plan_town_edits(
                    0,
                    &[TownEdit::SetGarrisonSlot {
                        slot,
                        creature_id: Some(0),
                        count: 1
                    }]
                ),
                Err(EditError::TownNotEditable { .. })
            ),
            "slot {slot} stays read-only on an unverified town"
        );
    }
}
