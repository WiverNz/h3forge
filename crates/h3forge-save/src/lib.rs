//! Save-format and domain library boundary for H3Forge.
//!
//! Heroes III savegame parsing and validated editing: gzip container
//! validation and bounded decompression, format/version detection, map text
//! metadata, heuristic hero record location with decoded domain views, and
//! (H3F-007+) typed hero edit operations planned as checked same-length
//! patches against the retained original payload, recompressed into a valid
//! gzip member. The original compressed bytes and decompressed payload are
//! retained verbatim; nothing here touches the file system.
//!
//! This crate deliberately has no desktop or frontend dependencies.

pub mod artifacts;
mod container;
mod coordinates;
pub mod creatures;
pub mod definitions;
mod detect;
mod dismiss;
mod domain;
mod edit;
mod error;
mod locate;
pub mod neutral;
mod parse;
mod patch;
pub mod players;
pub mod town_state;
pub mod towns;

pub use container::Limits;
pub use detect::{Compatibility, ContainerKind, DetectedFormat, GameFamily};
pub use domain::{
    ArmySlot, BackpackItem, EquipmentGating, EquipmentItem, HeroConfidence, HeroNote, HeroOwner,
    HeroSkill, HeroView, PortraitIdentity, PrimaryAttributes, TownConfidence, TownNote, TownView,
    annotate_duplicates,
};
pub use edit::{BackpackEdit, EquipmentEdit, HeroEdit, TownEdit};
pub use error::{EditError, SaveError};
pub use locate::{LocatedHero, RejectReason};
pub use parse::{MapMetadata, decode_legacy_text};
pub use patch::{BytePatch, apply_patches, compress_gzip};
pub use players::{PlayerEdit, PlayerView};
pub use town_state::{BuildingGroup, BuildingStatus, TownState};
pub use towns::{LocatedTown, TownRejectReason};

use detect::detect;
use locate::locate_heroes;

/// Non-fatal observation about the document as a whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable machine-readable category.
    pub code: DiagnosticCode,
    /// Human-readable explanation.
    pub message: String,
}

/// Categories of document-level diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    /// Map name/description fields were not found in the header window.
    MapMetadataNotFound,
    /// The container is recognized but the version bytes match no researched
    /// family; content stays unread beyond container level.
    UnrecognizedVersion,
    /// The family is recognized but no hero layout with fixture evidence
    /// applies; heroes are not scanned rather than guessed.
    HeroScanNotImplemented,
}

/// An opened save: validated container, detected format, map text and hero
/// views over the retained original bytes. The document itself is immutable;
/// edits are planned as patches and serialized into new bytes, never written
/// back into the receiver.
#[derive(Debug, Clone)]
pub struct SaveDocument {
    original: Vec<u8>,
    payload: Vec<u8>,
    /// Detected container identity and support grading.
    pub format: DetectedFormat,
    /// Map name/description from the payload header.
    pub map: MapMetadata,
    /// Located hero records, in locator order; IDs are indexes into this vec.
    pub heroes: Vec<HeroView>,
    /// Located town records, in locator order; IDs are indexes into this vec
    /// (H3F-013). Empty when the format has no fixture-evidenced layout.
    pub towns: Vec<TownView>,
    pub players: Vec<PlayerView>,
    /// Non-fatal observations; a document with diagnostics is still usable.
    pub diagnostics: Vec<Diagnostic>,
}

impl SaveDocument {
    /// Opens a save from its original compressed bytes with default limits.
    pub fn open(original: Vec<u8>) -> Result<Self, SaveError> {
        Self::open_with_limits(original, Limits::default())
    }

    /// Opens a save with explicit resource limits.
    pub fn open_with_limits(original: Vec<u8>, limits: Limits) -> Result<Self, SaveError> {
        let payload = container::decompress_single_member(&original, limits)?;
        let format = detect(&payload)?;
        let map = parse::parse_map_metadata(&payload);
        let mut diagnostics = Vec::new();
        if !map.is_found() {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::MapMetadataNotFound,
                message: "map name/description not found in the payload header window".into(),
            });
        }
        if format.compatibility == Compatibility::Unsupported {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::UnrecognizedVersion,
                message: format!(
                    "save magic is recognized but version {}/{} matches no researched family",
                    format.major, format.minor
                ),
            });
        }
        let mut heroes = Vec::new();
        let mut towns = Vec::new();
        if format.hero_layout_supported() {
            let located = locate_heroes(&payload);
            let mut views: Vec<HeroView> = located
                .iter()
                .enumerate()
                .map(|(id, hero)| domain::build_hero_view(id, hero, &payload))
                .collect();
            annotate_duplicates(&mut views);
            heroes = views;
            // The towns section sits right before the hero array; locate and
            // decode it with the same heuristic discipline, then link
            // garrisoned/visiting heroes by hero type (never by name).
            let first_hero_start = heroes.first().map(|hero| hero.span.start);
            let located_towns = towns::locate_towns(&payload, first_hero_start);
            let mut town_views: Vec<TownView> = located_towns
                .iter()
                .enumerate()
                .map(|(id, town)| domain::build_town_view(id, town, &payload))
                .collect();
            if format.army_edit_supported() {
                // Building and Mage Guild offsets are verified for exact
                // 44/11 only (H3F-022); other layouts keep `state` empty.
                for town in &mut town_views {
                    town.state = payload
                        .get(town.span.clone())
                        .and_then(town_state::TownState::parse);
                }
            }
            domain::annotate_town_heroes(&mut town_views, &heroes, &payload);
            domain::annotate_town_duplicates(&mut town_views);
            towns = town_views;
        } else if format.compatibility != Compatibility::Unsupported {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::HeroScanNotImplemented,
                message: format!(
                    "hero record scanning has fixture evidence only for the HotA layout; {} save is recognized but not scanned",
                    format
                        .family_title()
                        .unwrap_or("unknown-family")
                ),
            });
        }
        let players = if format.army_edit_supported() {
            players::parse(&payload, &towns, &heroes)
        } else {
            Vec::new()
        };
        Ok(SaveDocument {
            original,
            payload,
            format,
            map,
            heroes,
            towns,
            players,
            diagnostics,
        })
    }

    /// Original compressed bytes exactly as supplied; never modified.
    pub fn original_bytes(&self) -> &[u8] {
        &self.original
    }

    /// Decompressed payload; immutable and complete.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Raw bytes of one located hero record, for inspection by future
    /// read-only milestones. Returns `None` for unknown IDs.
    pub fn hero_record_bytes(&self, id: usize) -> Option<&[u8]> {
        let hero = self.heroes.get(id)?;
        self.payload.get(hero.span.clone())
    }

    /// Raw bytes of one located town record. Returns `None` for unknown IDs.
    pub fn town_record_bytes(&self, id: usize) -> Option<&[u8]> {
        let town = self.towns.get(id)?;
        self.payload.get(town.span.clone())
    }

    /// Heroes that passed every implemented structural check.
    pub fn confirmed_heroes(&self) -> impl Iterator<Item = &HeroView> {
        self.heroes
            .iter()
            .filter(|hero| hero.confidence == HeroConfidence::High)
    }

    /// Towns that passed every implemented structural check.
    pub fn confirmed_towns(&self) -> impl Iterator<Item = &TownView> {
        self.towns
            .iter()
            .filter(|town| town.confidence == TownConfidence::High)
    }
}
