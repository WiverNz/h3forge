//! Read-only map model: the subset of an `.h3m` that H3Forge uses.
//!
//! Everything here describes the map **as authored** - the state before
//! turn one. A savegame made from the map is authoritative for anything
//! that can change during play; these values are reference data only.
//! Text is kept as raw bytes (maps use legacy single-byte codepages).

/// Map container format, from the first payload word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapFormat {
    /// Restoration of Erathia (`0x0E`).
    RestorationOfErathia,
    /// Armageddon's Blade (`0x15`).
    ArmageddonsBlade,
    /// Shadow of Death (`0x1C`).
    ShadowOfDeath,
    /// Horn of the Abyss (`0x20`), with its format subrevision.
    HornOfTheAbyss { subrevision: u32 },
    /// In the Wake of Gods (`0x33`): header readable, body not implemented.
    WakeOfGods,
}

impl MapFormat {
    /// Short display label such as `HotA (format 10)`.
    pub fn label(&self) -> String {
        match self {
            MapFormat::RestorationOfErathia => "RoE".into(),
            MapFormat::ArmageddonsBlade => "AB".into(),
            MapFormat::ShadowOfDeath => "SoD".into(),
            MapFormat::HornOfTheAbyss { subrevision } => format!("HotA (format {subrevision})"),
            MapFormat::WakeOfGods => "WoG".into(),
        }
    }
}

/// Tile coordinate; `z` is 0 for the surface and 1 for the underground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

/// Customized main hero declared in a player's header slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainHero {
    pub hero_type: u8,
    /// `None` when the stored portrait byte is `0xFF` (keep the default).
    pub portrait: Option<u8>,
    pub name_raw: Vec<u8>,
}

/// One of the eight player slots of the header.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerInfo {
    pub can_human_play: bool,
    pub can_computer_play: bool,
    /// Position of the main town, when the slot declares one.
    pub main_town: Option<Position>,
    pub generate_hero_at_main_town: bool,
    pub has_random_hero: bool,
    pub main_hero: Option<MainHero>,
    /// `(hero type, name)` pairs for the heroes this player starts with.
    pub hero_names: Vec<(u8, Vec<u8>)>,
    /// Team index when the map defines teams.
    pub team: Option<u8>,
}

impl PlayerInfo {
    pub fn is_playable(&self) -> bool {
        self.can_human_play || self.can_computer_play
    }
}

/// Header entry customizing a hero that is not placed on the map
/// ("disposed hero": available for hire with a custom name/portrait).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisposedHero {
    pub hero_type: u8,
    pub portrait: Option<u8>,
    pub name_raw: Vec<u8>,
    /// Bitmask of the players who may hire the hero.
    pub players: u8,
}

/// Everything before the map options block. Cheap to read: discovery
/// parses only this part of each candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapHeader {
    pub format: MapFormat,
    /// HotA release that wrote the map (`major.minor.patch`), HotA format
    /// 8 and later only.
    pub hota_release: Option<(u32, u32, u32)>,
    /// Edge length in tiles; maps are square.
    pub size: u32,
    pub has_underground: bool,
    pub name_raw: Vec<u8>,
    pub description_raw: Vec<u8>,
    pub difficulty: u8,
    pub level_limit: u8,
    pub players: Vec<PlayerInfo>,
    pub disposed_heroes: Vec<DisposedHero>,
}

/// What kind of adventure-map object declared a hero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapHeroKind {
    /// A hero standing on the map (object class 34).
    Hero,
    /// A hero locked in a prison (object class 62).
    Prison,
    /// A random hero (object class 70); its type is chosen at game start.
    Random,
}

/// A hero object placed on the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapHero {
    pub kind: MapHeroKind,
    /// Stored object position: the bottom-right tile of the object sprite.
    pub anchor: Position,
    /// Hero type ID; `None` for random heroes (stored as `0xFF`).
    pub hero_type: Option<u8>,
    /// Owning player 0-7, `None` for neutral/prison heroes.
    pub owner: Option<u8>,
    /// Map-unique quest identifier (AB and later).
    pub identifier: Option<u32>,
    pub name_raw: Option<Vec<u8>>,
    pub portrait: Option<u8>,
    pub experience: Option<u32>,
}

impl MapHero {
    /// Tile the hero stands on. Hero sprites are anchored one tile to the
    /// right of the hero, so this is `anchor.x - 1`. Convention taken from
    /// the stock hero templates; not yet verified against save coordinates.
    pub fn tile(&self) -> Position {
        Position {
            x: self.anchor.x.saturating_sub(1),
            ..self.anchor
        }
    }
}

/// A town object placed on the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTown {
    /// Stored object position: the bottom-right tile of the object sprite.
    pub anchor: Position,
    /// Faction index from the object template; `None` for a random town.
    pub faction: Option<u32>,
    pub owner: Option<u8>,
    pub identifier: Option<u32>,
    pub name_raw: Option<Vec<u8>>,
    /// Whether the map gives the town a garrison of its own.
    pub has_garrison: bool,
    /// Built/forbidden building bitmasks when the map customizes them.
    pub buildings: Option<TownBuildings>,
    /// Number of timed town events.
    pub event_count: u32,
}

impl MapTown {
    /// Town gate tile (`anchor.x - 2`), by the stock town templates; not
    /// yet verified against save coordinates.
    pub fn entrance(&self) -> Position {
        Position {
            x: self.anchor.x.saturating_sub(2),
            ..self.anchor
        }
    }
}

/// Raw 48-bit building masks of a customized town.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TownBuildings {
    pub built: [u8; 6],
    pub forbidden: [u8; 6],
}

/// Data that requires walking the whole payload.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapBody {
    /// Keymaster Tent sprite anchors, color subtypes and original DEF names.
    pub keymaster_tents: Vec<(Position, u32, Vec<u8>)>,
    /// Authored armies only; never evidence of current savegame quantities.
    pub neutral_armies: Vec<MapArmy>,
    pub heroes: Vec<MapHero>,
    pub towns: Vec<MapTown>,
    /// H3M object ordinal and stored sprite anchor, not a GM2 obelisk ID.
    pub obelisks: Vec<(usize, Position)>,
    /// Authored Grail placement centers and randomization radii. These are
    /// not the current buried Grail position or proof it remains available.
    pub grails: Vec<(Position, u32)>,
    /// Hero types customized in the predefined-heroes block (SoD and later).
    pub predefined_hero_types: Vec<u8>,
    /// Number of object instances on the map.
    pub object_count: usize,
    /// `(object class, count)` in ascending class order.
    pub object_classes: Vec<(u32, usize)>,
    /// Bytes left after the last section; 0 for a cleanly consumed map.
    pub trailing_bytes: usize,
}

/// Initial army of a wandering monster or map garrison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapArmy {
    pub class: u32,
    pub anchor: Position,
    /// Random monster classes have no resolved creature ID.
    pub stacks: Vec<(Option<u32>, u32)>,
}

/// A parsed map: the header always, the body when it could be walked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMap {
    pub header: MapHeader,
    /// `Err` keeps the reason the body is unavailable (unsupported section,
    /// malformed data); the header stays usable either way.
    pub body: Result<MapBody, crate::MapError>,
}
