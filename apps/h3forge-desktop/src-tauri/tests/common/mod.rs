//! Synthetic save fixtures for app-level integration tests: a hand-built
//! stored-block gzip container and crafted HotA hero records over a minimal
//! save header. No game data is used; every fixture is built in memory and
//! written only to the OS temporary directory.

use h3forge_save::definitions::hero::*;

/// CRC-32 (IEEE 802.3), tiny local implementation so the tests need no
/// additional dependencies.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Builds a single-member gzip container with stored (uncompressed) DEFLATE
/// blocks and a valid CRC-32/ISIZE trailer.
pub fn gzip_stored(payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1F, 0x8B, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xFF];
    let chunks: Vec<&[u8]> = payload.chunks(u16::MAX as usize).collect();
    if chunks.is_empty() {
        out.push(1);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(!0u16).to_le_bytes());
    }
    for (index, chunk) in chunks.iter().enumerate() {
        out.push((index == chunks.len() - 1) as u8);
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&crc32(payload).to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out
}

/// Builds a synthetic HotA save payload: magic, version 44/11, map name and
/// description, zero padding, then records at the given absolute offsets.
pub fn save_payload(name: &[u8], desc: &[u8], records: &[(usize, Vec<u8>)]) -> Vec<u8> {
    let max_end = records
        .iter()
        .map(|(at, record)| at + record.len())
        .max()
        .unwrap_or(0);
    let mut payload = vec![0u8; max_end.max(30_000) + 1024];
    payload[..5].copy_from_slice(b"H3SVG");
    payload[8..12].copy_from_slice(&44u32.to_le_bytes());
    payload[12..16].copy_from_slice(&11u32.to_le_bytes());
    // Header shape matching observed saves: nonzero + 3 NULs + byte, then
    // length-prefixed texts.
    payload[16] = 1;
    let mut pos = 20;
    payload[pos] = 0; // skipped byte before name length
    pos += 1;
    payload[pos..pos + 2].copy_from_slice(&(name.len() as u16).to_le_bytes());
    pos += 2;
    payload[pos..pos + name.len()].copy_from_slice(name);
    pos += name.len();
    payload[pos..pos + 2].copy_from_slice(&(desc.len() as u16).to_le_bytes());
    pos += 2;
    payload[pos..pos + desc.len()].copy_from_slice(desc);
    for (at, record) in records {
        payload[*at..*at + record.len()].copy_from_slice(record);
    }
    payload
}

/// A structurally valid 1122-byte HotA hero record. The spellbook equipment
/// word is zeroed (present) unless `blank_spellbook` is set.
pub fn craft_record(
    owner: u8,
    name: &[u8],
    level: u8,
    experience: u32,
    mana: u16,
    blank_spellbook: bool,
) -> Vec<u8> {
    let mut record = vec![0u8; RECORD_LEN];
    record[OWNER] = owner;
    record[MOVEMENT_TOTAL..MOVEMENT_TOTAL + 4].copy_from_slice(&1500u32.to_le_bytes());
    record[MOVEMENT_LEFT..MOVEMENT_LEFT + 4].copy_from_slice(&1500u32.to_le_bytes());
    record[EXPERIENCE..EXPERIENCE + 4].copy_from_slice(&experience.to_le_bytes());
    record[SKILL_COUNT] = 2;
    record[SPELL_POINTS..SPELL_POINTS + 2].copy_from_slice(&mana.to_le_bytes());
    record[LEVEL] = level;
    record[NAME..NAME + name.len()].copy_from_slice(name);
    // One skill: id 7 (Wisdom), mastery 2 (Advanced), slot 1.
    record[SKILL_LEVELS + 7] = 2;
    record[SKILL_SLOTS_HOT_A + 7] = 1;
    // Learned + available spells: IDs 3 and 15.
    record[SPELLS_LEARNED + 3] = 1;
    record[SPELLS_LEARNED + 15] = 1;
    record[SPELLS_AVAILABLE + 3] = 1;
    record[SPELLS_AVAILABLE + 15] = 1;
    // Equipment: all blank except the spellbook word.
    for slot in 0..EQUIPMENT_SLOTS {
        let at = EQUIPMENT + slot * EQUIPMENT_SLOT_LEN;
        record[at..at + 4].fill(0xFF);
        record[at + 4..at + 8].fill(0);
    }
    let book = EQUIPMENT + 17 * EQUIPMENT_SLOT_LEN;
    if blank_spellbook {
        record[book..book + 4].fill(0xFF);
    } else {
        record[book..book + 4].fill(0);
    }
    // Inventory: all blank entries.
    for entry in 0..INVENTORY_ENTRIES {
        record[INVENTORY + entry * 8..INVENTORY + entry * 8 + 8].fill(0xFF);
    }
    record
}
