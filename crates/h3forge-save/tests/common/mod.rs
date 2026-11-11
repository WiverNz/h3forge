//! Shared helpers for integration tests: stored-block gzip builder and
//! synthetic save payloads. No game data is used; every fixture is built
//! in memory.

use h3forge_save::definitions::hero::*;

/// Builds a single-member gzip container with stored (uncompressed) DEFLATE
/// blocks, valid CRC-32 and ISIZE trailer.
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

/// CRC-32 via the same crate the container uses.
pub fn crc32(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes)
}

/// Builds a synthetic HotA save payload: magic, version, map name and
/// description, zero padding, then records at the given offsets.
pub fn save_payload(
    major: u32,
    minor: u32,
    name: &[u8],
    desc: &[u8],
    records: &[(usize, Vec<u8>)],
) -> Vec<u8> {
    let max_end = records
        .iter()
        .map(|(at, record)| at + record.len())
        .max()
        .unwrap_or(0);
    let mut payload = vec![0u8; max_end.max(HERO_SCAN_START_CONST) + 1024];
    payload[..5].copy_from_slice(b"H3SVG");
    payload[8..12].copy_from_slice(&major.to_le_bytes());
    payload[12..16].copy_from_slice(&minor.to_le_bytes());
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

const HERO_SCAN_START_CONST: usize = 30_000;

/// A structurally valid 1122-byte HotA hero record with known field values.
pub fn craft_record(owner: u8, name: &[u8; 8], level: u8, experience: u32, mana: u16) -> Vec<u8> {
    let mut record = vec![0u8; RECORD_LEN];
    record[OWNER] = owner;
    record[MOVEMENT_TOTAL..MOVEMENT_TOTAL + 4].copy_from_slice(&1500u32.to_le_bytes());
    record[MOVEMENT_LEFT..MOVEMENT_LEFT + 4].copy_from_slice(&1500u32.to_le_bytes());
    record[EXPERIENCE..EXPERIENCE + 4].copy_from_slice(&experience.to_le_bytes());
    record[SKILL_COUNT] = 2;
    record[SPELL_POINTS..SPELL_POINTS + 2].copy_from_slice(&mana.to_le_bytes());
    record[LEVEL] = level;
    record[NAME..NAME + 8].copy_from_slice(name);
    record[ATTACK] = 3;
    record[DEFENSE] = 7;
    record[SPELL_POWER] = 6;
    record[KNOWLEDGE] = 6;
    // One skill: id 7 (Wisdom), mastery 2, slot 1 at the HotA slot table.
    record[SKILL_LEVELS + 7] = 2;
    record[SKILL_SLOTS_HOT_A + 7] = 1;
    // Learned spells: IDs 3 and 15.
    record[SPELLS_LEARNED + 3] = 1;
    record[SPELLS_LEARNED + 15] = 1;
    record[SPELLS_AVAILABLE + 3] = 1;
    record[SPELLS_AVAILABLE + 15] = 1;
    // Equipment: all blank slots.
    for slot in 0..EQUIPMENT_SLOTS {
        let at = EQUIPMENT + slot * EQUIPMENT_SLOT_LEN;
        record[at..at + 4].fill(0xFF);
    }
    // Inventory: all blank entries.
    for entry in 0..INVENTORY_ENTRIES {
        record[INVENTORY + entry * 8..INVENTORY + entry * 8 + 8].fill(0xFF);
    }
    record
}
