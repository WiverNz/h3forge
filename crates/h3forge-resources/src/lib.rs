//! Read-only game-resource library for H3Forge.
//!
//! Archive and image decoding for Heroes III / HotA artwork: bounded,
//! read-only LOD directory access, explicit Heroes "PCX" decoding to
//! indexed data or RGBA, portrait catalog indexing, minimal PNG encoding
//! and bounded installation discovery. Adapted from mmarch's MIT-licensed
//! Rust code with provenance and adaptation notes in each module and in
//! `licenses/mmarch-MIT.txt`.
//!
//! This crate deliberately has no desktop or frontend dependencies and no
//! writer API: `LodArchive::read_entry` returns raw decompressed entry
//! bytes, `decode_pcx` is called explicitly, and installation discovery
//! never writes under an installation. Static interface DEF decoding supports formats 0 and 1 (H3F-006).

mod catalog;
mod creature_portraits;
mod def;
mod spell_icons;
pub use artifact_icons::{ARTIFACT_ID_MAX, ArtifactIcons, HotAArtifactIcons};
pub use creature_portraits::{
    CreaturePortraits, HotACreaturePortraits, creature_frame_name, hota_creature_frame_index,
};
pub use def::{DefFile, DefFrame, DefImage};
mod discovery;
mod error;
mod lod;
mod pcx;
mod png;

pub use catalog::{PortraitAsset, PortraitCatalog, PortraitKind};
pub use discovery::{
    ArchiveState, ArchiveStatus, DiscoveryError, Installation, KNOWN_ARCHIVES, discover, validate,
};
pub use error::ResourceError;
pub use lod::{EntryInfo, LodArchive, ResourceLimits};
pub use pcx::{PcxImage, RgbaImage, decode_pcx};
pub use png::encode_png;
pub use spell_icons::{SPELL_ID_MAX, SPELL_ID_MIN, SpellIcons};

mod artifact_icons;
mod hero_portraits;
pub use hero_portraits::{
    HOTA_PORTRAIT_MAX, HOTA_PORTRAIT_MIN, HeroPortraitFile, SOD_PORTRAIT_MAX, decode_p32,
    decode_portrait_image, hero_portrait_file, is_p32,
};
mod hota32;
pub use hota32::Hota32File;
mod skill_icons;
pub use skill_icons::{HotASkillIcons, SkillIcons, hota_skill_frame_index, skill_frame_index};
mod building_icons;
pub use building_icons::{BUILDING_ID_MAX, BuildingIcons, HotABuildingIcons};
mod town_icons;
pub use town_icons::{
    BASE_TOWN_TYPE_MAX, HOTA_TOWN_TYPE_MAX, HotATownIcons, TownIconSize, TownIcons,
    hota_town_icon_frame, town_icon_frame_name,
};

mod resource_icons;
pub use resource_icons::ResourceIcons;

mod player_flags;
pub use player_flags::PlayerFlags;

mod keymaster_icons;
pub use keymaster_icons::KeymasterIcons;
