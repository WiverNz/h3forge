//! Verified artifact IDs for HotA 44/11 backpack editing (H3F-011).
//!
//! Provenance: h3sed 4.1 `metadata.IDS` artifact entries (base SoD set,
//! values 0x00-0x7E) merged with `version/hota.py IDS` (HotA additions
//! 0x7F-0xA5), crossed against the h3sed version-aware inventory artifact
//! lists (`metadata.ARTIFACTS` + `version/hota.ARTIFACTS` + Spellbook/The
//! Grail). Like the spell and skill tables, these are binary artifact IDs,
//! not display-order indexes. Every ID was checked to appear exactly once;
//! the table drives both display names and write validation.

/// Artifact ID of a spell scroll; its backpack entry pairs it with a spell
/// ID in the second word (`01 00 00 00 SS 00 00 00`).
pub const SCROLL_ARTIFACT_ID: u32 = 1;
/// Artifact ID 0 is ambiguous in inventory and is never writable.
pub const SPELLBOOK_ARTIFACT_ID: u32 = 0;
/// Fixed backpack capacity: 64 eight-byte entries at record offset 534.
pub const BACKPACK_SLOTS: usize = 64;

/// (binary artifact ID, name) for every verified HotA 44/11 inventory
/// artifact, sorted by ID. Scrolls are not listed; they are encoded via
/// [`SCROLL_ARTIFACT_ID`] plus a spell ID.
pub const HOTA_44_11_ARTIFACTS: &[(u32, &str)] = &[
    (0, "Spellbook"),
    (2, "The Grail"),
    (7, "Centaur's Axe"),
    (8, "Blackshard of the Dead Knight"),
    (9, "Greater Gnoll's Flail"),
    (10, "Ogre's Club of Havoc"),
    (11, "Sword of Hellfire"),
    (12, "Titan's Gladius"),
    (13, "Shield of the Dwarven Lords"),
    (14, "Shield of the Yawning Dead"),
    (15, "Buckler of the Gnoll King"),
    (16, "Targ of the Rampaging Ogre"),
    (17, "Shield of the Damned"),
    (18, "Sentinel's Shield"),
    (19, "Helm of the Alabaster Unicorn"),
    (20, "Skull Helmet"),
    (21, "Helm of Chaos"),
    (22, "Crown of the Supreme Magi"),
    (23, "Hellstorm Helmet"),
    (24, "Thunder Helmet"),
    (25, "Breastplate of Petrified Wood"),
    (26, "Rib Cage"),
    (27, "Scales of the Greater Basilisk"),
    (28, "Tunic of the Cyclops King"),
    (29, "Breastplate of Brimstone"),
    (30, "Titan's Cuirass"),
    (31, "Armor of Wonder"),
    (32, "Sandals of the Saint"),
    (33, "Celestial Necklace of Bliss"),
    (34, "Lion's Shield of Courage"),
    (35, "Sword of Judgement"),
    (36, "Helm of Heavenly Enlightenment"),
    (37, "Quiet Eye of the Dragon"),
    (38, "Red Dragon Flame Tongue"),
    (39, "Dragon Scale Shield"),
    (40, "Dragon Scale Armor"),
    (41, "Dragonbone Greaves"),
    (42, "Dragon Wing Tabard"),
    (43, "Necklace of Dragonteeth"),
    (44, "Crown of Dragontooth"),
    (45, "Still Eye of the Dragon"),
    (46, "Clover of Fortune"),
    (47, "Cards of Prophecy"),
    (48, "Ladybird of Luck"),
    (49, "Badge of Courage"),
    (50, "Crest of Valor"),
    (51, "Glyph of Gallantry"),
    (52, "Speculum"),
    (53, "Spyglass"),
    (54, "Amulet of the Undertaker"),
    (55, "Vampire's Cowl"),
    (56, "Dead Man's Boots"),
    (57, "Garniture of Interference"),
    (58, "Surcoat of Counterpoise"),
    (59, "Boots of Polarity"),
    (60, "Bow of Elven Cherrywood"),
    (61, "Bowstring of the Unicorn's Mane"),
    (62, "Angel Feather Arrows"),
    (63, "Bird of Perception"),
    (64, "Stoic Watchman"),
    (65, "Emblem of Cognizance"),
    (66, "Statesman's Medal"),
    (67, "Diplomat's Ring"),
    (68, "Ambassador's Sash"),
    (69, "Ring of the Wayfarer"),
    (70, "Equestrian's Gloves"),
    (71, "Necklace of Ocean Guidance"),
    (72, "Angel Wings"),
    (73, "Charm of Mana"),
    (74, "Talisman of Mana"),
    (75, "Mystic Orb of Mana"),
    (76, "Collar of Conjuring"),
    (77, "Ring of Conjuring"),
    (78, "Cape of Conjuring"),
    (79, "Orb of the Firmament"),
    (80, "Orb of Silt"),
    (81, "Orb of Tempestuous Fire"),
    (82, "Orb of Driving Rain"),
    (83, "Recanter's Cloak"),
    (84, "Spirit of Oppression"),
    (85, "Hourglass of the Evil Hour"),
    (86, "Tome of Fire"),
    (87, "Tome of Air"),
    (88, "Tome of Water"),
    (89, "Tome of Earth"),
    (90, "Boots of Levitation"),
    (91, "Golden Bow"),
    (92, "Sphere of Permanence"),
    (93, "Orb of Vulnerability"),
    (94, "Ring of Vitality"),
    (95, "Ring of Life"),
    (96, "Vial of Lifeblood"),
    (97, "Necklace of Swiftness"),
    (98, "Boots of Speed"),
    (99, "Cape of Velocity"),
    (100, "Pendant of Dispassion"),
    (101, "Pendant of Second Sight"),
    (102, "Pendant of Holiness"),
    (103, "Pendant of Life"),
    (104, "Pendant of Death"),
    (105, "Pendant of Free Will"),
    (106, "Pendant of Negativity"),
    (107, "Pendant of Total Recall"),
    (108, "Pendant of Courage"),
    (109, "Everflowing Crystal Cloak"),
    (110, "Ring of Infinite Gems"),
    (111, "Everpouring Vial of Mercury"),
    (112, "Inexhaustible Cart of Ore"),
    (113, "Eversmoking Ring of Sulfur"),
    (114, "Inexhaustible Cart of Lumber"),
    (115, "Endless Sack of Gold"),
    (116, "Endless Bag of Gold"),
    (117, "Endless Purse of Gold"),
    (118, "Legs of Legion"),
    (119, "Loins of Legion"),
    (120, "Torso of Legion"),
    (121, "Arms of Legion"),
    (122, "Head of Legion"),
    (123, "Sea Captain's Hat"),
    (124, "Spellbinder's Hat"),
    (125, "Shackles of War"),
    (126, "Orb of Inhibition"),
    (127, "Vial of Dragon Blood"),
    (128, "Armageddon's Blade"),
    (129, "Angelic Alliance"),
    (130, "Cloak of the Undead King"),
    (131, "Elixir of Life"),
    (132, "Armor of the Damned"),
    (133, "Statue of Legion"),
    (134, "Power of the Dragon Father"),
    (135, "Titan's Thunder"),
    (136, "Admiral's Hat"),
    (137, "Bow of the Sharpshooter"),
    (138, "Wizard's Well"),
    (139, "Ring of the Magi"),
    (140, "Cornucopia"),
    (141, "Diplomat's Cloak"),
    (142, "Pendant of Reflection"),
    (143, "Ironfist of the Ogre"),
    (147, "Trident of Dominion"),
    (148, "Shield of Naval Glory"),
    (149, "Royal Armor of Nix"),
    (150, "Crown of the Five Seas"),
    (151, "Wayfarer's Boots"),
    (152, "Runes of Imminency"),
    (153, "Demon's Horseshoe"),
    (154, "Shaman's Puppet"),
    (155, "Hideous Mask"),
    (156, "Ring of Suppression"),
    (157, "Pendant of Downfall"),
    (158, "Ring of Oblivion"),
    (159, "Cape of Silence"),
    (160, "Golden Goose"),
    (161, "Horn of the Abyss"),
    (162, "Charm of Eclipse"),
    (163, "Seal of Sunset"),
    (164, "Plate of Dying Light"),
    (165, "Sleepkeeper"),
];

/// Name for one verified artifact ID; `None` for unknown/unsupported IDs,
/// which stay read-only.
pub fn name(artifact_id: u32) -> Option<&'static str> {
    HOTA_44_11_ARTIFACTS
        .iter()
        .find(|&&(id, _)| id == artifact_id)
        .map(|&(_, name)| name)
}

/// Wear-slot category for an artifact (h3sed `metadata.ARTIFACT_SLOTS`
/// reduced to the primary slot; ordinary artifacts occupy exactly one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSlotKind {
    Helm,
    Cloak,
    Neck,
    Weapon,
    Shield,
    Armor,
    Hand,
    Feet,
    Side,
}

impl EquipmentSlotKind {
    /// Every wear-slot kind, in reservation-byte order.
    pub const ALL: [EquipmentSlotKind; 9] = {
        use EquipmentSlotKind::*;
        [Helm, Cloak, Neck, Weapon, Shield, Armor, Hand, Feet, Side]
    };

    /// Index of this kind's counter inside the 10-byte reservation region at
    /// record offset 1046 (h3sed `HERO_BYTE_POSITIONS["reserved"]`: helm 1047
    /// ... side 1055; byte 1046 itself is not a counter).
    pub fn reservation_index(self) -> usize {
        use EquipmentSlotKind::*;
        match self {
            Helm => 1,
            Cloak => 2,
            Neck => 3,
            Weapon => 4,
            Shield => 5,
            Armor => 6,
            Hand => 7,
            Feet => 8,
            Side => 9,
        }
    }

    /// Number of wear slots of this kind (two hands, five side slots).
    pub fn capacity(self) -> u8 {
        match self {
            EquipmentSlotKind::Hand => 2,
            EquipmentSlotKind::Side => 5,
            _ => 1,
        }
    }

    pub fn label(self) -> &'static str {
        use EquipmentSlotKind::*;
        match self {
            Helm => "helm",
            Cloak => "cloak",
            Neck => "neck",
            Weapon => "weapon",
            Shield => "shield",
            Armor => "armor",
            Hand => "hand",
            Feet => "feet",
            Side => "side",
        }
    }
}

/// Equipment slot index -> wear-slot kind; `None` for the war-machine slots
/// (ballista/ammo/tent/catapult) and the spellbook slot, which stay
/// read-only (h3sed `metadata.EQUIPMENT_SLOTS`/`HERO_BYTE_POSITIONS` order).
pub fn wear_slot_kind(slot_index: u8) -> Option<EquipmentSlotKind> {
    Some(match slot_index {
        0 => EquipmentSlotKind::Helm,
        1 => EquipmentSlotKind::Cloak,
        2 => EquipmentSlotKind::Neck,
        3 => EquipmentSlotKind::Weapon,
        4 => EquipmentSlotKind::Shield,
        5 => EquipmentSlotKind::Armor,
        6 | 7 => EquipmentSlotKind::Hand,
        8 => EquipmentSlotKind::Feet,
        9..=12 | 18 => EquipmentSlotKind::Side,
        _ => return None,
    })
}

/// Primary wear slot for every writable artifact (h3sed
/// `metadata.ARTIFACT_SLOTS` plus the `version/sod.py` and `version/hota.py`
/// additions, names crossed to binary IDs). Artifacts absent here (The
/// Grail, spell scrolls, the spellbook, war machines) can never be equipped.
/// The HotA primary slots agree with every equipped occurrence in the 44/11
/// samples (147 weapon, 148 shield, 149 armor, 150 helm, 151 feet, 153-155,
/// 162 and 165 side, 156 and 163 hand).
pub fn slot_kind(artifact_id: u32) -> Option<EquipmentSlotKind> {
    use EquipmentSlotKind::*;
    Some(match artifact_id {
        19 | 20 | 21 | 22 | 23 | 24 | 36 | 44 | 123 | 124 | 136 | 150 => Helm,
        42 | 55 | 58 | 68 | 72 | 78 | 83 | 99 | 109 | 130 | 141 | 159 => Cloak,
        33 | 43 | 54 | 57 | 66 | 71 | 76 | 97 | 100..=108 | 142 | 157 => Neck,
        7..=12 | 35 | 38 | 128 | 129 | 135 | 143 | 147 => Weapon,
        13..=18 | 34 | 39 | 148 => Shield,
        25..=31 | 40 | 132 | 134 | 149 | 164 => Armor,
        37 | 45 | 67 | 69 | 70 | 77 | 94 | 95 | 110 | 113 | 139 | 156 | 158 | 163 => Hand,
        32 | 41 | 56 | 59 | 90 | 98 | 151 => Feet,
        46..=53
        | 60..=65
        | 73..=75
        | 79..=82
        | 84..=89
        | 91..=93
        | 96
        | 111
        | 112
        | 114..=122
        | 125
        | 126
        | 127
        | 131
        | 133
        | 137
        | 138
        | 140
        | 152..=155
        | 160..=162
        | 165 => Side,
        _ => return None,
    })
}

/// Extra slots a combination artifact reserves besides its primary slot
/// (h3sed `ARTIFACT_SLOTS[name][1:]` for SoD and HotA). The game stores them
/// as per-kind counters, not positions. Empty for ordinary artifacts.
///
/// Verified against the 44/11 samples: Power of the Dragon Father (134),
/// Cloak of the Undead King (130), Bow of the Sharpshooter (137) and Armor
/// of the Damned (132) each produce exactly these counters at 1047-1055.
pub fn reserved_slots(artifact_id: u32) -> &'static [EquipmentSlotKind] {
    use EquipmentSlotKind::*;
    match artifact_id {
        129 => &[Helm, Neck, Armor, Shield, Feet],
        130 => &[Neck, Feet],
        131 => &[Hand, Hand],
        132 => &[Helm, Weapon, Shield],
        133 => &[Side, Side, Side, Side],
        134 => &[Helm, Neck, Weapon, Shield, Hand, Hand, Cloak, Feet],
        135 => &[Helm, Armor, Shield],
        136 => &[Neck],
        137 => &[Side, Side],
        138 => &[Side, Side],
        139 => &[Neck, Cloak],
        140 => &[Hand, Hand, Cloak],
        141 => &[Neck, Hand],
        142 => &[Cloak, Feet],
        143 => &[Helm, Armor, Shield],
        160 => &[Side, Side],
        _ => &[],
    }
}

/// Component artifact IDs of a combination artifact, in h3sed slot order
/// (`COMBINATION_ARTIFACTS` for SoD and HotA). Empty for ordinary artifacts.
/// Display-only: H3Forge never assembles or disassembles automatically.
pub fn components(artifact_id: u32) -> &'static [u32] {
    match artifact_id {
        129 => &[35, 36, 33, 31, 34, 32],
        130 => &[55, 54, 56],
        131 => &[96, 95, 94],
        132 => &[26, 20, 8, 14],
        133 => &[122, 121, 120, 119, 118],
        134 => &[40, 44, 43, 38, 39, 37, 45, 42, 41],
        135 => &[12, 24, 30, 18],
        136 => &[123, 71],
        137 => &[60, 61, 62],
        138 => &[73, 75, 74],
        139 => &[77, 76, 78],
        140 => &[111, 113, 110, 109],
        141 => &[68, 66, 67],
        142 => &[57, 58, 59],
        143 => &[10, 22, 28, 16],
        160 => &[116, 117, 115],
        _ => &[],
    }
}

/// Combination artifact a component belongs to, if any (reverse of
/// [`components`]; every component belongs to exactly one combination).
pub fn combination_of(component_id: u32) -> Option<u32> {
    HOTA_44_11_ARTIFACTS
        .iter()
        .map(|&(id, _)| id)
        .find(|&id| components(id).contains(&component_id))
}

/// Primary-attribute bonus of one artifact, `(attack, defense, spell power,
/// knowledge)` as stored-stat deltas (h3sed `metadata.ARTIFACT_STATS`).
/// Absent artifacts grant no bonus. The stored attribute bytes include the
/// bonuses of everything currently equipped, so equipping or removing an
/// artifact rewrites the stored bytes by the bonus delta.
pub fn stat_bonus(artifact_id: u32) -> Option<[i8; 4]> {
    Some(match artifact_id {
        7 => [2, 0, 0, 0],
        8 => [3, 0, 0, 0],
        9 => [4, 0, 0, 0],
        10 => [5, 0, 0, 0],
        11 => [6, 0, 0, 0],
        12 => [12, -3, 0, 0],
        13 => [0, 2, 0, 0],
        14 => [0, 3, 0, 0],
        15 => [0, 4, 0, 0],
        16 => [0, 5, 0, 0],
        17 => [0, 6, 0, 0],
        18 => [-3, 12, 0, 0],
        19 => [0, 0, 0, 1],
        20 => [0, 0, 0, 2],
        21 => [0, 0, 0, 3],
        22 => [0, 0, 0, 4],
        23 => [0, 0, 0, 5],
        24 => [0, 0, -2, 10],
        25 => [0, 0, 1, 0],
        26 => [0, 0, 2, 0],
        27 => [0, 0, 3, 0],
        28 => [0, 0, 4, 0],
        29 => [0, 0, 5, 0],
        30 => [0, 0, 10, -2],
        31 => [1, 1, 1, 1],
        32 => [2, 2, 2, 2],
        33 => [3, 3, 3, 3],
        34 => [4, 4, 4, 4],
        35 => [5, 5, 5, 5],
        36 => [6, 6, 6, 6],
        37 => [1, 1, 0, 0],
        38 => [2, 2, 0, 0],
        39 => [3, 3, 0, 0],
        40 => [4, 4, 0, 0],
        41 => [0, 0, 1, 1],
        42 => [0, 0, 2, 2],
        43 => [0, 0, 3, 3],
        44 => [0, 0, 4, 4],
        // Expansion and combination artifacts (h3sed `version/sod.py` and
        // `version/hota.py` ARTIFACT_STATS). Combination values are totals
        // that already include their components.
        128 => [3, 3, 3, 6],
        129 => [21, 21, 21, 21],
        132 => [3, 3, 2, 2],
        134 => [16, 16, 16, 16],
        135 => [9, 9, 8, 8],
        143 => [5, 5, 4, 4],
        147 => [7, 0, 0, 0],
        148 => [0, 7, 0, 0],
        149 => [0, 0, 6, 0],
        150 => [0, 0, 0, 6],
        _ => return None,
    })
}

/// Spells an equipped artifact makes available without being learned
/// (h3sed `metadata.ARTIFACT_SPELLS`, names crossed to binary spell IDs =
/// flag-array indexes). Only ordinary writable artifacts are listed; the
/// learned flags are never touched by equipment edits.
pub fn spell_grants(artifact_id: u32) -> &'static [u8] {
    match artifact_id {
        // Tome of Fire.
        86 => &[
            26, 59, 62, 43, 42, 29, 13, 21, 56, 22, 11, 15, 52, 31, 40, 55, 66, 2,
        ],
        // Tome of Air.
        87 => &[
            28, 19, 58, 25, 8, 4, 47, 6, 51, 53, 60, 17, 15, 36, 44, 30, 69, 5, 2,
        ],
        // Tome of Earth.
        88 => &[
            39, 34, 24, 14, 12, 18, 15, 23, 33, 10, 38, 27, 54, 50, 46, 67, 9, 3, 2,
        ],
        // Tome of Water.
        89 => &[
            41, 65, 37, 35, 61, 20, 16, 15, 49, 48, 32, 64, 1, 0, 68, 63, 2, 7, 45,
        ],
        // Spellbinder's Hat.
        124 => &[8, 6, 18, 36, 40, 69, 67, 66, 68],
        // Armageddon's Blade: Armageddon. Titan's Thunder: Titan's Lightning
        // Bolt (h3sed `version/hota.py` ARTIFACT_SPELLS; HotA drops the SoD
        // Admiral's Hat boat-spell grants).
        128 => &[26],
        135 => &[57],
        _ => &[],
    }
}

/// Spells a map can ban even for artifact grants (h3sed `version/hota.py`
/// `BANNABLE_SPELLS`): Summon Boat, Scuttle Boat, Water Walk. A grant is not
/// written when the stored available flag is 0 for one of these, matching
/// h3sed's conditional rule; removal of a grant is never blocked.
pub const BANNABLE_SPELLS: [u8; 3] = [0, 1, 7];

/// Write set for 44/11: The Grail and every ordinary, expansion, combination
/// and HotA artifact of the catalog. The spellbook (0), scroll item (1) and
/// war machines (3-6) keep their dedicated representations and are never
/// written as plain artifacts; IDs 144-146 are not in the catalog.
pub fn writable(id: u32) -> bool {
    id == 2 || (7..=143).contains(&id) || (147..=165).contains(&id)
}

/// Per-kind reservation counters implied by a set of equipped artifact IDs,
/// indexed by [`EquipmentSlotKind::reservation_index`] (index 0 unused).
pub fn reservation_counts(equipped: impl IntoIterator<Item = u32>) -> [u8; 10] {
    let mut counts = [0u8; 10];
    for id in equipped {
        for kind in reserved_slots(id) {
            counts[kind.reservation_index()] += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combination_tables_are_consistent() {
        for &(id, name) in HOTA_44_11_ARTIFACTS {
            let reserved = reserved_slots(id);
            let parts = components(id);
            assert_eq!(reserved.is_empty(), parts.is_empty(), "{name}");
            if writable(id) && id != 2 {
                assert!(slot_kind(id).is_some(), "{name} needs a wear slot");
            }
            if parts.is_empty() {
                continue;
            }
            // One component per occupied slot, and the component kinds are
            // exactly the primary slot plus the reserved ones.
            assert_eq!(parts.len(), reserved.len() + 1, "{name}");
            let mut wanted: Vec<&str> = std::iter::once(slot_kind(id).unwrap())
                .chain(reserved.iter().copied())
                .map(EquipmentSlotKind::label)
                .collect();
            let mut actual: Vec<&str> = parts
                .iter()
                .map(|&part| slot_kind(part).expect("component is wearable").label())
                .collect();
            wanted.sort_unstable();
            actual.sort_unstable();
            assert_eq!(wanted, actual, "{name}");
            for &part in parts {
                assert_eq!(
                    combination_of(part),
                    Some(id),
                    "{name} owns component {part}"
                );
            }
            for kind in EquipmentSlotKind::ALL {
                let count = reserved.iter().filter(|&&k| k == kind).count()
                    + usize::from(slot_kind(id) == Some(kind));
                assert!(count <= kind.capacity() as usize, "{name}");
            }
        }
    }
}
