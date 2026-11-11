//! Read-only Heroes III / HotA map (`.h3m`) library for H3Forge.
//!
//! Three parts, none of which depends on savegame parsing, Tauri or the
//! frontend:
//!
//! - [`decompress_map`] / [`parse_map`]: a bounded reader for the fields
//!   H3Forge uses - header, player slots, customized heroes, and the hero
//!   and town objects with their coordinates;
//! - [`list_map_files`] / [`read_map_header`]: bounded discovery inside
//!   explicitly given `Maps` folders;
//! - [`match_candidates`]: matching a savegame's retained map header fields
//!   against discovered maps.
//!
//! Maps are reference data describing the scenario before turn one. There
//! is no writer API and nothing here modifies a file.

mod container;
mod discovery;
mod error;
mod model;
mod parse;
mod reader;

pub use container::{MapLimits, decompress_map, decompress_prefix};
pub use discovery::{
    FileStamp, MapCandidate, MatchOutcome, MatchStrength, SaveMapKey, ScanLimits, list_map_files,
    match_candidates, match_strength, read_map, read_map_header,
};
pub use error::MapError;
pub use model::{
    DisposedHero, MainHero, MapArmy, MapBody, MapFormat, MapHeader, MapHero, MapHeroKind, MapTown,
    ParsedMap, PlayerInfo, Position, TownBuildings,
};
pub use parse::{MAX_HOTA_SUBREVISION, parse_header, parse_map};
