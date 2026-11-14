//! Town record location in the decompressed payload (H3F-013).
//!
//! Ported from h3sed `Savefile.parse_towns`, `metadata.TOWN_REGEX` and
//! `version/hota.TOWN_REGEX` (MIT, copyright 2020 Erki Suurjaak; see
//! licenses/h3sed-MIT.txt). Like the hero locator this is heuristic record
//! discovery, not a complete file schema: the towns section sits right before
//! the densely packed hero array, record length is learned from adjacency
//! (HotA 44/11: `RECORD_FIXED` + name length), and candidates that match the
//! base structure but fail a secondary check are kept as low-confidence
//! records with explicit reasons instead of being silently dropped.
//!
//! Cross-validated against the HotA 44/11 sample: 37 towns, every span exact
//! by adjacency, every record's `type` byte equal to the source map's town
//! faction and its coordinates equal to the map town entrance (save town
//! coordinates are single bytes at offsets 4-6).

use std::ops::Range;

use crate::definitions::HERO_SCAN_START;
use crate::definitions::towns::*;

/// Why a record is not a confirmed town record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TownRejectReason {
    /// Garrison ID words and count words disagree about emptiness (one blank,
    /// the other not), or an occupied stack has a zero count (h3sed
    /// `parse_towns` do_skip cases).
    GarrisonInconsistent,
    /// An occupied stack carries a creature ID unknown to the verified 44/11
    /// creature table; the record stays inspectable but unverified.
    UnknownCreature,
}

/// A located town record and its verification status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedTown {
    /// Byte span of the record in the payload. `start..start + RECORD_FIXED +
    /// name_len` for the last record; every earlier record in a dense section
    /// ends exactly where the next begins (adjacency-learned, as h3sed does).
    pub span: Range<usize>,
    /// `true` when every implemented check passed; `false` when the record
    /// only matched the base structure (see `rejections`).
    pub verified: bool,
    /// Reasons the record is not confirmed; empty iff `verified`.
    pub rejections: Vec<TownRejectReason>,
}

impl LocatedTown {
    /// Record length in bytes (`RECORD_FIXED` + name length).
    pub fn len(&self) -> usize {
        self.span.end - self.span.start
    }

    /// Whether the record is empty; always false for located records, kept
    /// for API completeness with clippy's `len_without_is_empty`.
    pub fn is_empty(&self) -> bool {
        self.span.is_empty()
    }
}

/// Scans the payload for HotA-layout town records, replaying h3sed's
/// strategy: search the region before the hero array, accept structurally
/// valid records, learn the record length from the first dense adjacency, and
/// end the section on the first unexplainable gap. `first_hero_start` bounds
/// the scan above; without heroes the whole payload after `HERO_SCAN_START`
/// is searched.
pub fn locate_towns(payload: &[u8], first_hero_start: Option<usize>) -> Vec<LocatedTown> {
    let max_pos = first_hero_start.unwrap_or(payload.len()).min(payload.len());
    let mut pos = match first_hero_start {
        Some(hero_start) => hero_start
            .saturating_sub(TOWNS_MAX * RECORD_MAX + 100)
            .max(HERO_SCAN_START.min(payload.len())),
        None => HERO_SCAN_START.min(payload.len()),
    };
    let mut towns: Vec<LocatedTown> = Vec::new();
    // Learned fixed record length (h3sed `BYTES_MINLEN`): starts at the
    // reference default and adjusts to `next_start - prev_start -
    // prev_name_len` once two towns are on the map.
    let mut fixed_len = RECORD_FIXED_INITIAL;

    while pos + RECORD_FIXED_INITIAL < max_pos.min(payload.len()) {
        let Some(candidate) = find_candidate(payload, pos, max_pos) else {
            break;
        };
        let town_start = candidate;
        if town_start + fixed_len > max_pos {
            break; // Reached the heroes section.
        }
        let name_len = payload[town_start + NAME_LEN] as usize;
        if name_len == 0 || name_len > NAME_MAX {
            pos = town_start + 1;
            continue;
        }
        let Some(record) = payload
            .get(town_start..town_start + fixed_len + name_len)
            .filter(|_| town_start + fixed_len + name_len <= max_pos)
        else {
            break; // Never expose a partially truncated record.
        };
        let name = &record[NAME..NAME + name_len];
        if !name_valid(name) {
            pos = town_start + 1;
            continue;
        }
        let rejections = secondary_checks(&record[..fixed_len + name_len]);
        if !rejections.is_empty() {
            // A rejected (unverified) record still occupies its learned span;
            // keep it and continue after it, like a verified one.
            let span_end = town_start + fixed_len + name_len;
            towns.push(LocatedTown {
                span: town_start..span_end,
                verified: false,
                rejections,
            });
            pos = span_end;
            continue;
        }
        if let Some(last) = towns.last() {
            let uncertainty = if fixed_len + NAME_MAX < RECORD_MAX {
                RECORD_MAX - fixed_len
            } else {
                0
            };
            if town_start > last.span.end + 1 + uncertainty {
                if towns.len() == 1 {
                    // A lone match far from anything else was a false
                    // positive; drop it and keep scanning.
                    towns.pop();
                } else {
                    break; // The towns section has ended.
                }
            }
        }
        if towns.len() == 1
            && fixed_len + NAME_MAX < RECORD_MAX
            && town_start != towns[0].span.end + 1
        {
            // Dense records are back to back; HotA record length varies with
            // the name, so learn the fixed length from this adjacency
            // (h3sed's BYTES_MINLEN adjustment, allowed while the learned
            // length can still explain a maximum-length name).
            let first_name_len = towns[0].len() - fixed_len;
            let learned = town_start - towns[0].span.start;
            let new_fixed = learned.saturating_sub(first_name_len);
            if (RECORD_FIXED_INITIAL - NAME_MAX..=RECORD_MAX).contains(&new_fixed) {
                fixed_len = new_fixed;
                towns[0].span.end = town_start;
            }
        }
        let span_end = town_start + fixed_len + name_len;
        if span_end > max_pos {
            break;
        }
        towns.push(LocatedTown {
            span: town_start..span_end,
            verified: true,
            rejections: Vec::new(),
        });
        pos = span_end;
    }
    towns
}

/// Finds the next structural candidate at or after `pos`, bounded by
/// `max_pos`. Cheap, highly selective checks run first; every access is
/// bounds-checked against `payload` and the section bound.
fn find_candidate(payload: &[u8], pos: usize, max_pos: usize) -> Option<usize> {
    let limit = max_pos.min(payload.len());
    let mut offset = pos;
    while offset + NAME < limit {
        let record = &payload[offset..];
        // Owner byte: player 0-7 or neutral 255 (TOWN_REGEX `faction`).
        if !matches!(record[OWNER], 0..=7 | 0xFF) {
            offset += 1;
            continue;
        }
        // Town type 0-11 (h3sed `TOWN_TYPES`, HotA through Bulwark).
        if record[TYPE] > TYPE_MAX {
            offset += 1;
            continue;
        }
        // Coordinates: x/y 0-252, z 0-1.
        if record[X] > COORD_MAX || record[Y] > COORD_MAX || record[Z] > 1 {
            offset += 1;
            continue;
        }
        // Seven garrison ID words: any byte followed by three 0x00/0xFF bytes.
        let mut army_shape_ok = true;
        for slot in 0..GARRISON_SLOTS {
            let at = GARRISON_IDS + slot * 4;
            if record
                .get(at..at + 4)
                .is_none_or(|word| !word[1..].iter().all(|&byte| byte == 0 || byte == 0xFF))
            {
                army_shape_ok = false;
                break;
            }
        }
        if !army_shape_ok {
            offset += 1;
            continue;
        }
        // Name length: bounded u16 LE, high byte zero for supported names.
        if record[NAME_LEN] == 0
            || record[NAME_LEN] as usize > NAME_MAX
            || record[NAME_LEN + 1] != 0
        {
            offset += 1;
            continue;
        }
        let name_len = record[NAME_LEN] as usize;
        let Some(name) = record.get(NAME..NAME + name_len) else {
            return None; // Ran out of payload.
        };
        if offset + NAME + name_len > limit {
            return None;
        }
        if !name_valid(name) {
            offset += 1;
            continue;
        }
        return Some(offset);
    }
    None
}

/// Length-prefixed legacy text, not a sentinel-terminated string. In CP1251,
/// 0xFF is the letter "я" and must survive verbatim. Control bytes and leading
/// spaces still reject a heuristic candidate; NAME_MAX bounds the length.
fn name_valid(name: &[u8]) -> bool {
    !name.is_empty()
        && name[0] > 0x20
        && name.len() <= NAME_MAX
        && name[1..].iter().all(|&byte| byte >= 0x20)
}

/// Secondary verification h3sed applies to each structural match
/// (`parse_towns`): whole-section emptiness consistency (ID words blank iff
/// count words blank) and per-stack validity (h3sed `do_skip` cases). A
/// record failing these stays a low-confidence candidate; h3sed drops it
/// from the town list entirely, H3Forge keeps it visible but unverified.
fn secondary_checks(record: &[u8]) -> Vec<TownRejectReason> {
    let word =
        |offset: usize| -> [u8; 4] { record[offset..offset + 4].try_into().expect("checked") };
    // Whole-section check: every garrison ID byte 0xFF vs every count byte
    // 0x00/0xFF must agree on blankness.
    let ids_blank = (0..GARRISON_SLOTS).all(|slot| word(GARRISON_IDS + slot * 4) == [0xFF; 4]);
    let counts_blank = (GARRISON_COUNTS..GARRISON_COUNTS + GARRISON_SLOTS * 4)
        .all(|index| matches!(record[index], 0 | 0xFF));
    let mut rejections = Vec::new();
    if ids_blank != counts_blank {
        rejections.push(TownRejectReason::GarrisonInconsistent);
        return rejections;
    }
    // Per-stack checks.
    for slot in 0..GARRISON_SLOTS {
        let id = u32::from_le_bytes(word(GARRISON_IDS + slot * 4));
        let count_bytes = word(GARRISON_COUNTS + slot * 4);
        let count = u32::from_le_bytes(count_bytes);
        if id == u32::MAX {
            // Blank ID: the count word must be all zero (h3sed first branch).
            if count_bytes != [0; 4] {
                rejections.push(TownRejectReason::GarrisonInconsistent);
            }
            continue;
        }
        if id == 0 && count_bytes == [0; 4] {
            continue; // Zero ID with zero count: valid empty form.
        }
        if count == 0 {
            // Occupied ID with a zero count: not a town stack.
            rejections.push(TownRejectReason::GarrisonInconsistent);
            continue;
        }
        if crate::creatures::name(id).is_none() {
            rejections.push(TownRejectReason::UnknownCreature);
        }
    }
    rejections
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::HERO_SCAN_START as SCAN_START;

    /// Builds a structurally valid town record with the given name and owner.
    fn craft_town(name: &[u8], owner: u8, type_: u8, x: u8, y: u8, z: u8) -> Vec<u8> {
        let mut record = vec![0u8; RECORD_FIXED + name.len()];
        record[OWNER] = owner;
        record[TYPE] = type_;
        record[X] = x;
        record[Y] = y;
        record[Z] = z;
        // Garrison: all empty (FF ids, zero counts).
        for slot in 0..GARRISON_SLOTS {
            record[GARRISON_IDS + slot * 4..GARRISON_IDS + slot * 4 + 4].fill(0xFF);
            record[GARRISON_COUNTS + slot * 4..GARRISON_COUNTS + slot * 4 + 4].fill(0);
        }
        record[GARRISONED_HERO] = 0xFF;
        record[VISITING_HERO] = 0xFF;
        record[MAGE_GUILD_LEVEL] = 3;
        record[NAME_LEN..NAME_LEN + 2].copy_from_slice(&(name.len() as u16).to_le_bytes());
        record[NAME..NAME + name.len()].copy_from_slice(name);
        record
    }

    fn embed(records: &[(usize, Vec<u8>)], hero_at: usize) -> Vec<u8> {
        let mut payload = vec![0u8; hero_at + 2000];
        payload[..5].copy_from_slice(b"H3SVG");
        payload[8..12].copy_from_slice(&44u32.to_le_bytes());
        payload[12..16].copy_from_slice(&11u32.to_le_bytes());
        for (at, record) in records {
            payload[*at..*at + record.len()].copy_from_slice(record);
        }
        payload
    }

    #[test]
    fn finds_crafted_towns_and_learns_span() {
        let a = craft_town(b"Alpha", 4, 2, 10, 20, 0);
        let b = craft_town(b"Beta", 6, 0, 30, 40, 1);
        let start = 40_000;
        let payload = embed(
            &[(start, a.clone()), (start + a.len(), b.clone())],
            start + a.len() + b.len() + 500,
        );
        let towns = locate_towns(&payload, Some(start + a.len() + b.len() + 500));
        assert_eq!(towns.len(), 2);
        assert!(towns.iter().all(|town| town.verified));
        // First span adjusted to adjacency: RECORD_FIXED + 5 / + 4.
        assert_eq!(towns[0].span, start..start + RECORD_FIXED + 5);
        assert_eq!(
            towns[1].span,
            start + RECORD_FIXED + 5..start + 2 * RECORD_FIXED + 9
        );
    }

    #[test]
    fn rejects_garrison_mismatch_as_candidate() {
        // ID blank but count set: inconsistent stack -> candidate.
        let mut bad = craft_town(b"Alpha", 4, 2, 10, 20, 0);
        bad[GARRISON_COUNTS..GARRISON_COUNTS + 4].copy_from_slice(&5u32.to_le_bytes());
        let good = craft_town(b"Beta", 6, 0, 30, 40, 1);
        let start = 40_000;
        let payload = embed(
            &[(start, bad), (start + RECORD_FIXED + 5, good)],
            start + 2 * RECORD_FIXED + 500,
        );
        let towns = locate_towns(&payload, Some(start + 2 * RECORD_FIXED + 500));
        assert_eq!(towns.len(), 2);
        assert!(!towns[0].verified);
        assert!(
            towns[0]
                .rejections
                .contains(&TownRejectReason::GarrisonInconsistent)
        );
        assert!(towns[1].verified);
    }

    #[test]
    fn legacy_names_preserve_ff_and_sixteen_bytes_in_dense_table() {
        // CP1251 "Упавшая Звезда" and "Забытая Крепость", from native saves.
        let names: [&[u8]; 4] = [
            b"Alpha",
            b"\xD3\xEF\xE0\xE2\xF8\xE0\xFF \xC7\xE2\xE5\xE7\xE4\xE0",
            b"\xC7\xE0\xE1\xFB\xF2\xE0\xFF \xCA\xF0\xE5\xEF\xEE\xF1\xF2\xFC",
            b"Omega",
        ];
        let start = 40_000;
        let mut at = start;
        let mut records = Vec::new();
        for name in names {
            let record = craft_town(name, 0, 2, 6, 7, 0);
            let len = record.len();
            records.push((at, record));
            at += len;
        }
        let payload = embed(&records, at + 25);
        let towns = locate_towns(&payload, Some(at + 25));
        assert_eq!(towns.len(), records.len());
        for (town, (offset, record)) in towns.iter().zip(records) {
            assert!(town.verified);
            assert_eq!(town.span, offset..offset + record.len());
            let len = record[NAME_LEN] as usize;
            assert_eq!(
                &payload[town.span.start + NAME..town.span.start + NAME + len],
                &record[NAME..NAME + len]
            );
        }
    }

    #[test]
    fn bounded_names_and_truncated_records_do_not_panic_or_become_verified() {
        for name in [b"Name\0bad".as_slice(), b" Leading", b"0123456789abcdefg"] {
            let start = 40_000;
            let payload = embed(&[(start, craft_town(name, 0, 2, 6, 7, 0))], start + 1000);
            assert!(locate_towns(&payload, Some(start + 1000)).is_empty());
        }
        let start = 40_000;
        let record = craft_town(b"Alpha", 0, 2, 6, 7, 0);
        let payload = embed(&[(start, record)], start + 1000);
        // Complete candidate name but incomplete record; the slice cannot
        // reach beyond either the payload or the supplied section boundary.
        for end in [start + NAME + 5, start + RECORD_FIXED_INITIAL + 1] {
            assert!(locate_towns(&payload[..end], None).is_empty());
            assert!(locate_towns(&payload, Some(end)).is_empty());
        }
    }

    #[test]
    fn keeps_unknown_creature_as_candidate() {
        // ID 210 keeps the required word shape (byte + 00 00 00) but is
        // outside the verified 44/11 creature table.
        let mut town = craft_town(b"Alpha", 4, 2, 10, 20, 0);
        town[GARRISON_IDS..GARRISON_IDS + 4].copy_from_slice(&210u32.to_le_bytes());
        town[GARRISON_COUNTS..GARRISON_COUNTS + 4].copy_from_slice(&7u32.to_le_bytes());
        let start = 40_000;
        let hero_at = start + RECORD_FIXED + 300;
        let payload = embed(&[(start, town)], hero_at);
        let towns = locate_towns(&payload, Some(hero_at));
        assert_eq!(towns.len(), 1);
        assert!(!towns[0].verified);
        assert!(
            towns[0]
                .rejections
                .contains(&TownRejectReason::UnknownCreature)
        );
    }

    #[test]
    fn ignores_false_positives_outside_section() {
        // A structurally valid record far away from the hero area must not
        // be collected (the bounded scan starts near the heroes).
        let lonely = craft_town(b"Alpha", 4, 2, 10, 20, 0);
        let hero_at = 4_600_000;
        let mut payload = embed(&[(60_000, lonely)], hero_at);
        // Two real towns right before the heroes.
        let a = craft_town(b"Beta", 6, 0, 30, 40, 1);
        let start = hero_at - 2 * (RECORD_FIXED + 4);
        payload[start..start + a.len()].copy_from_slice(&a);
        payload[start + a.len()..start + 2 * a.len()].copy_from_slice(&a);
        let towns = locate_towns(&payload, Some(hero_at));
        assert_eq!(towns.len(), 2);
    }

    #[test]
    fn empty_payload_yields_no_towns() {
        assert!(locate_towns(&[], None).is_empty());
        let mut payload = vec![0u8; 100_000];
        payload[..5].copy_from_slice(b"H3SVG");
        assert!(locate_towns(&payload, None).is_empty());
    }

    #[test]
    fn no_heroes_searches_after_scan_start() {
        let town = craft_town(b"Alpha", 4, 2, 10, 20, 0);
        let payload = embed(&[(SCAN_START + 50, town)], 4_000_000);
        let towns = locate_towns(&payload, None);
        assert_eq!(towns.len(), 1);
        assert!(towns[0].verified);
    }
}
