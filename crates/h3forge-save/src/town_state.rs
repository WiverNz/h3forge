//! Town building masks and Mage Guild state of HotA 44/11 town records
//! (H3F-022).
//!
//! Field names follow the h3svg `Town` struct (h3mtxt branch `h3svg`,
//! `doc/h3svg_specification.MD`; building bit numbering from its
//! `TownBuildingTypeH3SVG` enum). Every offset below was located and
//! cross-checked on the 37 towns of the HotA 44/11 sample; see
//! `docs/formats/SAVEGAME_FORMAT.md` section D.2 for the evidence. Offsets
//! are relative to the end of the town name (`NAME + name_len`).
//!
//! Three 64-bit building masks are stored back to back:
//!
//! * `visible` - the graphic actually drawn for each upgrade chain (Mage
//!   Guild 5 hides 1-4, Castle hides Fort and Citadel, an upgraded dwelling
//!   hides its base, ...). The h3svg specification leaves these bytes unknown.
//! * `built` - the logical set, with every chain member set.
//! * `allowed` - what may be constructed here (faction limits combined with
//!   the map's prohibitions).
//!
//! Only the upgrade chains shared by every faction are modeled. Bits hidden
//! by a faction-specific graphic replacement (horde buildings, a few special
//! buildings) are preserved byte for byte and stay locked.

use crate::definitions::towns::{NAME, NAME_LEN};
use crate::error::EditError;

/// Offsets relative to the end of the town name.
pub mod layout {
    /// Spell count per Mage Guild level, five bytes; zero for unbuilt levels.
    pub const GUILD_COUNTS: usize = 84;
    /// Visible-graphic building mask, u64 little-endian (44 bits used).
    pub const VISIBLE: usize = 89;
    /// Logical built mask, u64 little-endian (h3svg `built` + reserved).
    pub const BUILT: usize = 97;
    /// Constructible mask, u64 little-endian (h3svg `can_be_built`).
    pub const ALLOWED: usize = 105;
    /// Thirty i32 spell slots: six per guild level (h3svg `spells`).
    pub const SPELLS: usize = 113;
    /// Banned-spell bitmask, one bit per spell ID (h3svg `banned_spells`).
    pub const BANNED: usize = 233;
    pub const BANNED_LEN: usize = 9;
    /// First byte past the fields parsed here.
    pub const END: usize = BANNED + BANNED_LEN;
}

/// Building bit numbers (h3svg `TownBuildingTypeH3SVG`).
pub mod building {
    pub const MAGE_GUILD_1: u8 = 0;
    pub const MAGE_GUILD_5: u8 = 4;
    pub const TAVERN: u8 = 5;
    pub const SHIPYARD: u8 = 6;
    pub const FORT: u8 = 7;
    pub const CITADEL: u8 = 8;
    pub const CASTLE: u8 = 9;
    pub const VILLAGE_HALL: u8 = 10;
    pub const TOWN_HALL: u8 = 11;
    pub const CITY_HALL: u8 = 12;
    pub const CAPITOL: u8 = 13;
    pub const MARKETPLACE: u8 = 14;
    pub const RESOURCE_SILO: u8 = 15;
    pub const BLACKSMITH: u8 = 16;
    pub const SPECIAL_17: u8 = 17;
    pub const HORDE_1: u8 = 18;
    pub const HORDE_1_UPGRADED: u8 = 19;
    pub const SHIP: u8 = 20;
    pub const SPECIAL_21: u8 = 21;
    pub const SPECIAL_22: u8 = 22;
    pub const SPECIAL_23: u8 = 23;
    pub const HORDE_2: u8 = 24;
    pub const HORDE_2_UPGRADED: u8 = 25;
    pub const GRAIL: u8 = 26;
    pub const EXTRA_27: u8 = 27;
    pub const EXTRA_29: u8 = 29;
    pub const DWELLING_1: u8 = 30;
    pub const DWELLING_7: u8 = 36;
    pub const DWELLING_1_UPGRADED: u8 = 37;
    pub const DWELLING_7_UPGRADED: u8 = 43;
    /// Number of defined building bits.
    pub const COUNT: u8 = 44;
}

const fn bit(building: u8) -> u64 {
    1u64 << building
}

const HORDE_BITS: u64 = bit(building::HORDE_1)
    | bit(building::HORDE_1_UPGRADED)
    | bit(building::HORDE_2)
    | bit(building::HORDE_2_UPGRADED);

/// Record offsets of the boat tile (h3svg `generated_boat_x/y`).
const BOAT_X: usize = 7;
const BOAT_Y: usize = 8;

/// Mage Guild levels.
pub const GUILD_LEVELS: usize = 5;
/// Stored spell slots per guild level.
pub const GUILD_SLOTS: usize = 6;
/// Tower town type; its Library (special building 22) adds one spell per
/// guild level.
const TOWER: u8 = 2;

/// Guild level (1-5) of each spell ID 0-69. Standard Heroes III levels; the
/// sample test checks every stored guild spell against this table.
pub const SPELL_LEVELS: [u8; 70] = [
    1, 2, 2, 1, 2, 1, 5, 4, 5, 4, // 0-9 adventure spells
    2, 3, 3, 2, 3, 1, 2, 2, 5, 4, // 10-19
    3, 3, 4, 4, 2, 3, 4, 1, 3, 4, // 20-29
    2, 1, 1, 3, 3, 1, 5, 1, 4, 3, // 30-39
    5, 1, 1, 1, 2, 2, 1, 2, 4, 3, // 40-49
    4, 2, 3, 1, 1, 4, 4, 5, 4, 4, // 50-59
    3, 3, 2, 3, 2, 4, 5, 5, 5, 5, // 60-69
];

/// Guild level of a spell ID, when it is a guild spell.
pub fn spell_level(spell_id: u8) -> Option<u8> {
    SPELL_LEVELS.get(spell_id as usize).copied()
}

/// Special, horde and grail building names by town type and building bit.
///
/// Slot assignment comes from the game's own tables: `BldgSpec.txt` for the
/// nine original factions and the `casspcost10`-`12` tables of `HotA.dat`
/// for Cove, Factory and Bulwark (eleven entries per town: bits 17-26, then
/// the Resource Silo; the fourth entry names the shipyard slot). English
/// spellings follow the original game and the HotA wiki
/// (heroes.thelazy.net). Slots a faction does not use have no entry.
const SPECIAL_NAMES: [&[(u8, &str)]; 12] = [
    // Castle
    &[
        (17, "Lighthouse"),
        (18, "Griffin Bastion"),
        (21, "Stables"),
        (22, "Brotherhood of the Sword"),
        (26, "Colossus"),
    ],
    // Rampart
    &[
        (17, "Mystic Pond"),
        (18, "Miners' Guild"),
        (21, "Fountain of Fortune"),
        (22, "Dwarven Treasury"),
        (24, "Dendroid Saplings"),
        (26, "Spirit Guardian"),
    ],
    // Tower
    &[
        (17, "Artifact Merchants"),
        (18, "Sculptor's Wings"),
        (21, "Lookout Tower"),
        (22, "Library"),
        (23, "Wall of Knowledge"),
        (26, "Skyship"),
    ],
    // Inferno
    &[
        (18, "Birthing Pools"),
        (21, "Brimstone Stormclouds"),
        (22, "Castle Gate"),
        (23, "Order of Fire"),
        (24, "Cages"),
        (26, "Deity of Fire"),
    ],
    // Necropolis
    &[
        (17, "Cover of Darkness"),
        (18, "Unearthed Graves"),
        (21, "Necromancy Amplifier"),
        (22, "Skeleton Transformer"),
        (26, "Soul Prison"),
    ],
    // Dungeon
    &[
        (17, "Artifact Merchants"),
        (18, "Mushroom Rings"),
        (21, "Mana Vortex"),
        (22, "Portal of Summoning"),
        (23, "Battle Scholar Academy"),
        (26, "Guardian of Earth"),
    ],
    // Stronghold
    &[
        (17, "Escape Tunnel"),
        (18, "Mess Hall"),
        (21, "Freelancer's Guild"),
        (22, "Ballista Yard"),
        (23, "Hall of Valhalla"),
        (26, "Warlords' Monument"),
    ],
    // Fortress
    &[
        (17, "Cage of Warlords"),
        (18, "Captain's Quarters"),
        (21, "Glyphs of Fear"),
        (22, "Blood Obelisk"),
        (26, "Carnivorous Plant"),
    ],
    // Conflux
    &[
        (17, "Artifact Merchants"),
        (18, "Garden of Life"),
        (21, "Magic University"),
        (26, "Aurora Borealis"),
    ],
    // Cove
    &[
        (18, "Pub"),
        (21, "Gunpowder Warehouse"),
        (22, "Thieves' Guild"),
        (23, "Grotto"),
        (24, "Roost"),
        (26, "Lodestar"),
    ],
    // Factory: the shipyard slot (6) is the Mana Generator; 22/23 are the
    // Dreadnought dwelling and its upgrade.
    &[
        (6, "Mana Generator"),
        (17, "Artifact Merchants"),
        (18, "Pen"),
        (21, "Bank"),
        (22, "Gantry"),
        (23, "Upg. Gantry"),
        (26, "Lightning Rod"),
    ],
    // Bulwark
    &[
        (17, "Fair"),
        (18, "Hay Bins"),
        (21, "Sieidi of the Runes"),
        (22, "Altar of the Runes"),
        (26, "Spirits of the Forebears"),
    ],
];

/// Creature dwelling names by town type, levels 1-7 (`Dwelling.txt` and the
/// `casdwcost10`-`12` tables of `HotA.dat` give the order; upgraded
/// dwellings carry the same name with an "Upg." prefix in the game).
const DWELLING_NAMES: [[&str; 7]; 12] = [
    [
        "Guardhouse",
        "Archers' Tower",
        "Griffin Tower",
        "Barracks",
        "Monastery",
        "Training Grounds",
        "Portal of Glory",
    ],
    [
        "Centaur Stables",
        "Dwarf Cottage",
        "Homestead",
        "Enchanted Spring",
        "Dendroid Arches",
        "Unicorn Glade",
        "Dragon Cliffs",
    ],
    [
        "Workshop",
        "Parapet",
        "Golem Factory",
        "Mage Tower",
        "Altar of Wishes",
        "Golden Pavilion",
        "Cloud Temple",
    ],
    [
        "Imp Crucible",
        "Hall of Sins",
        "Kennels",
        "Demon Gate",
        "Hell Hole",
        "Fire Lake",
        "Forsaken Palace",
    ],
    [
        "Cursed Temple",
        "Graveyard",
        "Tomb of Souls",
        "Estate",
        "Mausoleum",
        "Hall of Darkness",
        "Dragon Vault",
    ],
    [
        "Warren",
        "Harpy Loft",
        "Pillar of Eyes",
        "Chapel of Stilled Voices",
        "Labyrinth",
        "Manticore Lair",
        "Dragon Cave",
    ],
    [
        "Goblin Barracks",
        "Wolf Pen",
        "Orc Tower",
        "Ogre Fort",
        "Cliff Nest",
        "Cyclops Cave",
        "Behemoth Lair",
    ],
    [
        "Gnoll Hut",
        "Lizard Den",
        "Serpent Fly Hive",
        "Basilisk Pit",
        "Gorgon Lair",
        "Wyvern Nest",
        "Hydra Pond",
    ],
    [
        "Magic Lantern",
        "Altar of Air",
        "Altar of Water",
        "Altar of Fire",
        "Altar of Earth",
        "Altar of Thought",
        "Pyre",
    ],
    [
        "Nymph Waterfall",
        "Shack",
        "Frigate",
        "Nest",
        "Tower of the Seas",
        "Nix Fort",
        "Maelstrom",
    ],
    [
        "Halfling Adobe",
        "Foundry",
        "Ranch",
        "Manufactory",
        "Catacombs",
        "Watchtower",
        "Serpentarium",
    ],
    [
        "Colliery",
        "Frigid Spur",
        "Hall of the Bold",
        "Mountain Embassy",
        "Shaman Fane",
        "Mammoth Stalls",
        "Frosthome",
    ],
];

/// Factory town type: its guild and shipyard slots differ from the others.
const FACTORY: u8 = 10;

/// Faction-specific name of a special, horde or grail building, when the
/// faction has a verified table entry. An upgraded horde bit shares the name
/// of its base bit.
pub fn special_name(town_type: u8, building: u8) -> Option<&'static str> {
    let lookup = match building {
        building::HORDE_1_UPGRADED => building::HORDE_1,
        building::HORDE_2_UPGRADED => building::HORDE_2,
        other => other,
    };
    SPECIAL_NAMES
        .get(town_type as usize)?
        .iter()
        .find(|(id, _)| *id == lookup)
        .map(|(_, name)| *name)
}

/// Display group of a building bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingGroup {
    Halls,
    Fortifications,
    MageGuild,
    Economy,
    Dwellings,
    Special,
    Other,
}

/// Group of one building bit.
pub fn building_group(building: u8) -> BuildingGroup {
    use building::*;
    match building {
        MAGE_GUILD_1..=MAGE_GUILD_5 => BuildingGroup::MageGuild,
        FORT..=CASTLE => BuildingGroup::Fortifications,
        VILLAGE_HALL..=CAPITOL => BuildingGroup::Halls,
        TAVERN | SHIPYARD | SHIP | MARKETPLACE | RESOURCE_SILO | BLACKSMITH => {
            BuildingGroup::Economy
        }
        DWELLING_1..=DWELLING_7_UPGRADED => BuildingGroup::Dwellings,
        SPECIAL_17 | HORDE_1 | HORDE_1_UPGRADED | SPECIAL_21..=GRAIL => BuildingGroup::Special,
        _ => BuildingGroup::Other,
    }
}

/// Display name of one building bit for a town type.
pub fn building_name(town_type: u8, building: u8) -> String {
    use building::*;
    if let Some(name) = special_name(town_type, building) {
        return match building {
            HORDE_1_UPGRADED | HORDE_2_UPGRADED => format!("{name} (upgraded dwelling)"),
            _ => name.to_owned(),
        };
    }
    if let Some(names) = DWELLING_NAMES.get(town_type as usize) {
        match building {
            DWELLING_1..=DWELLING_7 => return names[(building - DWELLING_1) as usize].to_owned(),
            DWELLING_1_UPGRADED..=DWELLING_7_UPGRADED => {
                return format!("Upg. {}", names[(building - DWELLING_1_UPGRADED) as usize]);
            }
            _ => {}
        }
    }
    if town_type == FACTORY && building == SHIP {
        return "Unused structure (slot 20)".into();
    }
    match building {
        MAGE_GUILD_1..=MAGE_GUILD_5 => format!("Mage Guild Level {}", building + 1),
        TAVERN => "Tavern".into(),
        SHIPYARD => "Shipyard".into(),
        FORT => "Fort".into(),
        CITADEL => "Citadel".into(),
        CASTLE => "Castle".into(),
        VILLAGE_HALL => "Village Hall".into(),
        TOWN_HALL => "Town Hall".into(),
        CITY_HALL => "City Hall".into(),
        CAPITOL => "Capitol".into(),
        MARKETPLACE => "Marketplace".into(),
        RESOURCE_SILO => "Resource Silo".into(),
        BLACKSMITH => "Blacksmith".into(),
        SHIP => "Shipyard with ship".into(),
        HORDE_1 => "Horde building 1".into(),
        HORDE_1_UPGRADED => "Horde building 1 (upgraded dwelling)".into(),
        HORDE_2 => "Horde building 2".into(),
        HORDE_2_UPGRADED => "Horde building 2 (upgraded dwelling)".into(),
        GRAIL => "Grail structure".into(),
        SPECIAL_17 | SPECIAL_21..=SPECIAL_23 => format!("Special building (slot {building})"),
        EXTRA_27..=EXTRA_29 => format!("Extra structure (slot {building})"),
        DWELLING_1..=DWELLING_7 => format!("Dwelling Level {}", building - DWELLING_1 + 1),
        DWELLING_1_UPGRADED..=DWELLING_7_UPGRADED => {
            format!(
                "Upgraded Dwelling Level {}",
                building - DWELLING_1_UPGRADED + 1
            )
        }
        other => format!("Building bit {other}"),
    }
}

/// The building that must be built before `building`, inside an upgrade
/// chain shared by every faction. Unrelated in-game build-tree requirements
/// (for example a Blacksmith before a dwelling) are not modeled, exactly as
/// in the original map editor.
pub fn prerequisite(building: u8) -> Option<u8> {
    use building::*;
    match building {
        1..=MAGE_GUILD_5 => Some(building - 1),
        CITADEL => Some(FORT),
        CASTLE => Some(CITADEL),
        TOWN_HALL => Some(VILLAGE_HALL),
        CITY_HALL => Some(TOWN_HALL),
        CAPITOL => Some(CITY_HALL),
        RESOURCE_SILO => Some(MARKETPLACE),
        HORDE_1_UPGRADED => Some(HORDE_1),
        HORDE_2_UPGRADED => Some(HORDE_2),
        DWELLING_1_UPGRADED..=DWELLING_7_UPGRADED => Some(building - 7),
        _ => None,
    }
}

/// Creature level (1-7) whose dwelling each of the two horde buildings
/// boosts, by town type. `None`: the faction has no such horde, or its
/// effect on the drawn dwelling was not observed in the sample (Dungeon has
/// no horde town there; Cove's Roost always appears together with the
/// Gunpowder Warehouse, so its hidden dwelling cannot be told apart).
const HORDE_LEVELS: [[Option<u8>; 2]; 12] = [
    [Some(3), None],    // Castle: Griffin Bastion
    [Some(2), Some(5)], // Rampart: Miners' Guild, Dendroid Saplings
    [Some(2), None],    // Tower: Sculptor's Wings
    [Some(1), Some(3)], // Inferno: Birthing Pools, Cages
    [Some(1), None],    // Necropolis: Unearthed Graves
    [None, None],       // Dungeon: Mushroom Rings (not observed)
    [Some(1), None],    // Stronghold: Mess Hall
    [Some(1), None],    // Fortress: Captain's Quarters
    [Some(1), None],    // Conflux: Garden of Life
    [Some(2), None],    // Cove: Pub; Roost not separable
    [Some(3), None],    // Factory: Pen
    [Some(6), None],    // Bulwark: Hay Bins
];

/// Base bit of horde building 0 or 1.
const fn horde_base(index: usize) -> u8 {
    if index == 0 {
        building::HORDE_1
    } else {
        building::HORDE_2
    }
}

/// Which horde building (0 or 1) a bit belongs to.
fn horde_index(building: u8) -> Option<usize> {
    match building {
        building::HORDE_1 | building::HORDE_1_UPGRADED => Some(0),
        building::HORDE_2 | building::HORDE_2_UPGRADED => Some(1),
        _ => None,
    }
}

/// The building whose graphic replaces `building` once built (same chains as
/// [`prerequisite`], minus the Marketplace/Resource Silo pair, which are
/// separate structures, plus the horde base/upgraded pairs).
fn superseded_by(building: u8) -> Option<u8> {
    use building::*;
    match building {
        MAGE_GUILD_1..=3 => Some(building + 1),
        FORT => Some(CITADEL),
        CITADEL => Some(CASTLE),
        VILLAGE_HALL => Some(TOWN_HALL),
        TOWN_HALL => Some(CITY_HALL),
        CITY_HALL => Some(CAPITOL),
        HORDE_1 => Some(HORDE_1_UPGRADED),
        HORDE_2 => Some(HORDE_2_UPGRADED),
        DWELLING_1..=DWELLING_7 => Some(building + 7),
        _ => None,
    }
}

/// Bits of `built` whose graphic is replaced by a built successor.
fn chain_hidden(built: u64) -> u64 {
    (0..building::COUNT)
        .filter(|&id| built & bit(id) != 0)
        .filter(|&id| superseded_by(id).is_some_and(|next| built & bit(next) != 0))
        .fold(0, |mask, id| mask | bit(id))
}

/// Why a building cannot be toggled by this editor, independent of the
/// town's current state.
fn structural_lock(building: u8) -> Option<&'static str> {
    use building::*;
    match building {
        MAGE_GUILD_1..=TAVERN
        | TOWN_HALL
        | CITY_HALL
        | MARKETPLACE
        | RESOURCE_SILO
        | BLACKSMITH
        | DWELLING_1..=DWELLING_7_UPGRADED => None,
        // State-dependent rules live in `TownState::direct_lock`.
        SHIPYARD | HORDE_1 | HORDE_1_UPGRADED | HORDE_2 | HORDE_2_UPGRADED => None,
        // The save stores a separate adventure-map sprite per fortification
        // level and for the Capitol (`avc???x0/0/c0/f0/z0.def` templates);
        // which object record selects it is not located.
        FORT | CITADEL | CASTLE => {
            Some("also changes the town's look on the adventure map; not supported yet")
        }
        VILLAGE_HALL => Some("every town keeps its Village Hall"),
        CAPITOL => Some(
            "one per player and changes the town's look on the adventure map; not supported yet",
        ),
        SHIP => Some("shows a docked ship, which is map state; not supported yet"),
        GRAIL => Some("only one grail exists per map; not supported yet"),
        _ => Some("has extra saved state of its own; not supported yet"),
    }
}

/// Decoded building and Mage Guild fields of one town record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TownState {
    /// Town type byte, needed for faction rules (Library capacity, names).
    pub town_type: u8,
    /// Stored Mage Guild level byte (0-5).
    pub mage_guild_level: u8,
    pub visible: u64,
    pub built: u64,
    pub allowed: u64,
    /// Stored spell count per guild level.
    pub guild_counts: [u8; GUILD_LEVELS],
    /// Raw spell slots; `-1` is empty, slots past the capacity hold
    /// uninitialized data and are never interpreted.
    pub guild_slots: [[i32; GUILD_SLOTS]; GUILD_LEVELS],
    pub banned_spells: [u8; layout::BANNED_LEN],
    /// Tile where boats built here appear (record bytes 7 and 8, h3svg
    /// `generated_boat_x/y`); `None` when the town has no such tile. Never
    /// written.
    pub boat: Option<(u8, u8)>,
}

/// State of one building bit for display and editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildingStatus {
    pub id: u8,
    pub built: bool,
    /// The save allows constructing it here (faction and map restrictions).
    pub allowed: bool,
    /// Why it cannot be toggled; `None` when it is editable.
    pub lock: Option<String>,
}

impl TownState {
    /// Offset of the first post-name byte in a town record.
    pub fn tail_offset(record: &[u8]) -> Option<usize> {
        let name_len = *record.get(NAME_LEN)? as usize;
        Some(NAME + name_len)
    }

    /// Decodes the state from a located town record. `None` when the record
    /// is too short to hold the fields.
    pub fn parse(record: &[u8]) -> Option<TownState> {
        use crate::definitions::towns::{MAGE_GUILD_LEVEL, TYPE};
        let tail = Self::tail_offset(record)?;
        let fields = record.get(tail..tail + layout::END)?;
        let mask = |offset: usize| {
            u64::from_le_bytes(fields[offset..offset + 8].try_into().expect("8 bytes"))
        };
        let mut guild_slots = [[0i32; GUILD_SLOTS]; GUILD_LEVELS];
        for (level, slots) in guild_slots.iter_mut().enumerate() {
            for (slot, value) in slots.iter_mut().enumerate() {
                let at = layout::SPELLS + (level * GUILD_SLOTS + slot) * 4;
                *value = i32::from_le_bytes(fields[at..at + 4].try_into().expect("4 bytes"));
            }
        }
        Some(TownState {
            boat: match (record[BOAT_X], record[BOAT_Y]) {
                (0xFF, _) | (_, 0xFF) => None,
                tile => Some(tile),
            },
            town_type: record[TYPE],
            mage_guild_level: record[MAGE_GUILD_LEVEL],
            visible: mask(layout::VISIBLE),
            built: mask(layout::BUILT),
            allowed: mask(layout::ALLOWED),
            guild_counts: fields[layout::GUILD_COUNTS..layout::GUILD_COUNTS + GUILD_LEVELS]
                .try_into()
                .expect("5 bytes"),
            guild_slots,
            banned_spells: fields[layout::BANNED..layout::BANNED + layout::BANNED_LEN]
                .try_into()
                .expect("9 bytes"),
        })
    }

    pub fn is_built(&self, building: u8) -> bool {
        building < 64 && self.built & bit(building) != 0
    }

    pub fn is_allowed(&self, building: u8) -> bool {
        building < 64 && self.allowed & bit(building) != 0
    }

    /// Whether the Fort is built; selects the town icon variant.
    pub fn has_fort(&self) -> bool {
        self.is_built(building::FORT)
    }

    /// Tower Library: one extra spell per guild level.
    pub fn has_library(&self) -> bool {
        self.town_type == TOWER && self.is_built(building::SPECIAL_22)
    }

    /// Highest guild level whose Mage Guild building is built, counting only
    /// an unbroken chain from level 1.
    pub fn built_guild_level(&self) -> u8 {
        (0..GUILD_LEVELS as u8)
            .take_while(|&level| self.is_built(level))
            .count() as u8
    }

    /// Highest guild level the save allows in this town (built levels count).
    pub fn max_guild_level(&self) -> u8 {
        (0..GUILD_LEVELS as u8)
            .take_while(|&level| self.is_built(level) || self.is_allowed(level))
            .count() as u8
    }

    /// Spell capacity of a zero-based guild level: 5, 4, 3, 2, 1, plus one
    /// with the Library.
    pub fn guild_capacity(&self, level: usize) -> usize {
        (GUILD_LEVELS - level) + usize::from(self.has_library())
    }

    /// Whether a slot value is a spell that belongs to the zero-based level.
    fn slot_spell(level: usize, value: i32) -> Option<u8> {
        let id = u8::try_from(value).ok()?;
        (spell_level(id)? as usize == level + 1).then_some(id)
    }

    /// Spells of one zero-based guild level: the leading valid slots.
    pub fn guild_spells(&self, level: usize) -> Vec<u8> {
        self.guild_slots[level][..self.guild_capacity(level)]
            .iter()
            .map_while(|&value| Self::slot_spell(level, value))
            .collect()
    }

    pub fn spell_banned(&self, spell_id: u8) -> bool {
        self.banned_spells
            .get(spell_id as usize / 8)
            .is_some_and(|byte| byte >> (spell_id % 8) & 1 != 0)
    }

    /// Why the Mage Guild fields do not follow the verified representation
    /// (leading spells of the right level, then empty slots; counts equal to
    /// the spell number of built levels and zero above; level byte equal to
    /// the Mage Guild building chain). `None` when they do.
    pub fn guild_issue(&self) -> Option<String> {
        if self.mage_guild_level != self.built_guild_level() {
            return Some(format!(
                "stored Mage Guild level {} disagrees with the built Mage Guild buildings",
                self.mage_guild_level
            ));
        }
        let guild_bits = self.built & 0x1F;
        if guild_bits != (1u64 << self.mage_guild_level) - 1 {
            return Some("Mage Guild buildings do not form an unbroken chain".into());
        }
        for level in 0..GUILD_LEVELS {
            let spells = self.guild_spells(level);
            let capacity = self.guild_capacity(level);
            if self.guild_slots[level][spells.len()..capacity]
                .iter()
                .any(|&value| value != -1)
            {
                return Some(format!(
                    "Mage Guild level {} holds an unrecognized spell slot",
                    level + 1
                ));
            }
            let mut sorted = spells.clone();
            sorted.sort_unstable();
            sorted.dedup();
            if sorted.len() != spells.len() {
                return Some(format!(
                    "Mage Guild level {} lists a spell twice",
                    level + 1
                ));
            }
            let expected = if level < self.mage_guild_level as usize {
                spells.len()
            } else {
                0
            };
            if self.guild_counts[level] as usize != expected {
                return Some(format!(
                    "Mage Guild level {} stores spell count {} where {} is expected",
                    level + 1,
                    self.guild_counts[level],
                    expected
                ));
            }
        }
        None
    }

    /// Built bits hidden by something other than the shared upgrade chains
    /// (faction-specific graphic replacement). Preserved and locked.
    pub fn unexplained_hidden(&self) -> u64 {
        self.built & !self.visible & !chain_hidden(self.built) & !self.horde_hidden(self.built)
    }

    /// The upgraded dwelling a verified horde building boosts, as a bit.
    fn horde_dwelling(&self, index: usize) -> Option<u8> {
        let level = (*HORDE_LEVELS.get(self.town_type as usize)?)[index]?;
        Some(building::DWELLING_1_UPGRADED + level - 1)
    }

    /// Upgraded dwellings drawn as part of a built horde building in `built`
    /// (sample: every town with a verified horde hides exactly that
    /// dwelling).
    fn horde_hidden(&self, built: u64) -> u64 {
        (0..2)
            .filter(|&index| built & bit(horde_base(index) + 1) != 0)
            .filter_map(|index| self.horde_dwelling(index))
            .fold(0, |mask, dwelling| mask | bit(dwelling))
    }

    /// Why a horde building cannot be toggled. Only the observed form is
    /// editable: both horde bits together, attached to a built upgraded
    /// dwelling that it replaces on screen.
    fn horde_lock(&self, index: usize) -> Option<String> {
        let Some(dwelling) = self.horde_dwelling(index) else {
            return Some("not verified for this faction yet".into());
        };
        let base = horde_base(index);
        let (has_base, has_upgraded) = (self.is_built(base), self.is_built(base + 1));
        if has_base != has_upgraded {
            return Some("attached to a basic dwelling; not supported yet".into());
        }
        if !self.is_built(dwelling) {
            return Some(format!(
                "needs {} first",
                building_name(self.town_type, dwelling)
            ));
        }
        if has_base {
            let as_observed = self.visible & bit(base + 1) != 0
                && self.visible & (bit(base) | bit(dwelling)) == 0;
            if !as_observed {
                return Some("stored in an unusual form; not supported yet".into());
            }
        } else if !(self.is_allowed(base) && self.is_allowed(base + 1)) {
            return Some("not available in this town".into());
        }
        None
    }

    /// Why the building masks deviate from the verified model, which blocks
    /// every building edit of the town. `None` when they follow it.
    pub fn buildings_issue(&self) -> Option<String> {
        if !self.is_built(building::VILLAGE_HALL) {
            return Some("the town has no Village Hall bit; unverified building state".into());
        }
        for id in 0..building::COUNT {
            if self.is_built(id) && prerequisite(id).is_some_and(|needed| !self.is_built(needed)) {
                return Some(format!(
                    "{} is built without its prerequisite; unverified building state",
                    building_name(self.town_type, id)
                ));
            }
        }
        if self.built >> building::COUNT != 0 {
            return Some("unknown building bits are set".into());
        }
        None
    }

    /// Built bits still drawn although their chain successor is built. Only
    /// Factory shows this (Mage Guild 4 beside level 5, identical in every
    /// local save); the bits are preserved and the pair stays locked.
    pub fn shown_with_successor(&self) -> u64 {
        self.visible & chain_hidden(self.built)
    }

    /// Why one building cannot be toggled right now; `None` when editable.
    /// A built building that a locked building depends on is locked too,
    /// because removing it would remove the locked one.
    pub fn building_lock(&self, building: u8) -> Option<String> {
        if let Some(reason) = self.direct_lock(building) {
            return Some(reason);
        }
        if !self.is_built(building) {
            return None;
        }
        let mut dependent = building;
        while let Some(next) = (0..building::COUNT)
            .find(|&id| self.is_built(id) && prerequisite(id) == Some(dependent))
        {
            if self.direct_lock(next).is_some() {
                return Some(format!(
                    "required by {}, which cannot be changed",
                    building_name(self.town_type, next)
                ));
            }
            dependent = next;
        }
        None
    }

    fn direct_lock(&self, building: u8) -> Option<String> {
        if building >= building::COUNT {
            return Some("unknown building".into());
        }
        if let Some(reason) = structural_lock(building) {
            return Some(reason.into());
        }
        if let Some(issue) = self.buildings_issue() {
            return Some(issue);
        }
        let anomaly = self.shown_with_successor();
        let pairs_with_anomaly = (0..building::COUNT)
            .any(|id| anomaly & bit(id) != 0 && superseded_by(id) == Some(building));
        if (self.town_type == FACTORY && building == building::MAGE_GUILD_5)
            || anomaly & bit(building) != 0
            || pairs_with_anomaly
        {
            return Some(
                "this faction draws it together with the previous level; that rule is not \
                 verified for editing"
                    .into(),
            );
        }
        if self.unexplained_hidden() & bit(building) != 0 {
            return Some("shown as part of a faction-specific structure; not supported yet".into());
        }
        if building == building::SHIPYARD {
            if self.town_type == FACTORY {
                return Some("has extra saved state of its own; not supported yet".into());
            }
            if self.boat.is_none() {
                return Some("this town has no water tile for boats".into());
            }
            if self.is_built(building::SHIP) {
                return Some("a ship is docked here; not supported yet".into());
            }
        }
        if let Some(index) = horde_index(building)
            && let Some(reason) = self.horde_lock(index)
        {
            return Some(reason);
        }
        let is_dwelling =
            (building::DWELLING_1..=building::DWELLING_7_UPGRADED).contains(&building);
        if is_dwelling && self.built & HORDE_BITS != 0 {
            let base_without_upgrade = (self.is_built(building::HORDE_1)
                && !self.is_built(building::HORDE_1_UPGRADED))
                || (self.is_built(building::HORDE_2) && !self.is_built(building::HORDE_2_UPGRADED));
            if base_without_upgrade {
                return Some(
                    "a horde building is attached to this town's dwellings; not supported yet"
                        .into(),
                );
            }
            if self.is_built(building) {
                return Some("dwellings cannot be removed while a horde building is built".into());
            }
        }
        if !self.is_built(building) && !self.is_allowed(building) {
            return Some("not available in this town".into());
        }
        if building <= building::MAGE_GUILD_5
            && let Some(issue) = self.guild_issue()
        {
            return Some(issue);
        }
        None
    }

    /// Status of every defined building bit.
    pub fn building_statuses(&self) -> Vec<BuildingStatus> {
        (0..building::COUNT)
            .map(|id| BuildingStatus {
                id,
                built: self.is_built(id),
                allowed: self.is_allowed(id),
                lock: self.building_lock(id),
            })
            .collect()
    }

    /// The logical built mask after constructing or removing one building,
    /// starting from `current` (the base mask or a pending target).
    /// Constructing adds missing chain prerequisites; removing also removes
    /// everything that depends on the building. The result is validated by
    /// [`TownState::validate_built_target`].
    pub fn toggle_building(
        &self,
        current: u64,
        building: u8,
        build: bool,
    ) -> Result<u64, EditError> {
        if building >= building::COUNT {
            return Err(invalid("building", "unknown building".into()));
        }
        let mut target = current;
        if build {
            let mut next = Some(building);
            while let Some(id) = next {
                target |= bit(id);
                next = prerequisite(id);
            }
            if let Some(index) = horde_index(building) {
                target |= bit(horde_base(index) + 1);
            }
        } else {
            target &= !bit(building);
            if let Some(index) = horde_index(building) {
                target &= !(bit(horde_base(index)) | bit(horde_base(index) + 1));
            }
            // Remove dependents until the chain rules hold again.
            loop {
                let orphan = (0..building::COUNT).find(|&id| {
                    target & bit(id) != 0
                        && prerequisite(id).is_some_and(|needed| target & bit(needed) == 0)
                });
                match orphan {
                    Some(id) => target &= !bit(id),
                    None => break,
                }
            }
        }
        self.validate_built_target(target)?;
        Ok(target)
    }

    /// Checks a desired logical built mask against this (base) state: every
    /// changed bit must be editable, constructed bits must be allowed, and
    /// the chain prerequisites must hold.
    pub fn validate_built_target(&self, target: u64) -> Result<(), EditError> {
        let changed = self.built ^ target;
        if changed == 0 {
            return Ok(());
        }
        if let Some(issue) = self.buildings_issue() {
            return Err(invalid("buildings", issue));
        }
        for id in (0..64u8).filter(|&id| changed & bit(id) != 0) {
            if let Some(reason) = self.building_lock(id) {
                return Err(invalid(
                    "buildings",
                    format!(
                        "{} cannot be changed: {reason}",
                        building_name(self.town_type, id)
                    ),
                ));
            }
        }
        for index in 0..2 {
            let base = horde_base(index);
            let pair = bit(base) | bit(base + 1);
            if changed & pair == 0 {
                continue;
            }
            let complete = target & pair == pair;
            let dwelling_kept = self
                .horde_dwelling(index)
                .is_some_and(|dwelling| target & bit(dwelling) != 0);
            if target & pair != 0 && !(complete && dwelling_kept) {
                return Err(invalid(
                    "buildings",
                    format!(
                        "{} needs its upgraded dwelling and both of its stored flags",
                        building_name(self.town_type, base)
                    ),
                ));
            }
        }
        for id in (0..building::COUNT).filter(|&id| target & bit(id) != 0) {
            if let Some(needed) = prerequisite(id)
                && target & bit(needed) == 0
            {
                return Err(invalid(
                    "buildings",
                    format!(
                        "{} requires {}",
                        building_name(self.town_type, id),
                        building_name(self.town_type, needed)
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Checks a desired spell list for one zero-based guild level.
    pub fn validate_guild_spells(&self, level: usize, spells: &[u8]) -> Result<(), EditError> {
        if level >= GUILD_LEVELS {
            return Err(invalid("mage guild", "guild levels are 1-5".into()));
        }
        if let Some(issue) = self.guild_issue() {
            return Err(invalid("mage guild", issue));
        }
        let current = self.guild_spells(level);
        if spells == current.as_slice() {
            return Ok(());
        }
        if level >= self.max_guild_level() as usize {
            return Err(invalid(
                "mage guild",
                format!("Mage Guild level {} cannot exist in this town", level + 1),
            ));
        }
        let capacity = self.guild_capacity(level);
        if spells.len() > capacity {
            return Err(invalid(
                "mage guild",
                format!("level {} holds at most {capacity} spells", level + 1),
            ));
        }
        for (index, &spell) in spells.iter().enumerate() {
            if spell_level(spell).map(usize::from) != Some(level + 1) {
                return Err(invalid(
                    "mage guild",
                    format!("spell {spell} is not a level {} spell", level + 1),
                ));
            }
            if spells[..index].contains(&spell) {
                return Err(invalid(
                    "mage guild",
                    format!("spell {spell} is listed twice"),
                ));
            }
            if self.spell_banned(spell) && !current.contains(&spell) {
                return Err(invalid(
                    "mage guild",
                    format!("spell {spell} is banned in this town by the saved game"),
                ));
            }
        }
        Ok(())
    }

    /// The state after applying a desired built mask and desired spell lists
    /// (`None` keeps the stored value). Callers validate first. Derived
    /// fields follow the verified representation: the Mage Guild level byte
    /// tracks the building chain, counts cover built levels only, and the
    /// visible mask re-applies the shared chains while preserving every bit
    /// this model does not explain.
    pub fn with_changes(
        &self,
        built: Option<u64>,
        spells: &[Option<Vec<u8>>; GUILD_LEVELS],
    ) -> TownState {
        let mut next = self.clone();
        if let Some(target) = built.filter(|&target| target != self.built) {
            let locked_hidden = self.unexplained_hidden();
            let visible_only = self.visible & !self.built;
            // Locked, so still built in every valid target.
            let kept_with_successor = self.shown_with_successor() & target;
            next.built = target;
            next.visible =
                (target & !chain_hidden(target) & !self.horde_hidden(target) & !locked_hidden)
                    | visible_only
                    | kept_with_successor;
            next.mage_guild_level = next.built_guild_level();
        }
        for (level, desired) in spells.iter().enumerate() {
            let Some(desired) = desired else { continue };
            let capacity = self.guild_capacity(level);
            for slot in 0..capacity {
                next.guild_slots[level][slot] =
                    desired.get(slot).map_or(-1, |&spell| i32::from(spell));
            }
        }
        if next != *self {
            for level in 0..GUILD_LEVELS {
                next.guild_counts[level] = if level < next.mage_guild_level as usize {
                    next.guild_spells(level).len() as u8
                } else {
                    0
                };
            }
        }
        next
    }

    /// Field-level differences from `self` to `next` as `(offset relative to
    /// the name end, old bytes, new bytes, label)`, plus the Mage Guild level
    /// byte reported separately because it lives before the name.
    pub fn diff(&self, next: &TownState) -> Vec<(usize, Vec<u8>, Vec<u8>, &'static str)> {
        let mut changes = Vec::new();
        for level in 0..GUILD_LEVELS {
            if self.guild_counts[level] != next.guild_counts[level] {
                changes.push((
                    layout::GUILD_COUNTS + level,
                    vec![self.guild_counts[level]],
                    vec![next.guild_counts[level]],
                    "mage guild spell count",
                ));
            }
        }
        if self.visible != next.visible {
            changes.push((
                layout::VISIBLE,
                self.visible.to_le_bytes().to_vec(),
                next.visible.to_le_bytes().to_vec(),
                "visible buildings",
            ));
        }
        if self.built != next.built {
            changes.push((
                layout::BUILT,
                self.built.to_le_bytes().to_vec(),
                next.built.to_le_bytes().to_vec(),
                "built buildings",
            ));
        }
        for level in 0..GUILD_LEVELS {
            for slot in 0..GUILD_SLOTS {
                let (old, new) = (self.guild_slots[level][slot], next.guild_slots[level][slot]);
                if old != new {
                    changes.push((
                        layout::SPELLS + (level * GUILD_SLOTS + slot) * 4,
                        old.to_le_bytes().to_vec(),
                        new.to_le_bytes().to_vec(),
                        "mage guild spell",
                    ));
                }
            }
        }
        changes
    }
}

fn invalid(field: &'static str, reason: String) -> EditError {
    EditError::InvalidValue { field, reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use building::*;

    fn mask(bits: &[u8]) -> u64 {
        bits.iter().fold(0, |mask, &id| mask | bit(id))
    }

    /// A consistent town: Village Hall, Fort, Mage Guild 2, Tavern, one
    /// dwelling; everything in the editable set allowed.
    fn town() -> TownState {
        let built = mask(&[0, 1, TAVERN, FORT, VILLAGE_HALL, DWELLING_1]);
        let mut slots = [[-1i32; GUILD_SLOTS]; GUILD_LEVELS];
        slots[0][..5].copy_from_slice(&[15, 46, 43, 42, 31]);
        slots[1][..4].copy_from_slice(&[24, 13, 64, 16]);
        slots[2][..3].copy_from_slice(&[39, 52, 34]);
        TownState {
            town_type: 4,
            mage_guild_level: 2,
            visible: built & !chain_hidden(built),
            built,
            allowed: (1u64 << COUNT) - 1,
            guild_counts: [5, 4, 0, 0, 0],
            guild_slots: slots,
            banned_spells: [0; 9],
            boat: None,
        }
    }

    #[test]
    fn horde_is_built_and_removed_as_one_unit() {
        // Necropolis: Unearthed Graves boosts level 1.
        let mut town = town();
        town.built |= bit(DWELLING_1_UPGRADED);
        town.visible = town.built & !chain_hidden(town.built);
        // Either bit builds both, hides the upgraded dwelling, shows bit 19.
        for clicked in [HORDE_1, HORDE_1_UPGRADED] {
            let target = town
                .toggle_building(town.built, clicked, true)
                .expect("build");
            assert_eq!(target, town.built | bit(HORDE_1) | bit(HORDE_1_UPGRADED));
            let next = town.with_changes(Some(target), &Default::default());
            assert_eq!(
                next.visible,
                (town.visible & !bit(DWELLING_1_UPGRADED)) | bit(HORDE_1_UPGRADED)
            );
            assert_eq!(next.unexplained_hidden(), 0);
            // Removing it restores the original masks exactly.
            let back = next
                .toggle_building(next.built, clicked, false)
                .expect("remove");
            assert_eq!(next.with_changes(Some(back), &Default::default()), town);
        }
        // Half a horde is never a valid target.
        assert!(
            town.validate_built_target(town.built | bit(HORDE_1))
                .is_err()
        );
        // Without the upgraded dwelling, or for an unobserved faction: locked.
        let mut basic = town.clone();
        basic.built &= !bit(DWELLING_1_UPGRADED);
        basic.visible = basic.built & !chain_hidden(basic.built);
        assert!(basic.toggle_building(basic.built, HORDE_1, true).is_err());
        let mut dungeon = town.clone();
        dungeon.town_type = 5;
        assert!(
            dungeon
                .toggle_building(dungeon.built, HORDE_1, true)
                .is_err()
        );
        assert!(town.toggle_building(town.built, HORDE_2, true).is_err());
    }

    #[test]
    fn shipyard_needs_a_boat_tile_and_no_docked_ship() {
        let mut town = town();
        assert!(town.toggle_building(town.built, SHIPYARD, true).is_err());
        town.boat = Some((10, 12));
        let target = town
            .toggle_building(town.built, SHIPYARD, true)
            .expect("build");
        let next = town.with_changes(Some(target), &Default::default());
        assert_eq!(next.visible, town.visible | bit(SHIPYARD));
        assert_eq!(next.boat, town.boat);
        let mut docked = next.clone();
        docked.built |= bit(SHIP);
        assert!(
            docked
                .toggle_building(docked.built, SHIPYARD, false)
                .is_err()
        );
        let mut factory = town.clone();
        factory.town_type = FACTORY;
        assert!(
            factory
                .toggle_building(factory.built, SHIPYARD, true)
                .is_err()
        );
    }

    #[test]
    fn spell_levels_cover_the_guild_shape() {
        let per_level = |level| SPELL_LEVELS.iter().filter(|&&l| l == level).count();
        assert_eq!(
            [1, 2, 3, 4, 5].map(per_level),
            [15, 15, 15, 15, 10],
            "level histogram of the 70 guild spells"
        );
    }

    #[test]
    fn consistent_town_has_no_issues() {
        let town = town();
        assert_eq!(town.guild_issue(), None);
        assert_eq!(town.buildings_issue(), None);
        assert_eq!(town.guild_spells(2), vec![39, 52, 34]);
        assert_eq!(town.guild_capacity(0), 5);
    }

    #[test]
    fn building_guild_level_updates_level_counts_and_visible() {
        let town = town();
        let target = town
            .toggle_building(town.built, 3, true)
            .expect("build MG4");
        assert_eq!(target & 0x1F, 0x0F, "prerequisite level 3 added");
        let next = town.with_changes(Some(target), &Default::default());
        assert_eq!(next.mage_guild_level, 4);
        assert_eq!(next.guild_counts, [5, 4, 3, 0, 0]);
        assert_eq!(next.visible & 0x1F, 0x08, "only the top guild is drawn");
        assert_eq!(next.guild_issue(), None);
        assert_eq!(next.guild_slots, town.guild_slots);
    }

    #[test]
    fn removal_cascades_to_dependents() {
        let town = town();
        let target = town
            .toggle_building(town.built, 0, false)
            .expect("remove MG1");
        assert_eq!(target & 0x1F, 0);
        let next = town.with_changes(Some(target), &Default::default());
        assert_eq!(next.mage_guild_level, 0);
        assert_eq!(next.guild_counts, [0; 5]);
    }

    #[test]
    fn locked_and_unavailable_buildings_are_refused() {
        let mut town = town();
        assert!(town.toggle_building(town.built, CAPITOL, true).is_err());
        assert!(town.toggle_building(town.built, FORT, false).is_err());
        assert!(town.toggle_building(town.built, SPECIAL_22, true).is_err());
        town.allowed &= !bit(MARKETPLACE);
        // The Resource Silo needs the Marketplace, which is not allowed here.
        assert!(
            town.toggle_building(town.built, RESOURCE_SILO, true)
                .is_err()
        );
        assert!(town.toggle_building(town.built, CITADEL, true).is_err());
        assert!(town.toggle_building(town.built, TOWN_HALL, true).is_ok());
    }

    #[test]
    fn horde_blocks_dwelling_removal_but_not_construction() {
        let mut town = town();
        town.built |= mask(&[DWELLING_1_UPGRADED, HORDE_1, HORDE_1_UPGRADED]);
        town.visible = mask(&[1, TAVERN, FORT, VILLAGE_HALL, HORDE_1_UPGRADED]);
        assert_eq!(town.buildings_issue(), None);
        assert_eq!(
            town.unexplained_hidden(),
            0,
            "the horde explains the hidden dwelling"
        );
        assert!(town.toggle_building(town.built, DWELLING_1, false).is_err());
        let target = town
            .toggle_building(town.built, DWELLING_1 + 2, true)
            .expect("new dwelling");
        let next = town.with_changes(Some(target), &Default::default());
        assert_eq!(
            next.visible,
            town.visible | bit(DWELLING_1 + 2),
            "horde-hidden dwelling stays hidden"
        );
    }

    #[test]
    fn level_drawn_beside_its_upgrade_locks_only_that_pair() {
        let mut town = town();
        town.visible |= bit(0); // MG1 drawn although MG2 is built.
        assert_eq!(town.buildings_issue(), None);
        assert!(town.building_lock(0).is_some());
        assert!(town.building_lock(1).is_some());
        assert!(town.toggle_building(town.built, 1, false).is_err());
        // Other buildings stay editable and the odd bit is preserved.
        let target = town
            .toggle_building(town.built, TAVERN, false)
            .expect("tavern");
        let next = town.with_changes(Some(target), &Default::default());
        assert_eq!(next.visible, town.visible & !bit(TAVERN));
    }

    #[test]
    fn factory_keeps_guild_five_locked_and_names_its_slots() {
        let mut town = town();
        town.town_type = FACTORY;
        town.built |= mask(&[2, 3]);
        town.visible = town.built & !chain_hidden(town.built);
        town.mage_guild_level = 4;
        town.guild_counts = [5, 4, 3, 0, 0];
        assert!(town.toggle_building(town.built, 4, true).is_err());
        assert!(town.toggle_building(town.built, 3, false).is_ok());
        assert_eq!(building_name(FACTORY, SHIPYARD), "Mana Generator");
        assert_eq!(building_name(FACTORY, SPECIAL_23), "Upg. Gantry");
        assert_eq!(building_name(FACTORY, DWELLING_1), "Halfling Adobe");
        assert_eq!(building_name(9, HORDE_2), "Roost");
        assert_eq!(building_name(11, DWELLING_7_UPGRADED), "Upg. Frosthome");
        assert_eq!(building_name(11, GRAIL), "Spirits of the Forebears");
        assert_eq!(building_name(0, DWELLING_1), "Guardhouse");
        assert_eq!(building_name(3, SPECIAL_17), "Special building (slot 17)");
    }

    #[test]
    fn locked_dependents_lock_their_prerequisites() {
        let mut town = town();
        town.town_type = FACTORY;
        town.built |= mask(&[2, 3, 4]);
        town.visible = (town.built & !chain_hidden(town.built)) | bit(3);
        town.mage_guild_level = 5;
        town.guild_counts = [5, 4, 3, 0, 0];
        for level in 0..5 {
            assert!(town.building_lock(level).is_some(), "MG{}", level + 1);
        }
        assert_eq!(town.building_lock(TAVERN), None);
    }

    #[test]
    fn guild_spell_lists_are_validated() {
        let mut town = town();
        assert!(town.validate_guild_spells(0, &[15, 46, 43, 42, 27]).is_ok());
        assert!(town.validate_guild_spells(0, &[15, 46]).is_ok());
        assert!(town.validate_guild_spells(0, &[15, 15]).is_err());
        assert!(town.validate_guild_spells(0, &[24]).is_err(), "wrong level");
        assert!(
            town.validate_guild_spells(0, &[15, 46, 43, 42, 31, 27])
                .is_err()
        );
        town.banned_spells[27 / 8] |= 1 << (27 % 8);
        assert!(town.validate_guild_spells(0, &[27]).is_err(), "banned");
        town.allowed &= !mask(&[2, 3, 4]);
        assert!(
            town.validate_guild_spells(2, &[39]).is_err(),
            "level cannot exist"
        );
    }

    #[test]
    fn spell_edit_rewrites_slots_and_count() {
        let town = town();
        let mut spells: [Option<Vec<u8>>; GUILD_LEVELS] = Default::default();
        spells[0] = Some(vec![15, 27]);
        spells[2] = Some(vec![39]);
        let next = town.with_changes(None, &spells);
        assert_eq!(next.guild_slots[0], [15, 27, -1, -1, -1, -1]);
        assert_eq!(
            next.guild_counts,
            [2, 4, 0, 0, 0],
            "unbuilt level keeps count 0"
        );
        assert_eq!(next.guild_issue(), None);
        let diff = town.diff(&next);
        assert!(diff.iter().all(|(_, old, new, _)| old.len() == new.len()));
        assert_eq!(
            diff.iter()
                .filter(|(_, _, _, label)| *label == "mage guild spell")
                .count(),
            6
        );
    }

    #[test]
    fn library_adds_one_slot_per_level() {
        let mut town = town();
        town.town_type = TOWER;
        town.built |= bit(SPECIAL_22);
        town.visible |= bit(SPECIAL_22);
        assert_eq!(town.guild_capacity(0), 6);
        assert_eq!(town.guild_capacity(4), 2);
    }
}
