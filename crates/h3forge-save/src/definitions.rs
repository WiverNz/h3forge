//! Format constants ported from h3sed (`metadata.py`, `version/hota.py`).
//!
//! Provenance: h3sed 4.1, `D:/Projects/h3sed_4.1/src/h3sed/metadata.py` and
//! `version/hota.py` (MIT, copyright 2020 Erki Suurjaak; see licenses/h3sed-MIT.txt).
//! Values were transcribed, not re-derived; adaptation notes appear at each site.
//! H3Forge reads structures natively and never monkey-patches gzip validation.

/// Minimum uncompressed byte offset where hero scanning starts.
///
/// h3sed `Savefile.populate_heroes` starts at 30_000 to skip campaign
/// carry-over hero structs stored ahead of the working hero set.
pub const HERO_SCAN_START: usize = 30_000;

/// Once a hero record has been accepted, h3sed continues searching only within
/// this many bytes after the previous match; hero structs are densely packed,
/// so a gap beyond this ends the heroes section.
pub const HERO_SCAN_WINDOW: usize = 5_000;

/// Heroes III save magics, as checked by h3sed `Savefile.RGX_MAGIC`
/// (with the Chronicles adaptation adding `HCHRONSVG`).
pub const MAGIC_SAVES: &[&[u8]] = &[b"H3SVG", b"H3SVC", b"HCHRONSVG"];

/// Payload offsets of the version identifiers (h3sed `metadata.BYTE_POSITIONS`).
pub const VERSION_MAJOR_OFFSET: usize = 8;
pub const VERSION_MINOR_OFFSET: usize = 12;

/// HotA hero record layout, offsets relative to the matched record start
/// (h3sed `metadata.HERO_BYTE_POSITIONS` + `version/hota.HERO_REGEX`).
pub mod hero {
    /// Owner byte: player 0-7 or 255 neutral.
    pub const OWNER: usize = 0;
    /// Hero type identity, enum HeroType (h3svg `Hero.hero_type`).
    pub const HERO_TYPE: usize = 10;
    /// Hero class, enum HeroClass (h3svg `Hero.hero_class`).
    pub const HERO_CLASS: usize = 11;
    /// Portrait the game displays, enum HeroPortrait (h3svg `Hero.hero_portrait`).
    pub const HERO_PORTRAIT: usize = 12;
    /// Movement points total, u32 little-endian.
    pub const MOVEMENT_TOTAL: usize = 31;
    /// Movement points remaining, u32 little-endian.
    pub const MOVEMENT_LEFT: usize = 35;
    /// Experience, u32 little-endian.
    pub const EXPERIENCE: usize = 39;
    /// Secondary skill count, low byte (remaining three bytes constrained to zero).
    pub const SKILL_COUNT: usize = 43;
    /// Spell points remaining, u16 little-endian.
    pub const SPELL_POINTS: usize = 47;
    /// Hero level, u8.
    pub const LEVEL: usize = 49;
    /// Seven creature IDs, u32 little-endian each.
    pub const ARMY_IDS: usize = 113;
    /// Seven creature counts, u32 little-endian each.
    pub const ARMY_COUNTS: usize = 141;
    /// Hero name, 13 bytes NUL-padded.
    pub const NAME: usize = 169;
    /// Name field length in bytes.
    pub const NAME_LEN: usize = 13;
    /// Skill mastery bytes (30 in HotA 44/10+), values 0-3.
    pub const SKILL_LEVELS: usize = 182;
    /// Legacy skill slot bytes (unused), 26 bytes in HotA.
    pub const SKILL_SLOTS_LEGACY: usize = 212;
    /// Primary attributes, one byte each in file order.
    pub const ATTACK: usize = 238;
    pub const DEFENSE: usize = 239;
    pub const SPELL_POWER: usize = 240;
    pub const KNOWLEDGE: usize = 241;
    /// Learned-spell flags, 70 bytes of 0/1.
    pub const SPELLS_LEARNED: usize = 242;
    /// Available-spell flags, 70 bytes of 0/1.
    pub const SPELLS_AVAILABLE: usize = 312;
    /// Nineteen 8-byte equipment slots.
    pub const EQUIPMENT: usize = 382;
    /// Equipment slot byte length.
    pub const EQUIPMENT_SLOT_LEN: usize = 8;
    /// Equipment slot count (HotA and SoD-style layouts).
    pub const EQUIPMENT_SLOTS: usize = 19;
    /// Sixty-four 8-byte backpack entries.
    pub const INVENTORY: usize = 534;
    /// HotA skill slot order bytes (29 in legacy HotA, 30 in 44/10+).
    pub const SKILL_SLOTS_HOT_A: usize = 1092;

    /// Record length with a 30-byte trailing HotA skill-slot run (44/10+).
    pub const RECORD_LEN: usize = 1122;
    /// Record length with a 29-byte trailing HotA skill-slot run.
    pub const RECORD_LEN_LEGACY: usize = 1121;
    /// Maximum stored skill count accepted by the locator (h3sed constrains
    /// the low byte to 0x00-0x1C; 28 base + Interference + Runes = 30 skills
    /// exist in 44/10+, but the stored count field stays within 0x1C).
    pub const SKILL_COUNT_MAX: u8 = 0x1C;
    /// Maximum skill mastery value.
    pub const SKILL_LEVEL_MAX: u8 = 3;
    /// Maximum spell flag value.
    pub const SPELL_FLAG_MAX: u8 = 1;
    /// Maximum skill slot index value.
    pub const SKILL_SLOT_MAX: u8 = 0x1C;
    /// Army slot count.
    pub const ARMY_SLOTS: usize = 7;
    /// Spell flag array length.
    pub const SPELL_FLAG_COUNT: usize = 70;
    /// Backpack entry count.
    pub const INVENTORY_ENTRIES: usize = 64;
    /// Combination-artifact slot counters after the backpack (h3sed
    /// `metadata.HERO_BYTE_POSITIONS["reserved"]`, 1046-1055; the per-slot
    /// positions are 1047-1055, one counter per wear-slot kind). Each worn
    /// combination artifact adds one per extra slot it takes. Equipment
    /// edits rewrite a counter only when its modeled value changes; bytes
    /// the model cannot explain disable equipment editing for the record.
    pub const RESERVED: usize = 1046;
    /// Length of the combination-artifact reservation region.
    pub const RESERVED_LEN: usize = 10;
}

/// Display-identification tables ported from h3sed (`metadata.py` and
/// `version/hota.py`, MIT, copyright 2020 Erki Suurjaak; see
/// licenses/h3sed-MIT.txt). These are read-only labels for parsed IDs; they
/// never gate parsing and unknown IDs stay unknown instead of being guessed.
pub mod names {
    /// Hero skills in file-order skill ID (h3sed `metadata.SKILLS`, 28 base
    /// game entries, plus h3sed `version/hota.SKILLS`: Interference 28 and
    /// Runes 29 for HotA 44/10+ records).
    pub const SKILL_NAMES: [&str; 30] = [
        "Pathfinding",
        "Archery",
        "Logistics",
        "Scouting",
        "Diplomacy",
        "Navigation",
        "Leadership",
        "Wisdom",
        "Mysticism",
        "Luck",
        "Ballistics",
        "Eagle Eye",
        "Necromancy",
        "Estates",
        "Fire Magic",
        "Air Magic",
        "Water Magic",
        "Earth Magic",
        "Scholar",
        "Tactics",
        "Artillery",
        "Learning",
        "Offense",
        "Armorer",
        "Intelligence",
        "Sorcery",
        "Resistance",
        "First Aid",
        "Interference",
        "Runes",
    ];

    /// Skill mastery names 1-3 (h3sed `metadata.SKILL_LEVELS`).
    pub const SKILL_MASTERY_NAMES: [&str; 3] = ["Basic", "Advanced", "Expert"];

    /// Spell names indexed by game spell ID (h3sed `metadata.IDS` spell
    /// entries, 0x00-0x45, verified identical to the h3mtxt `SpellType`
    /// enum). The save's 70-byte learned/available flag arrays use exactly
    /// these indexes - this table is the ID order, NOT h3sed's alphabetical
    /// `metadata.SPELLS` display list (H3F-006: using that list here mislabels
    /// every spell).
    pub const SPELL_NAMES: [&str; 70] = [
        "Summon Boat",
        "Scuttle Boat",
        "Visions",
        "View Earth",
        "Disguise",
        "View Air",
        "Fly",
        "Water Walk",
        "Dimension Door",
        "Town Portal",
        "Quicksand",
        "Land Mine",
        "Force Field",
        "Fire Wall",
        "Earthquake",
        "Magic Arrow",
        "Ice Bolt",
        "Lightning Bolt",
        "Implosion",
        "Chain Lightning",
        "Frost Ring",
        "Fireball",
        "Inferno",
        "Meteor Shower",
        "Death Ripple",
        "Destroy Undead",
        "Armageddon",
        "Shield",
        "Air Shield",
        "Fire Shield",
        "Protection from Air",
        "Protection from Fire",
        "Protection from Water",
        "Protection from Earth",
        "Anti-Magic",
        "Dispel",
        "Magic Mirror",
        "Cure",
        "Resurrection",
        "Animate Dead",
        "Sacrifice",
        "Bless",
        "Curse",
        "Bloodlust",
        "Precision",
        "Weakness",
        "Stone Skin",
        "Disrupting Ray",
        "Prayer",
        "Mirth",
        "Sorrow",
        "Fortune",
        "Misfortune",
        "Haste",
        "Slow",
        "Slayer",
        "Frenzy",
        "Titan's Lightning Bolt",
        "Counterstrike",
        "Berserk",
        "Hypnotize",
        "Forgetfulness",
        "Blind",
        "Teleport",
        "Remove Obstacle",
        "Clone",
        "Summon Fire Elemental",
        "Summon Earth Elemental",
        "Summon Water Elemental",
        "Summon Air Elemental",
    ];

    /// Player faction labels by owner index (h3sed `metadata.PLAYER_FACTIONS`).
    pub const PLAYER_COLORS: [&str; 8] = [
        "Red", "Blue", "Tan", "Green", "Orange", "Purple", "Teal", "Pink",
    ];

    /// Equipment slot names in file order (h3sed `metadata.HERO_BYTE_POSITIONS`
    /// equipment entries, offsets 382-526). Only labels the fixed positions;
    /// artifact IDs inside the slots stay raw until a later milestone.
    pub const EQUIPMENT_SLOT_NAMES: [&str; 19] = [
        "helm",
        "cloak",
        "neck",
        "weapon",
        "shield",
        "armor",
        "lefthand",
        "righthand",
        "feet",
        "side1",
        "side2",
        "side3",
        "side4",
        "ballista",
        "ammo",
        "tent",
        "catapult",
        "spellbook",
        "side5",
    ];

    /// Equipment slot index holding the spellbook word (h3sed offset 518).
    pub const SPELLBOOK_SLOT: usize = 17;

    /// Display labels for the 19 equipment slots, in file order.
    pub const EQUIPMENT_SLOT_LABELS: [&str; 19] = [
        "Helm",
        "Cloak",
        "Neck",
        "Weapon",
        "Shield",
        "Armor",
        "Left hand",
        "Right hand",
        "Feet",
        "Side 1",
        "Side 2",
        "Side 3",
        "Side 4",
        "Ballista",
        "Ammo cart",
        "First aid tent",
        "Catapult",
        "Spellbook",
        "Side 5",
    ];

    /// Returns the spell name for a game spell ID 0-69 (all 70 spells are
    /// named; IDs index the learned/available flag arrays directly).
    pub fn spell_name(spell_id: usize) -> Option<&'static str> {
        SPELL_NAMES.get(spell_id).copied()
    }

    /// Returns the skill name for a skill ID 0-29.
    pub fn skill_name(skill_id: usize) -> Option<&'static str> {
        SKILL_NAMES.get(skill_id).copied()
    }

    /// Mastery name for a stored mastery value 1-3; `None` otherwise.
    pub fn mastery_name(mastery: u8) -> Option<&'static str> {
        mastery
            .checked_sub(1)
            .and_then(|index| SKILL_MASTERY_NAMES.get(index as usize).copied())
    }
}

/// HotA town record layout (H3F-013), offsets relative to the record start.
///
/// Provenance: h3sed `metadata.TOWN_REGEX`/`TOWN_BYTE_POSITIONS` and the
/// h3svg specification's `Town` struct (branch h3svg), re-validated field by
/// field against the HotA 44/11 sample: the learned record length is
/// `RECORD_FIXED + name_len` (dense adjacency over 37 sample towns), the
/// `garrisoned_hero`/`visiting_hero` bytes at 65/66 match exactly one hero
/// record by hero type on the sample, and coordinates equal the map town
/// entrance. Offsets beyond `NAME` (buildings, spells, HotA extras) stay
/// UNKNOWN and are never interpreted.
pub mod towns {
    /// Owner byte: player 0-7 or 255 neutral (h3sed `faction`).
    pub const OWNER: usize = 0;
    /// Town type 0-11: Castle, Rampart, Tower, Inferno, Necropolis, Dungeon,
    /// Stronghold, Fortress, Conflux, Cove, Factory, Bulwark (h3sed
    /// `TOWN_TYPES`).
    pub const TYPE: usize = 3;
    pub const X: usize = 4;
    pub const Y: usize = 5;
    pub const Z: usize = 6;
    /// Seven garrison creature IDs, u32 little-endian each.
    pub const GARRISON_IDS: usize = 9;
    /// Seven garrison counts, u32 little-endian each.
    pub const GARRISON_COUNTS: usize = 37;
    /// Garrison slot count.
    pub const GARRISON_SLOTS: usize = 7;
    /// Garrisoned hero (enum HeroType) or 0xFF (h3svg `Town.garrisoned_hero`).
    pub const GARRISONED_HERO: usize = 65;
    /// Visiting hero (enum HeroType) or 0xFF (h3svg `Town.visiting_hero`).
    pub const VISITING_HERO: usize = 66;
    /// Mage guild level 0-5 (h3svg `Town.mage_guild_level`); read-only.
    pub const MAGE_GUILD_LEVEL: usize = 67;
    /// Name length, u16 little-endian (verified bounded range 1-16).
    pub const NAME_LEN: usize = 69;
    /// Name bytes (not NUL-terminated).
    pub const NAME: usize = 71;
    /// Verified name-length bound. Native Russian HotA names exceed h3sed's
    /// inherited 14-byte limit: "Забытая Крепость" is 16 CP1251 bytes.
    /// This is a conservative reader limit, not a claimed format maximum.
    pub const NAME_MAX: usize = 16;
    /// Maximum town type byte (HotA through Bulwark).
    pub const TYPE_MAX: u8 = 11;
    /// Maximum coordinate byte (h3sed TOWN_REGEX).
    pub const COORD_MAX: u8 = 0xFC;
    /// Record length excluding the name, learned from dense adjacency on the
    /// 44/11 sample (h3sed adjusts `BYTES_MINLEN` the same way).
    pub const RECORD_FIXED: usize = 567;
    /// h3sed's generic initial `town.bytelen` minimum, before adjacency
    /// learning; only the first span of a section can carry it.
    pub const RECORD_FIXED_INITIAL: usize = 546;
    /// Longest supported record under the verified name-length bound.
    pub const RECORD_MAX: usize = RECORD_FIXED + NAME_MAX;
    /// Upper bound for the town count search heuristic (h3sed
    /// `DATA_RANGES["town.count"]`).
    pub const TOWNS_MAX: usize = 48;

    /// Town type labels indexed by type byte (h3sed `version/hota.TOWN_TYPES`).
    pub const TOWN_TYPE_NAMES: [&str; 12] = [
        "Castle",
        "Rampart",
        "Tower",
        "Inferno",
        "Necropolis",
        "Dungeon",
        "Stronghold",
        "Fortress",
        "Conflux",
        "Cove",
        "Factory",
        "Bulwark",
    ];

    /// Town type name for a type byte, when in range.
    pub fn town_type_name(type_: u8) -> Option<&'static str> {
        TOWN_TYPE_NAMES.get(type_ as usize).copied()
    }
}

/// Minimum experience required for each hero level, index = level (h3sed
/// `metadata.EXPERIENCE_LEVELS`, levels 1-75). Used only as a diagnostic aid
/// when the stored level disagrees with stored experience.
pub const EXPERIENCE_LEVEL_MIN: [u64; 76] = [
    0,
    0,
    1_000,
    2_000,
    3_200,
    4_600,
    6_200,
    8_000,
    10_000,
    12_200,
    14_700,
    17_500,
    20_600,
    24_320,
    28_784,
    34_140,
    40_567,
    48_279,
    57_533,
    68_637,
    81_961,
    97_949,
    117_134,
    140_156,
    167_782,
    200_933,
    240_714,
    288_451,
    345_735,
    414_475,
    496_963,
    595_948,
    714_730,
    857_268,
    1_028_313,
    1_233_567,
    1_479_871,
    1_775_435,
    2_130_111,
    2_555_722,
    3_066_455,
    3_679_334,
    4_414_788,
    5_297_332,
    6_356_384,
    7_627_246,
    9_152_280,
    10_982_320,
    13_178_368,
    15_813_625,
    18_975_933,
    22_770_702,
    27_324_424,
    32_788_890,
    39_346_249,
    47_215_079,
    56_657_675,
    67_988_790,
    81_586_128,
    97_902_933,
    117_483_099,
    140_979_298,
    169_174_736,
    203_009_261,
    243_610_691,
    292_332_407,
    350_798_466,
    420_957_736,
    505_148_860,
    606_178_208,
    727_413_425,
    872_895_685,
    1_047_474_397,
    1_256_968_851,
    1_508_362_195,
    1_810_034_207,
];

/// Returns the highest level whose minimum experience does not exceed `experience`.
pub fn level_for_experience(experience: u32) -> u8 {
    let exp = experience as u64;
    let mut level = 1u8;
    for (index, &minimum) in EXPERIENCE_LEVEL_MIN.iter().enumerate().skip(2) {
        if exp < minimum {
            break;
        }
        level = index as u8;
    }
    level
}
