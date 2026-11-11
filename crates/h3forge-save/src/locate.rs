//! Hero record location in the decompressed payload.
//!
//! Ported from h3sed `Savefile.populate_heroes`, `metadata.HERO_REGEX`,
//! `metadata.EQUIPMENT_REGEX` and `version/hota.HERO_REGEX` (MIT copyright
//! 2020 Erki Suurjaak; see licenses/h3sed-MIT.txt). This is heuristic record
//! discovery, not a complete file schema: the checks below are exactly the
//! ones h3sed uses to accept a hero, so a pass means "verified against every
//! implemented structural check", never "game-verified". Records that match
//! the base structure but fail a secondary check are kept as low-confidence
//! candidates with explicit reasons instead of being silently dropped.

use std::ops::Range;

use crate::definitions::{
    HERO_SCAN_START, HERO_SCAN_WINDOW,
    hero::{
        EQUIPMENT, EQUIPMENT_SLOT_LEN, EQUIPMENT_SLOTS, INVENTORY, INVENTORY_ENTRIES, NAME,
        NAME_LEN, OWNER, RECORD_LEN, RECORD_LEN_LEGACY, SKILL_COUNT, SKILL_COUNT_MAX,
        SKILL_LEVEL_MAX, SKILL_LEVELS, SKILL_SLOT_MAX, SKILL_SLOTS_HOT_A, SPELL_FLAG_COUNT,
        SPELL_FLAG_MAX, SPELLS_AVAILABLE, SPELLS_LEARNED,
    },
};

/// Why a record is not a confirmed hero record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// Name bytes are not printable content followed by NUL padding
    /// (h3sed `rgx_strip`: rejects control bytes and all-0xFF content).
    NameNotStrippable,
    /// The 152-byte equipment section fails h3sed's `EQUIPMENT_REGEX`.
    EquipmentInvalid,
    /// Owner byte is neither a player 0-7 nor neutral 255.
    OwnerOutOfRange,
}

/// A located hero record and its verification status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedHero {
    /// Byte span of the fixed record in the payload.
    pub span: Range<usize>,
    /// `true` when every implemented check passed; `false` when the record
    /// only matched the base structure (see `rejections`).
    pub verified: bool,
    /// Reasons the record is not confirmed; empty iff `verified`.
    pub rejections: Vec<RejectReason>,
}

impl LocatedHero {
    /// Record length in bytes (1121 or 1122 for the HotA layout).
    pub fn len(&self) -> usize {
        self.span.end - self.span.start
    }

    /// Whether the record is empty; always false for located records, kept
    /// for API completeness with clippy's `len_without_is_empty`.
    pub fn is_empty(&self) -> bool {
        self.span.is_empty()
    }
}

/// Scans the payload for HotA-layout hero records, replaying h3sed's
/// strategy: start past the campaign carry-over area, accept records that
/// pass every check, and once the heroes section is reached search only
/// `HERO_SCAN_WINDOW` bytes ahead of the previous match.
pub fn locate_heroes(payload: &[u8]) -> Vec<LocatedHero> {
    let mut heroes = Vec::new();
    let mut pos = HERO_SCAN_START.min(payload.len());
    loop {
        let window_end = if heroes.is_empty() {
            payload.len()
        } else {
            (pos + HERO_SCAN_WINDOW).min(payload.len())
        };
        let Some(offset) = find_next_candidate(payload, pos, window_end) else {
            break;
        };
        let record_len =
            record_len_at(payload, offset, window_end).expect("candidate implies full record");
        let absolute = offset;
        let record = &payload[absolute..absolute + record_len];
        let rejections = secondary_checks(record);
        if rejections.is_empty() {
            heroes.push(LocatedHero {
                span: absolute..absolute + record_len,
                verified: true,
                rejections,
            });
            pos = absolute + record_len;
        } else {
            heroes.push(LocatedHero {
                span: absolute..absolute + record_len,
                verified: false,
                rejections,
            });
            pos = absolute + 1;
        }
    }
    heroes
}

/// Finds the next structural candidate in `payload[start..end]`, or `None`.
/// Like h3sed's windowed regex, the whole record must fit within `end`.
fn find_next_candidate(payload: &[u8], start: usize, end: usize) -> Option<usize> {
    let mut offset = start;
    while offset + RECORD_LEN_LEGACY <= end {
        if record_len_at(payload, offset, end).is_some() {
            return Some(offset);
        }
        offset += 1;
    }
    None
}

/// Returns the record length when the HotA record structure matches at
/// `offset`, or `None`. Cheap, highly selective checks run first; every
/// access is bounds-checked against `payload` and the scan window `limit`.
fn record_len_at(payload: &[u8], offset: usize, limit: usize) -> Option<usize> {
    // The shortest record must fit; the 30-slot variant needs one byte more.
    let record = payload.get(offset..offset + RECORD_LEN_LEGACY)?;
    if offset + RECORD_LEN_LEGACY > limit {
        return None;
    }
    // Skill count: low byte 0x00-0x1C, remaining three bytes zero.
    if record[SKILL_COUNT] > SKILL_COUNT_MAX
        || record[SKILL_COUNT + 1..SKILL_COUNT + 4] != [0, 0, 0]
    {
        return None;
    }
    // Name: first byte above 0x20, 13th byte NUL.
    if record[NAME] <= 0x20 || record[NAME + NAME_LEN - 1] != 0 {
        return None;
    }
    // Learned and available spell flags: 0 or 1.
    if !record[SPELLS_LEARNED..SPELLS_LEARNED + SPELL_FLAG_COUNT]
        .iter()
        .all(|&flag| flag <= SPELL_FLAG_MAX)
        || !record[SPELLS_AVAILABLE..SPELLS_AVAILABLE + SPELL_FLAG_COUNT]
            .iter()
            .all(|&flag| flag <= SPELL_FLAG_MAX)
    {
        return None;
    }
    // Equipment: 19 slots, each either FF FF FF FF + anything or ID + 3 NULs + anything.
    for slot in 0..EQUIPMENT_SLOTS {
        let at = EQUIPMENT + slot * EQUIPMENT_SLOT_LEN;
        let word = &record[at..at + 4];
        if !(word == [0xFF; 4] || word[1..] == [0, 0, 0]) {
            return None;
        }
    }
    // Inventory: 64 entries, each of two words, word = FF FF FF FF or ID + 3 NULs.
    for entry in 0..INVENTORY_ENTRIES {
        let at = INVENTORY + entry * 8;
        if !is_inventory_word(&record[at..at + 4]) || !is_inventory_word(&record[at + 4..at + 8]) {
            return None;
        }
    }
    // Skill levels: 30 mastery bytes 0-3.
    if !record[SKILL_LEVELS..SKILL_LEVELS + 30]
        .iter()
        .all(|&level| level <= SKILL_LEVEL_MAX)
    {
        return None;
    }
    // Trailing HotA skill-slot run: prefer 30 bytes (44/10+), accept 29.
    if offset + RECORD_LEN <= limit.min(payload.len()) {
        let tail = &payload[offset + SKILL_SLOTS_HOT_A..offset + RECORD_LEN];
        if tail.iter().all(|&slot| slot <= SKILL_SLOT_MAX) {
            return Some(RECORD_LEN);
        }
    }
    if record[SKILL_SLOTS_HOT_A..]
        .iter()
        .all(|&slot| slot <= SKILL_SLOT_MAX)
    {
        return Some(RECORD_LEN_LEGACY);
    }
    None
}

/// h3sed equipment/inventory word semantics: all-0xFF or ID + three NULs.
fn is_blank_word(word: &[u8]) -> bool {
    word == [0xFF; 4]
}

/// Inventory word: blank or ID + three NULs.
fn is_inventory_word(word: &[u8]) -> bool {
    is_blank_word(word) || word[1..] == [0, 0, 0]
}

/// Secondary verification h3sed applies to each structural match: the name
/// strip, the equipment section grammar, and (added by H3Forge) owner range.
fn secondary_checks(record: &[u8]) -> Vec<RejectReason> {
    let mut rejections = Vec::new();
    if !name_strips(&record[NAME..NAME + NAME_LEN]) {
        rejections.push(RejectReason::NameNotStrippable);
    }
    if !equipment_valid(&record[EQUIPMENT..EQUIPMENT + EQUIPMENT_SLOTS * EQUIPMENT_SLOT_LEN]) {
        rejections.push(RejectReason::EquipmentInvalid);
    }
    if !matches!(record[OWNER], 0..=7 | 0xFF) {
        rejections.push(RejectReason::OwnerOutOfRange);
    }
    rejections
}

/// h3sed `rgx_strip`: `^(?!\xFF+\x00+$)([^\x00-\x19]+)\x00+$` - some bytes
/// >= 0x1A, then NUL padding to the end, and not purely 0xFF content.
fn name_strips(name: &[u8]) -> bool {
    let mut content_len = 0usize;
    while content_len < name.len() && name[content_len] >= 0x1A {
        content_len += 1;
    }
    if content_len == 0 || content_len == name.len() {
        return false; // Empty content, or no NUL padding at all.
    }
    if name[content_len..].iter().any(|&byte| byte != 0) {
        return false; // Garbage after content.
    }
    // Reject all-0xFF content: not a real name.
    !name[..content_len].iter().all(|&byte| byte == 0xFF)
}

/// h3sed `EQUIPMENT_REGEX`: reject all-zero sections and sections that are
/// pure `(0000 0000 FFFF FFFF)` repetition, then require every 8-byte slot
/// to be blank (FF4 + anything), an artifact/scroll (ID + 3 NULs + 0000 or
/// FFFF FFFF), or a war machine (ID + 3 NULs + 2 bytes + 00 00).
///
/// One deliberate difference: h3sed's trailing `$` tolerates a final 0x0A
/// byte; this port requires the whole 152-byte section to satisfy the slot
/// grammar. The sample replay (215/215 accepted) is unaffected, and the
/// stricter check only turns pathological sections into candidates.
fn equipment_valid(section: &[u8]) -> bool {
    if section.len() != EQUIPMENT_SLOTS * EQUIPMENT_SLOT_LEN {
        return false;
    }
    if section.iter().all(|&byte| byte == 0) {
        return false;
    }
    let slots = &section.as_chunks::<EQUIPMENT_SLOT_LEN>().0;
    if slots
        .iter()
        .all(|slot| slot[..4] == [0, 0, 0, 0] && slot[4..] == [0xFF; 4])
    {
        return false;
    }
    slots.iter().all(|slot| {
        if slot[..4] == [0xFF; 4] {
            return true; // Blank spot.
        }
        if slot[1..4] != [0, 0, 0] {
            return false; // Not an ID word.
        }
        slot[4..] == [0, 0, 0, 0] || slot[4..] == [0xFF; 4] || slot[6..] == [0, 0]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::hero::{
        ARMY_COUNTS, ARMY_IDS, ATTACK, DEFENSE, EXPERIENCE, KNOWLEDGE, LEVEL, MOVEMENT_LEFT,
        MOVEMENT_TOTAL, SPELL_POINTS, SPELL_POWER,
    };

    /// Builds a structurally valid 1122-byte HotA record with tweakable bytes.
    fn craft_record() -> Vec<u8> {
        let mut record = vec![0x55u8; RECORD_LEN]; // 0x55 is never a valid constrained byte.
        record[OWNER] = 4;
        record[MOVEMENT_TOTAL..MOVEMENT_TOTAL + 4].copy_from_slice(&1500u32.to_le_bytes());
        record[MOVEMENT_LEFT..MOVEMENT_LEFT + 4].copy_from_slice(&1500u32.to_le_bytes());
        record[EXPERIENCE..EXPERIENCE + 4].copy_from_slice(&1064u32.to_le_bytes());
        record[SKILL_COUNT] = 3;
        record[SKILL_COUNT + 1..SKILL_COUNT + 4].fill(0);
        record[SPELL_POINTS..SPELL_POINTS + 2].copy_from_slice(&73u16.to_le_bytes());
        record[LEVEL] = 2;
        record[ARMY_IDS..ARMY_IDS + 28].fill(0);
        record[ARMY_COUNTS..ARMY_COUNTS + 28].fill(0);
        let name = b"Aiden";
        record[NAME..NAME + name.len()].copy_from_slice(name);
        record[NAME + name.len()..NAME + NAME_LEN].fill(0);
        record[SKILL_LEVELS..SKILL_LEVELS + 30].fill(0);
        record[ATTACK] = 3;
        record[DEFENSE] = 7;
        record[SPELL_POWER] = 6;
        record[KNOWLEDGE] = 6;
        record[SPELLS_LEARNED..SPELLS_LEARNED + SPELL_FLAG_COUNT].fill(0);
        record[SPELLS_AVAILABLE..SPELLS_AVAILABLE + SPELL_FLAG_COUNT].fill(0);
        // Equipment: all blank (FF4 + 0000).
        for slot in 0..EQUIPMENT_SLOTS {
            let at = EQUIPMENT + slot * EQUIPMENT_SLOT_LEN;
            record[at..at + 4].fill(0xFF);
            record[at + 4..at + 8].fill(0);
        }
        // Inventory: all blank entries.
        for entry in 0..INVENTORY_ENTRIES {
            record[INVENTORY + entry * 8..INVENTORY + entry * 8 + 8].fill(0xFF);
        }
        record[SKILL_SLOTS_HOT_A..SKILL_SLOTS_HOT_A + 30].fill(0);
        record
    }

    fn embed(record: &[u8], at: usize) -> Vec<u8> {
        let mut payload = vec![0u8; 40_000];
        payload[..5].copy_from_slice(b"H3SVG");
        payload[8..12].copy_from_slice(&44u32.to_le_bytes());
        payload[12..16].copy_from_slice(&11u32.to_le_bytes());
        payload[at..at + record.len()].copy_from_slice(record);
        payload
    }

    #[test]
    fn finds_crafted_record() {
        let record = craft_record();
        let payload = embed(&record, HERO_SCAN_START + 100);
        let heroes = locate_heroes(&payload);
        assert_eq!(heroes.len(), 1);
        assert!(heroes[0].verified);
        assert_eq!(
            heroes[0].span,
            HERO_SCAN_START + 100..HERO_SCAN_START + 100 + RECORD_LEN
        );
    }

    #[test]
    fn rejects_and_keeps_bad_equipment_as_candidate() {
        // Passes the loose structure check (ID word + anything) but fails the
        // strict equipment grammar: tail word is neither 0000, FFFFFFFF, nor
        // ?? ?? 00 00.
        let mut record = craft_record();
        let at = EQUIPMENT + 3 * EQUIPMENT_SLOT_LEN;
        record[at..at + 8].copy_from_slice(&[9, 0, 0, 0, 5, 6, 7, 8]);
        let payload = embed(&record, HERO_SCAN_START + 100);
        let heroes = locate_heroes(&payload);
        assert_eq!(heroes.len(), 1);
        assert!(!heroes[0].verified);
        assert!(
            heroes[0]
                .rejections
                .contains(&RejectReason::EquipmentInvalid)
        );
    }

    #[test]
    fn rejects_and_keeps_bad_name_as_candidate() {
        // Structure allows any first byte > 0x20 and NUL at offset 12; the
        // strip check rejects purely 0xFF content.
        let mut record = craft_record();
        record[NAME..NAME + NAME_LEN - 1].fill(0xFF);
        record[NAME + NAME_LEN - 1] = 0;
        let payload = embed(&record, HERO_SCAN_START + 100);
        let heroes = locate_heroes(&payload);
        assert_eq!(heroes.len(), 1);
        assert!(!heroes[0].verified);
        assert!(
            heroes[0]
                .rejections
                .contains(&RejectReason::NameNotStrippable)
        );
    }

    #[test]
    fn rejects_and_keeps_bad_owner_as_candidate() {
        let mut record = craft_record();
        record[OWNER] = 9;
        let payload = embed(&record, HERO_SCAN_START + 100);
        let heroes = locate_heroes(&payload);
        assert_eq!(heroes.len(), 1);
        assert!(!heroes[0].verified);
        assert!(
            heroes[0]
                .rejections
                .contains(&RejectReason::OwnerOutOfRange)
        );
    }

    #[test]
    fn finds_multiple_dense_records() {
        let record = craft_record();
        let mut payload = vec![0u8; 60_000];
        payload[..5].copy_from_slice(b"H3SVG");
        payload[8..12].copy_from_slice(&44u32.to_le_bytes());
        payload[12..16].copy_from_slice(&11u32.to_le_bytes());
        // Three records with the dense 38-byte gaps of the real heroes section.
        let mut at = HERO_SCAN_START;
        for _ in 0..3 {
            payload[at..at + RECORD_LEN].copy_from_slice(&record);
            at += RECORD_LEN + 38;
        }
        let heroes = locate_heroes(&payload);
        assert_eq!(heroes.len(), 3);
        assert!(heroes.iter().all(|hero| hero.verified));
    }

    #[test]
    fn stops_when_gap_exceeds_window() {
        let record = craft_record();
        let mut payload = vec![0u8; 80_000];
        payload[..5].copy_from_slice(b"H3SVG");
        payload[8..12].copy_from_slice(&44u32.to_le_bytes());
        payload[12..16].copy_from_slice(&11u32.to_le_bytes());
        payload[HERO_SCAN_START..HERO_SCAN_START + RECORD_LEN].copy_from_slice(&record);
        // Second record just beyond the 5000-byte post-match window.
        let far = HERO_SCAN_START + RECORD_LEN + HERO_SCAN_WINDOW + 1;
        payload[far..far + RECORD_LEN].copy_from_slice(&record);
        let heroes = locate_heroes(&payload);
        assert_eq!(heroes.len(), 1);
    }
}
