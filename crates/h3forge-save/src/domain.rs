//! Domain views over located hero records.
//!
//! `HeroView` stores decoded values plus each record's payload span, so the
//! original bytes stay the single source of truth for future editing
//! milestones. Views are read-only; nothing here serializes back.

use std::ops::Range;

use crate::definitions::hero::{
    ARMY_COUNTS, ARMY_IDS, ARMY_SLOTS, ATTACK, DEFENSE, EQUIPMENT, EQUIPMENT_SLOT_LEN,
    EQUIPMENT_SLOTS, EXPERIENCE, HERO_CLASS, HERO_PORTRAIT, HERO_TYPE, INVENTORY,
    INVENTORY_ENTRIES, KNOWLEDGE, LEVEL, MOVEMENT_LEFT, MOVEMENT_TOTAL, NAME, NAME_LEN, OWNER,
    RESERVED, RESERVED_LEN, SKILL_COUNT, SKILL_LEVELS, SKILL_SLOTS_HOT_A, SPELL_FLAG_COUNT,
    SPELL_POINTS, SPELL_POWER, SPELLS_AVAILABLE, SPELLS_LEARNED,
};
use crate::definitions::level_for_experience;
use crate::definitions::names::SPELLBOOK_SLOT;
use crate::locate::{LocatedHero, RejectReason};
use crate::parse::decode_legacy_text;

/// How much trust a located record earns from the implemented checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroConfidence {
    /// Passed every implemented structural check (layout, name, equipment,
    /// owner). This is detection confidence, not a game-load guarantee.
    High,
    /// Matched the base structure but failed secondary verification; inspect
    /// `notes` before trusting any field.
    Low,
}

/// Save-owning player or neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeroOwner {
    /// Player index 0-7 (Red, Blue, Tan, Green, Orange, Purple, Teal, Pink).
    Player(u8),
    /// Neutral / unowned (0xFF); ownership is not proof of active map status.
    Neutral,
}

impl HeroOwner {
    pub fn byte(self) -> u8 {
        match self {
            HeroOwner::Player(index) => index,
            HeroOwner::Neutral => 0xFF,
        }
    }
}

/// The four primary attributes, raw stored bytes.
///
/// Raw bytes are not effective in-game stats: overflow/wrapping and artifact
/// bonuses matter (research: RESEARCH.md "Fields and offsets").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PrimaryAttributes {
    pub attack: u8,
    pub defense: u8,
    pub spell_power: u8,
    pub knowledge: u8,
}

/// One secondary skill entry, decoded like h3sed `hero/skills.py:parse`:
/// mastery and slot bytes indexed by skill ID, kept only when both are set
/// and the slot is within the stored skill count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroSkill {
    /// Skill ID: file order index 0-27 base game, 28 Interference, 29 Runes.
    pub skill_id: u8,
    /// Mastery 1-3 (Basic/Advanced/Expert).
    pub mastery: u8,
    /// One-based skill slot from the HotA slot-order table at offset 1092.
    pub slot: u8,
}

/// One army slot: creature type ID and count, both u32 little-endian.
///
/// Blank slots keep their raw encodings: the game uses both zero and 0xFFFFFFFF
/// forms, so no normalization happens here (h3sed `common/army.py` semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmySlot {
    pub creature_id: u32,
    pub count: u32,
}

/// Decoded form of one 8-byte equipment slot (record offset 382, 19 slots;
/// h3sed `hero/equipment.py:parse` semantics).
///
/// Like the backpack, an all-FF first word is empty regardless of the second
/// word; the game writes both `00 00 00 00` and `FF FF FF FF` tails for
/// occupied slots (h3sed: "game uses both 0x00 and 0xFF"), so both are
/// accepted here. War machines (IDs 3-6) have no verified names and decode
/// as [`EquipmentItem::Unknown`]; they are never editable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentItem {
    /// Blank first word; the raw entry is preserved for untouched slots.
    Empty,
    /// Verified artifact ID from `crate::artifacts` (includes the spellbook
    /// ID 0 in the spellbook slot; read-only there).
    Artifact(u32),
    /// Equipped spell scroll carrying this game spell ID (h3sed parses the
    /// full 8-byte scroll ID). Read-only in H3F-012.
    Scroll { spell_id: u8 },
    /// Occupied but unverified (war machine, unknown ID, or unrecognized
    /// trailing word); bytes never modified.
    Unknown,
}

impl EquipmentItem {
    /// Decodes one raw 8-byte slot without consulting neighbouring slots.
    pub fn decode(entry: &[u8; 8]) -> Self {
        let word1 = u32::from_le_bytes(entry[..4].try_into().expect("8-byte entry"));
        let word2 = u32::from_le_bytes(entry[4..].try_into().expect("8-byte entry"));
        if word1 == u32::MAX {
            return EquipmentItem::Empty;
        }
        if word1 == crate::artifacts::SCROLL_ARTIFACT_ID {
            return match u8::try_from(word2).ok().filter(|&spell_id| {
                crate::definitions::names::spell_name(spell_id as usize).is_some()
            }) {
                Some(spell_id) => EquipmentItem::Scroll { spell_id },
                None => EquipmentItem::Unknown,
            };
        }
        match crate::artifacts::name(word1) {
            Some(_) if word2 == u32::MAX || word2 == 0 => EquipmentItem::Artifact(word1),
            _ => EquipmentItem::Unknown,
        }
    }
}

/// Decoded form of one 8-byte backpack entry (record offset 534, 64 slots;
/// h3sed `hero/inventory.py` semantics).
///
/// H3SED treats an all-FF first word as empty regardless of the second word.
/// Zero IDs are ambiguous and remain read-only. Ordinary artifacts accept
/// only observed zero/FF trailing words; scrolls use the entire second word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpackItem {
    /// Blank first word; the raw entry is preserved for untouched slots.
    Empty,
    /// Verified artifact ID from `crate::artifacts`.
    Artifact(u32),
    /// Spell scroll carrying this game spell ID (0-69).
    Scroll { spell_id: u8 },
    /// Occupied but unverified (unknown artifact ID, ambiguous spellbook
    /// form, or malformed scroll); bytes never modified.
    Unknown,
}

impl BackpackItem {
    /// Decodes one raw 8-byte entry without consulting neighbouring slots.
    pub fn decode(entry: &[u8; 8]) -> Self {
        let word1 = u32::from_le_bytes(entry[..4].try_into().expect("8-byte entry"));
        let word2 = u32::from_le_bytes(entry[4..].try_into().expect("8-byte entry"));
        if word1 == u32::MAX {
            return BackpackItem::Empty;
        }
        if word1 == crate::artifacts::SPELLBOOK_ARTIFACT_ID {
            return BackpackItem::Unknown;
        }
        if word1 == crate::artifacts::SCROLL_ARTIFACT_ID {
            return match u8::try_from(word2).ok().filter(|&spell_id| {
                crate::definitions::names::spell_name(spell_id as usize).is_some()
            }) {
                Some(spell_id) => BackpackItem::Scroll { spell_id },
                None => BackpackItem::Unknown,
            };
        }
        match crate::artifacts::name(word1) {
            Some(_) if word2 == u32::MAX || word2 == 0 => BackpackItem::Artifact(word1),
            _ => BackpackItem::Unknown,
        }
    }
}

impl ArmySlot {
    /// h3sed common/army.py:parse treats zero count or FF ID as empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.creature_id == u32::MAX
    }

    /// Unknown type words remain read-only, including unknown zero-count records.
    pub fn is_supported(&self) -> bool {
        (self.is_empty() && matches!(self.creature_id, 0 | u32::MAX))
            || crate::creatures::name(self.creature_id).is_some()
    }
}

/// Non-fatal per-record observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeroNote {
    /// Stored level disagrees with the experience-derived level. Custom maps
    /// script levels directly, so this is surfaced, not rejected.
    LevelExperienceMismatch {
        stored_level: u8,
        derived_level: u8,
        experience: u32,
    },
    /// Another record has identical name bytes; selection must use IDs/Spans,
    /// never names.
    DuplicateName { other_id: usize },
    /// Secondary verification failure (see [`RejectReason`]).
    Rejected(RejectReason),
}

/// Stable per-record identity backing the portrait resource.
///
/// Provenance: h3svg specification (`Hero` struct, `hero_type` / `hero_class`
/// / `hero_portrait` fields; https://github.com/alexanderbelous/h3mtxt h3svg
/// branch), validated against the HotA 44/11 sample by H3F-005: field
/// alignment (destination `-1` words, movement/experience offsets), the
/// class-byte histogram matching the 24 known hero classes, unchanged
/// CP1251 roster names sitting at their expected classes, and renamed heroes
/// ("AbsoluteZero", "J. Lannister", …) keeping their type bytes. Display
/// names are never used as identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortraitIdentity {
    /// Hero type ID (enum HeroType). Stable across renames.
    pub hero_type: u8,
    /// Hero class ID (enum HeroClass), 0-23 in the sample.
    pub hero_class: u8,
    /// Portrait the game displays (enum HeroPortrait). Equals `hero_type`
    /// unless the map customized the portrait.
    pub portrait_id: u8,
}

/// Exact reasons for conservative equipment gating; unknown couplings remain blocked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentGating {
    /// The 1046-1055 reservation bytes differ from the counters implied by
    /// the equipped combination artifacts (`artifacts::reservation_counts`).
    /// Such a record carries reservation state H3Forge cannot explain, so
    /// its equipment stays read-only.
    pub reservation_mismatch: bool,
    pub reservation_bytes: [u8; 10],
    /// Wear-slot indices occupied by items whose effects are not modeled
    /// (equipped scrolls, unknown entries, artifacts outside the catalog or
    /// in a slot of another kind); equipment editing stays blocked.
    pub unmodeled_slots: Vec<u8>,
}

impl EquipmentGating {
    /// No restriction at all: every wear slot is a modeled artifact or
    /// empty, and the reservation bytes match the equipped combinations.
    pub fn is_clear(&self) -> bool {
        !self.reservation_mismatch && self.unmodeled_slots.is_empty()
    }
}

/// Read-only view of one located hero record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroView {
    /// Session-local stable identifier: locator order. Never a name.
    pub id: usize,
    /// Byte span of the fixed record in the original payload.
    pub span: Range<usize>,
    pub confidence: HeroConfidence,
    pub owner: HeroOwner,
    /// Raw name bytes (NUL padding stripped).
    pub name_raw: Vec<u8>,
    /// Best-effort display decoding; raw bytes are authoritative.
    pub name_lossy: String,
    /// Hero level as stored.
    pub level: u8,
    /// Level implied by the stored experience table.
    pub level_from_experience: u8,
    /// Experience points as stored, u32 little-endian.
    pub experience: u32,
    /// Spell points remaining, u16 little-endian.
    pub spell_points: u16,
    /// Movement points total, u32 little-endian.
    pub movement_total: u32,
    /// Movement points remaining, u32 little-endian.
    pub movement_left: u32,
    pub primary: PrimaryAttributes,
    /// Secondary skill count byte (low byte of the u32 field).
    pub skill_count: u8,
    /// Decoded secondary skills, ordered by slot.
    pub skills: Vec<HeroSkill>,
    /// Learned-spell flags (70 bytes of 0/1, spell IDs = indexes).
    pub spells_learned: Box<[u8; SPELL_FLAG_COUNT]>,
    /// Available-spell flags (70 bytes of 0/1, spell IDs = indexes).
    pub spells_available: Box<[u8; SPELL_FLAG_COUNT]>,
    /// Seven army slots in file order.
    pub army: [ArmySlot; ARMY_SLOTS],
    /// Nineteen raw 8-byte equipment slots (IDs decoded in a later milestone).
    pub equipment: [[u8; EQUIPMENT_SLOT_LEN]; EQUIPMENT_SLOTS],
    /// Decoded equipment view over `equipment`, same order and length.
    pub equipment_items: [EquipmentItem; EQUIPMENT_SLOTS],
    /// Per-record equipment editing gates (reservations, unmodeled slots).
    pub equipment_gating: EquipmentGating,
    /// Whether the spellbook equipment word is non-blank (h3sed `hero/stats.py`
    /// `parse_special` semantics: blank = four 0xFF bytes). Detection only;
    /// not proof the hero can cast.
    pub has_spellbook: bool,
    /// Identity fields backing the portrait resource; `None` when the record
    /// is too short to carry them (never expected for located HotA records).
    pub portrait: Option<PortraitIdentity>,
    /// Sixty-four raw 8-byte backpack entries.
    pub inventory: [[u8; 8]; INVENTORY_ENTRIES],
    /// Decoded backpack view over `inventory`, same order and length.
    pub backpack: Vec<BackpackItem>,
    /// Observations about this record; empty for a fully consistent hero.
    pub notes: Vec<HeroNote>,
}

/// Builds the domain view for one located record. All reads are bounded by
/// the located span, which the locator guarantees to be in `payload`.
pub fn build_hero_view(id: usize, located: &LocatedHero, payload: &[u8]) -> HeroView {
    let record = &payload[located.span.clone()];
    let owner_byte = record[OWNER];
    let owner = match owner_byte {
        0..=7 => HeroOwner::Player(owner_byte),
        _ => HeroOwner::Neutral,
    };
    let name_end = record[NAME..NAME + NAME_LEN]
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(NAME_LEN);
    let name_raw = record[NAME..NAME + name_end].to_vec();
    let name_lossy = decode_legacy_text(&name_raw);
    let experience = read_u32(record, EXPERIENCE);
    let level = record[LEVEL];
    let skill_count = record[SKILL_COUNT];

    let mut skills = Vec::new();
    for skill_id in 0..30u8 {
        // Legacy 1121-byte records carry only 29 trailing slot bytes.
        let Some(&mastery) = record.get(SKILL_LEVELS + skill_id as usize) else {
            break;
        };
        let slot = record
            .get(SKILL_SLOTS_HOT_A + skill_id as usize)
            .copied()
            .unwrap_or(0);
        if mastery == 0 || slot == 0 || slot > skill_count {
            continue; // h3sed skills parse semantics.
        }
        skills.push(HeroSkill {
            skill_id,
            mastery,
            slot,
        });
    }
    skills.sort_by_key(|skill| skill.slot);

    let mut army = [ArmySlot {
        creature_id: 0,
        count: 0,
    }; ARMY_SLOTS];
    for (index, slot) in army.iter_mut().enumerate() {
        slot.creature_id = read_u32(record, ARMY_IDS + index * 4);
        slot.count = read_u32(record, ARMY_COUNTS + index * 4);
    }

    let mut equipment = [[0u8; EQUIPMENT_SLOT_LEN]; EQUIPMENT_SLOTS];
    let mut equipment_items = [EquipmentItem::Empty; EQUIPMENT_SLOTS];
    for (index, slot) in equipment.iter_mut().enumerate() {
        *slot = copy_array(record, EQUIPMENT + index * EQUIPMENT_SLOT_LEN);
        equipment_items[index] = EquipmentItem::decode(slot);
    }
    let has_spellbook = equipment
        .get(SPELLBOOK_SLOT)
        .is_some_and(|slot| slot[..4] != [0xFF; 4]);
    // Equipment editing gates: the reservation counters must equal what the
    // equipped combination artifacts imply (verified model), and every wear
    // occupant must be a catalog artifact sitting in a slot of its own kind.
    let equipment_gating = {
        use crate::artifacts::{slot_kind, wear_slot_kind, writable};
        let unmodeled_slots = (0..EQUIPMENT_SLOTS)
            .filter(|&index| {
                let Some(kind) = wear_slot_kind(index as u8) else {
                    return false;
                };
                match equipment_items[index] {
                    EquipmentItem::Empty => false,
                    EquipmentItem::Artifact(id) => !writable(id) || slot_kind(id) != Some(kind),
                    _ => true,
                }
            })
            .map(|index| index as u8)
            .collect();
        let reservation_bytes: [u8; RESERVED_LEN] = if record.len() >= RESERVED + RESERVED_LEN {
            copy_array(record, RESERVED)
        } else {
            [0; RESERVED_LEN]
        };
        let expected =
            crate::artifacts::reservation_counts((0..EQUIPMENT_SLOTS).filter_map(|index| {
                match equipment_items[index] {
                    EquipmentItem::Artifact(id) if wear_slot_kind(index as u8).is_some() => {
                        Some(id)
                    }
                    _ => None,
                }
            }));
        EquipmentGating {
            reservation_bytes,
            reservation_mismatch: reservation_bytes != expected,
            unmodeled_slots,
        }
    };
    let mut inventory = [[0u8; 8]; INVENTORY_ENTRIES];
    let mut backpack = Vec::with_capacity(INVENTORY_ENTRIES);
    for (index, entry) in inventory.iter_mut().enumerate() {
        *entry = copy_array(record, INVENTORY + index * 8);
        backpack.push(BackpackItem::decode(entry));
    }

    let mut notes = Vec::new();
    for reason in &located.rejections {
        notes.push(HeroNote::Rejected(*reason));
    }
    let level_from_experience = level_for_experience(experience);
    if level != level_from_experience {
        notes.push(HeroNote::LevelExperienceMismatch {
            stored_level: level,
            derived_level: level_from_experience,
            experience,
        });
    }

    HeroView {
        id,
        span: located.span.clone(),
        confidence: if located.verified {
            HeroConfidence::High
        } else {
            HeroConfidence::Low
        },
        owner,
        name_raw,
        name_lossy,
        level,
        level_from_experience,
        experience,
        spell_points: read_u16(record, SPELL_POINTS),
        movement_total: read_u32(record, MOVEMENT_TOTAL),
        movement_left: read_u32(record, MOVEMENT_LEFT),
        primary: PrimaryAttributes {
            attack: record[ATTACK],
            defense: record[DEFENSE],
            spell_power: record[SPELL_POWER],
            knowledge: record[KNOWLEDGE],
        },
        skill_count,
        skills,
        spells_learned: Box::new(copy_array(record, SPELLS_LEARNED)),
        spells_available: Box::new(copy_array(record, SPELLS_AVAILABLE)),
        army,
        equipment,
        equipment_items,
        equipment_gating,
        has_spellbook,
        portrait: (record.len() > HERO_PORTRAIT).then(|| PortraitIdentity {
            hero_type: record[HERO_TYPE],
            hero_class: record[HERO_CLASS],
            portrait_id: record[HERO_PORTRAIT],
        }),
        inventory,
        backpack,
        notes,
    }
}

/// Flags duplicate names across views; both records get a note so the UI can
/// warn that name-based selection is ambiguous (research: duplicate names
/// must still select the correct span via IDs).
pub fn annotate_duplicates(heroes: &mut [HeroView]) {
    for index in 0..heroes.len() {
        for other in index + 1..heroes.len() {
            if !heroes[index].name_raw.is_empty()
                && heroes[index].name_raw == heroes[other].name_raw
            {
                let index_id = heroes[index].id;
                let other_id = heroes[other].id;
                heroes[index]
                    .notes
                    .push(HeroNote::DuplicateName { other_id });
                heroes[other]
                    .notes
                    .push(HeroNote::DuplicateName { other_id: index_id });
            }
        }
    }
}

/// How much trust a located town record earns from the implemented checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TownConfidence {
    /// Passed every implemented structural check (owner, type, coordinates,
    /// name, garrison consistency, known creatures). Detection confidence,
    /// not a game-load guarantee.
    High,
    /// Matched the base structure but failed secondary verification; inspect
    /// `notes` before trusting any field.
    Low,
}

/// Non-fatal per-town observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TownNote {
    /// Secondary verification failure (see [`TownRejectReason`]).
    Rejected(crate::towns::TownRejectReason),
    /// Another town has identical name bytes; selection must use IDs/spans.
    DuplicateName { other_id: usize },
    /// The town names a garrisoned hero type, but no save hero record (or
    /// more than one) carries that type; the garrison relationship is
    /// reported as ambiguous and the garrison stays read-only.
    GarrisonHeroAmbiguous { hero_type: u8 },
    /// The garrisoned hero record's front-matter coordinates do not match
    /// the town; the link is reported rather than trusted blindly.
    GarrisonHeroLocationMismatch { hero_id: usize },
    /// A garrison stack carries a creature ID unknown to the verified 44/11
    /// table; the stack stays read-only and preserved.
    UnknownGarrisonCreature { slot: u8 },
}

/// Read-only view of one located town record (H3F-013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TownView {
    /// Session-local stable identifier: locator order. Never a name.
    pub id: usize,
    /// Byte span of the record in the original payload.
    pub span: Range<usize>,
    pub confidence: TownConfidence,
    pub owner: HeroOwner,
    /// Town type byte 0-11 (see `definitions::towns::town_type_name`).
    pub town_type: u8,
    pub x: u8,
    pub y: u8,
    pub z: u8,
    /// Raw name bytes as stored (not NUL-terminated).
    pub name_raw: Vec<u8>,
    /// Best-effort display decoding; raw bytes are authoritative.
    pub name_lossy: String,
    /// Seven garrison slots in file order.
    pub garrison: [ArmySlot; crate::definitions::towns::GARRISON_SLOTS],
    /// Garrisoned hero type byte from the record (0xFF = none).
    pub garrisoned_hero_type: Option<u8>,
    /// Visiting hero type byte from the record (0xFF = none).
    pub visiting_hero_type: Option<u8>,
    /// Save hero record ID of the garrisoned hero, resolved by hero type.
    pub garrison_hero: Option<usize>,
    /// Save hero record ID of the visiting hero, resolved by hero type.
    pub visiting_hero: Option<usize>,
    /// Mage guild level 0-5 as stored.
    pub mage_guild_level: u8,
    /// Building masks and Mage Guild spells (H3F-022); present only for the
    /// exact HotA 44/11 layout the offsets were verified on.
    pub state: Option<crate::town_state::TownState>,
    /// Observations about this record; empty for a fully consistent town.
    pub notes: Vec<TownNote>,
}

/// Builds the domain view for one located town record. All reads are bounded
/// by the located span, which the locator guarantees to be in `payload`.
pub fn build_town_view(id: usize, located: &crate::towns::LocatedTown, payload: &[u8]) -> TownView {
    use crate::definitions::towns::*;
    let record = &payload[located.span.clone()];
    let owner_byte = record[OWNER];
    let owner = match owner_byte {
        0..=7 => HeroOwner::Player(owner_byte),
        _ => HeroOwner::Neutral,
    };
    let name_len = record[NAME_LEN] as usize;
    let name_end = (NAME + name_len).min(record.len());
    let name_raw = record[NAME..name_end].to_vec();
    let name_lossy = decode_legacy_text(&name_raw);
    let mut garrison = [ArmySlot {
        creature_id: 0,
        count: 0,
    }; GARRISON_SLOTS];
    for (index, slot) in garrison.iter_mut().enumerate() {
        slot.creature_id = read_u32(record, GARRISON_IDS + index * 4);
        slot.count = read_u32(record, GARRISON_COUNTS + index * 4);
    }
    let mut notes = Vec::new();
    for reason in &located.rejections {
        notes.push(TownNote::Rejected(*reason));
    }
    for (index, slot) in garrison.iter().enumerate() {
        if !slot.is_empty() && crate::creatures::name(slot.creature_id).is_none() {
            notes.push(TownNote::UnknownGarrisonCreature { slot: index as u8 });
        }
    }
    TownView {
        id,
        span: located.span.clone(),
        confidence: if located.verified {
            TownConfidence::High
        } else {
            TownConfidence::Low
        },
        owner,
        town_type: record[TYPE],
        x: record[X],
        y: record[Y],
        z: record[Z],
        name_raw,
        name_lossy,
        garrison,
        garrisoned_hero_type: match record[GARRISONED_HERO] {
            0xFF => None,
            hero_type => Some(hero_type),
        },
        visiting_hero_type: match record[VISITING_HERO] {
            0xFF => None,
            hero_type => Some(hero_type),
        },
        garrison_hero: None,
        visiting_hero: None,
        mage_guild_level: record[MAGE_GUILD_LEVEL],
        state: None,
        notes,
    }
}

/// Bounded read of one hero's adventure-map location from the variable
/// front matter before the fixed record block (h3sed `hero/profile.py:parse`:
/// walk back over the optional biography to its start, then x int16 at -26,
/// y int16 at -24, z byte at -22, `is_visible` at -20, which is h3sed's
/// `on_map`). PARTIAL/heuristic: used only for cross-checking and notes,
/// never for writes. Returns `None` when the walk exceeds the bound or the
/// values are implausible.
pub fn hero_map_location(payload: &[u8], span_start: usize) -> Option<(u16, u16, u8, u8)> {
    if span_start < 34 {
        return None;
    }
    let floor = span_start.saturating_sub(MAX_BIO_LOOKBACK);
    let mut ptr = span_start;
    while ptr > floor && payload[ptr - 1] != 0 {
        ptr -= 1;
    }
    if ptr <= floor + 26 || ptr < 34 {
        return None;
    }
    let x = u16::from_le_bytes([payload[ptr - 26], payload[ptr - 25]]);
    let y = u16::from_le_bytes([payload[ptr - 24], payload[ptr - 23]]);
    let z = payload[ptr - 22];
    let is_visible = payload[ptr - 20];
    if x > 252 || y > 252 || z > 1 || is_visible > 1 {
        return None;
    }
    Some((x, y, z, is_visible))
}

/// Maximum bytes the biography walk-back may cover before the location is
/// treated as unknown (bounded validation of the heuristic).
const MAX_BIO_LOOKBACK: usize = 8 * 1024;

/// Links towns to hero records after both are located: a town's
/// `garrisoned_hero`/`visiting_hero` type bytes identify the hero record by
/// its immutable `hero_type` byte (offset 10), never by name or record order.
/// Sample-verified: 17 garrisoned and 1 visiting town each resolve to exactly
/// one record whose front-matter coordinates equal the town. Unresolvable or
/// ambiguous links stay unlinked with an explanatory note.
pub fn annotate_town_heroes(towns: &mut [TownView], heroes: &[HeroView], payload: &[u8]) {
    for town in towns.iter_mut() {
        for (type_byte, is_garrison) in [
            (town.garrisoned_hero_type, true),
            (town.visiting_hero_type, false),
        ] {
            let Some(hero_type) = type_byte else {
                continue;
            };
            let matches: Vec<usize> = heroes
                .iter()
                .filter(|hero| {
                    hero.portrait
                        .is_some_and(|identity| identity.hero_type == hero_type)
                })
                .map(|hero| hero.id)
                .collect();
            match matches.as_slice() {
                [hero_id] => {
                    if is_garrison {
                        town.garrison_hero = Some(*hero_id);
                    } else {
                        town.visiting_hero = Some(*hero_id);
                    }
                    if let Some((x, y, z, _)) = hero_map_location(payload, heroes[*hero_id].span.start)
                        && (x as u8, y as u8, z) != (town.x, town.y, town.z)
                    {
                        town.notes.push(TownNote::GarrisonHeroLocationMismatch {
                            hero_id: *hero_id,
                        });
                    }
                }
                _ => {
                    if is_garrison {
                        town.notes.push(TownNote::GarrisonHeroAmbiguous { hero_type });
                    }
                }
            }
        }
    }
}

/// Flags duplicate town names across views, like [`annotate_duplicates`]
/// does for heroes.
pub fn annotate_town_duplicates(towns: &mut [TownView]) {
    for index in 0..towns.len() {
        for other in index + 1..towns.len() {
            if !towns[index].name_raw.is_empty() && towns[index].name_raw == towns[other].name_raw
            {
                let index_id = towns[index].id;
                let other_id = towns[other].id;
                towns[index]
                    .notes
                    .push(TownNote::DuplicateName { other_id });
                towns[other]
                    .notes
                    .push(TownNote::DuplicateName { other_id: index_id });
            }
        }
    }
}

impl TownView {
    /// Whether the garrison may be edited at all: a verified record whose
    /// garrison is not owned by a garrisoned hero.
    pub fn garrison_editable(&self) -> bool {
        self.confidence == TownConfidence::High && self.garrison_hero.is_none()
    }
}

/// Bounded little-endian reads; located records always contain these
/// offsets, and short slices yield zero instead of panicking.
fn read_u16(record: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        record
            .get(offset..offset + 2)
            .and_then(|bytes| bytes.try_into().ok())
            .unwrap_or([0, 0]),
    )
}

fn read_u32(record: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        record
            .get(offset..offset + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .unwrap_or([0, 0, 0, 0]),
    )
}

fn copy_array<const N: usize>(record: &[u8], offset: usize) -> [u8; N] {
    record
        .get(offset..offset + N)
        .and_then(|bytes| bytes.try_into().ok())
        .unwrap_or([0u8; N])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::hero::EQUIPMENT;

    #[test]
    fn owner_mapping() {
        assert_eq!(HeroOwner::Player(4).byte(), 4);
        assert_eq!(HeroOwner::Neutral.byte(), 0xFF);
    }

    #[test]
    fn spellbook_word_semantics() {
        // h3sed `parse_special`: blank iff all four word bytes are 0xFF; any
        // other content (including 00 00 00 00) counts as carrying a spellbook.
        let mut record = vec![0u8; crate::definitions::hero::RECORD_LEN];
        for slot in 0..EQUIPMENT_SLOTS {
            let at = EQUIPMENT + slot * EQUIPMENT_SLOT_LEN;
            record[at..at + 4].fill(0xFF);
        }
        let located = LocatedHero {
            span: 0..record.len(),
            verified: true,
            rejections: Vec::new(),
        };
        let view = build_hero_view(0, &located, &record);
        assert!(!view.has_spellbook);

        let at = EQUIPMENT + SPELLBOOK_SLOT * EQUIPMENT_SLOT_LEN;
        record[at..at + 4].fill(0);
        let view = build_hero_view(0, &located, &record);
        assert!(view.has_spellbook);
    }
}

#[cfg(test)]
mod name_table_tests {
    use crate::definitions::names::{mastery_name, skill_name, spell_name};

    #[test]
    fn ported_tables_cover_expected_ranges() {
        assert_eq!(skill_name(0), Some("Pathfinding"));
        assert_eq!(skill_name(28), Some("Interference"));
        assert_eq!(skill_name(29), Some("Runes"));
        assert_eq!(skill_name(30), None);
        assert_eq!(mastery_name(1), Some("Basic"));
        assert_eq!(mastery_name(3), Some("Expert"));
        assert_eq!(mastery_name(0), None);
        assert_eq!(mastery_name(4), None);
    }

    #[test]
    fn spell_table_has_all_seventy_spells_in_game_id_order() {
        use crate::definitions::hero::SPELL_FLAG_COUNT;
        assert_eq!(crate::definitions::names::SPELL_NAMES.len(), 70);
        assert_eq!(spell_name(0), Some("Summon Boat"));
        assert_eq!(spell_name(2), Some("Visions"));
        assert_eq!(spell_name(28), Some("Air Shield"));
        assert_eq!(spell_name(68), Some("Summon Water Elemental"));
        assert_eq!(spell_name(69), Some("Summon Air Elemental"));
        assert_eq!(spell_name(70), None);
        for id in 0..SPELL_FLAG_COUNT {
            assert!(spell_name(id).is_some());
        }
    }
}

impl HeroView {
    /// Conservative diagnostic: opaque effects cannot be proven independent.
    pub fn equipment_block_reason(&self) -> Option<String> {
        let mut causes: Vec<String> = self
            .equipment_gating
            .unmodeled_slots
            .iter()
            .map(|&slot| {
                let item = match self.equipment_items[slot as usize] {
                    EquipmentItem::Artifact(id) => format!(
                        "{} (ID {id})",
                        crate::artifacts::name(id).unwrap_or("Unknown artifact")
                    ),
                    EquipmentItem::Scroll { spell_id } => {
                        format!("equipped spell scroll (spell {spell_id})")
                    }
                    _ => {
                        let raw = self.equipment[slot as usize];
                        let id = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
                        format!("Unknown artifact (ID {id}); unrecognized bytes {raw:02X?}")
                    }
                };
                format!(
                    "{} (slot {}): {item}",
                    crate::definitions::names::EQUIPMENT_SLOT_LABELS[slot as usize],
                    slot + 1
                )
            })
            .collect();
        if self.equipment_gating.reservation_mismatch {
            causes.push(format!(
                "combination reservation bytes {:02X?} at record offset {} that do not match                  the equipped combination artifacts",
                self.equipment_gating.reservation_bytes,
                crate::definitions::hero::RESERVED
            ));
        }
        (!causes.is_empty()).then(|| format!("Equipment editing is blocked by {}. Their stat/spell effects or slot reservations are not modeled; all original items and bytes are preserved.", causes.join("; ")))
    }
}
