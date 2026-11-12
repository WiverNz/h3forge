//! Frontend-facing serde DTOs (architecture: ARCHITECTURE.md "Binary, domain,
//! operations and DTOs").
//!
//! DTOs are read-only projections of core views: raw name bytes travel with
//! the lossy decode so the frontend can re-decode with a user-selected legacy
//! encoding, and notes carry human explanations for unsupported or uncertain
//! fields. No raw offsets are accepted from the frontend; hero IDs are
//! indexes into the document's hero list, never names.

use h3forge_resources::ArchiveState;
use h3forge_save::definitions::names;
use h3forge_save::{
    Compatibility, ContainerKind, DetectedFormat, Diagnostic, EditError, HeroConfidence, HeroNote,
    HeroOwner, HeroView, PortraitIdentity, RejectReason, SaveDocument,
};
use serde::{Deserialize, Serialize};

/// Structured command error: stable machine-readable code plus a human
/// diagnostic details. Frontend translates codes/causes and numeric parameters.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<&'static str>,
    pub params: std::collections::BTreeMap<&'static str, usize>,
    pub code: &'static str,
    pub message: String,
}

impl AppErrorDto {
    pub fn no_open_save() -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "no-open-save",
            message: "Open a savegame before requesting heroes.".into(),
        }
    }

    pub fn grant_not_found() -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "grant-not-found",
            message: "No such save grant; pick the file again.".into(),
        }
    }

    pub fn file_too_large(limit: usize) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::from([("limit", limit)]),
            code: "file-too-large",
            message: format!("file is larger than the {limit}-byte limit"),
        }
    }

    pub fn file_read(message: String) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "file-read",
            message: format!("could not read the selected file: {message}"),
        }
    }

    pub fn open_failed(error: &h3forge_save::SaveError) -> Self {
        use h3forge_save::SaveError;
        let cause = match error {
            SaveError::InputTooLarge { .. } | SaveError::PayloadTooLarge { .. } => "limit",
            SaveError::UnknownFormat { .. } | SaveError::NotGzip => "unknown-format",
            SaveError::MultiMemberGzip
            | SaveError::TrailingData { .. }
            | SaveError::GzipUnsupportedMethod { .. }
            | SaveError::GzipUnsupportedFlags { .. } => "unsupported-container",
            _ => "corrupt-input",
        };
        AppErrorDto {
            cause: Some(cause),
            params: std::collections::BTreeMap::new(),
            code: "open-failed",
            message: format!("save could not be opened: {error}"),
        }
    }

    pub fn map_failed(message: String) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "map-failed",
            message,
        }
    }

    pub fn hero_not_found(id: usize) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "hero-not-found",
            message: format!("no hero record with ID {id}"),
        }
    }

    pub fn town_not_found(id: usize) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "town-not-found",
            message: format!("no town record with ID {id}"),
        }
    }

    pub fn state_locked() -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "state-locked",
            message: "application state is unavailable; restart the app".into(),
        }
    }

    pub fn picker_unsupported() -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "picker-unsupported",
            message: "the native file picker is only available on Windows".into(),
        }
    }

    pub fn installation_not_found(error: &h3forge_resources::DiscoveryError) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "installation-not-found",
            message: format!("no game installation available: {error}"),
        }
    }

    pub fn installation_invalid(error: &h3forge_resources::DiscoveryError) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "installation-invalid",
            message: format!("selected directory is not a usable installation: {error}"),
        }
    }

    pub fn no_base_artwork() -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "no-base-artwork",
            message: "no readable H3bitmap.lod in the installation; portraits are unavailable"
                .into(),
        }
    }

    pub fn asset_failed(error: h3forge_resources::ResourceError) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "asset-failed",
            message: format!("game artwork could not be read: {error}"),
        }
    }

    pub fn edit_unsupported_format(format: &DetectedFormat) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "edit-unsupported-format",
            message: format!(
                "editing is not validated for {} {}/{}; this save stays read-only",
                format.family_title().unwrap_or("the recognized"),
                format.major,
                format.minor
            ),
        }
    }

    pub fn hero_not_editable(id: usize) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "hero-not-editable",
            message: format!("hero record {id} failed structural verification and stays read-only"),
        }
    }

    pub fn edit_failed(error: EditError) -> Self {
        let cause = match &error {
            EditError::UnsupportedFormat { .. } => "unsupported-format",
            EditError::RecordNotEditable { .. } | EditError::TownNotEditable { .. } => {
                "unverified-record"
            }
            EditError::HeroNotFound { .. } | EditError::TownNotFound { .. } => "record-not-found",
            EditError::InvalidValue { .. } => "invalid-value",
            EditError::PreimageMismatch { .. } => "stale-preimage",
            _ => "invalid-patch",
        };
        AppErrorDto {
            cause: Some(cause),
            params: std::collections::BTreeMap::new(),
            code: "edit-failed",
            message: format!("edit could not be applied: {error}"),
        }
    }

    pub fn nothing_to_save() -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "nothing-to-save",
            message: "there are no pending edits to save".into(),
        }
    }

    pub fn source_changed(path: &std::path::Path) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "source-changed",
            message: format!(
                "the source file changed on disk since it was opened; re-open {} and retry \
                 (your edits are still pending)",
                path.display()
            ),
        }
    }

    pub fn forbidden_destination(path: &std::path::Path) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "forbidden-destination",
            message: format!(
                "{} resolves into the protected reference games/ or Data/ directory",
                path.display()
            ),
        }
    }

    pub fn same_file(path: &std::path::Path) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "same-file",
            message: format!(
                "{} is the open source file; choose a new file so the original stays untouched",
                path.display()
            ),
        }
    }

    pub fn file_exists(path: &std::path::Path) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "file-exists",
            message: format!(
                "{} already exists; H3Forge never overwrites, pick another name",
                path.display()
            ),
        }
    }

    pub fn backup_failed(path: &std::path::Path, error: &std::io::Error) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "backup-failed",
            message: format!(
                "could not create the recovery backup at {} ({error}); nothing was written",
                path.display()
            ),
        }
    }

    pub fn backup_failed_verify(path: &std::path::Path) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "backup-failed",
            message: format!(
                "recovery backup at {} failed its verification; nothing was written",
                path.display()
            ),
        }
    }

    pub fn write_failed(path: &std::path::Path, error: &std::io::Error) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "write-failed",
            message: format!("could not write {} ({error})", path.display()),
        }
    }

    pub fn write_failed_plain(path: &std::path::Path) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "write-failed",
            message: format!("could not prepare a temporary file for {}", path.display()),
        }
    }

    pub fn validation_failed(reason: &str) -> Self {
        AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "validation-failed",
            message: format!("written file failed validation: {reason}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub platform: &'static str,
    pub architecture: &'static str,
}

/// One-time grant issued by the Rust-owned picker. The frontend can only
/// open the file through this token, never by naming an arbitrary path.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGrantDto {
    pub grant_id: String,
    pub file_name: String,
}

/// Raw text plus the core's lossy decode; the frontend may re-decode `raw`
/// with a user-selected legacy encoding (frontend concern per H3F-002).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDto {
    pub raw: Vec<u8>,
    pub lossy: String,
}

impl TextDto {
    pub fn new(raw: &[u8], lossy: &str) -> Self {
        TextDto {
            raw: raw.to_vec(),
            lossy: lossy.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerDto {
    pub kind: &'static str,
    pub player: Option<u8>,
    pub label: &'static str,
}

impl From<HeroOwner> for OwnerDto {
    fn from(owner: HeroOwner) -> Self {
        match owner {
            HeroOwner::Player(index) => OwnerDto {
                kind: "player",
                player: Some(index),
                label: names::PLAYER_COLORS
                    .get(index as usize)
                    .copied()
                    .unwrap_or("unknown"),
            },
            HeroOwner::Neutral => OwnerDto {
                kind: "neutral",
                player: None,
                label: "Neutral",
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticDto {
    pub code: &'static str,
    pub message: String,
}

impl From<&Diagnostic> for DiagnosticDto {
    fn from(diagnostic: &Diagnostic) -> Self {
        let code = match diagnostic.code {
            h3forge_save::DiagnosticCode::MapMetadataNotFound => "map-metadata-not-found",
            h3forge_save::DiagnosticCode::UnrecognizedVersion => "unrecognized-version",
            h3forge_save::DiagnosticCode::HeroScanNotImplemented => "hero-scan-not-implemented",
        };
        DiagnosticDto {
            code,
            message: diagnostic.message.clone(),
        }
    }
}

/// Document-level banner: format identity, map text and hero statistics.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSummaryDto {
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub save_disabled_reason: Option<String>,
    pub file_name: String,
    pub family_title: Option<&'static str>,
    pub major: u32,
    pub minor: u32,
    pub container_kind: &'static str,
    pub compatibility: &'static str,
    pub heroes_supported: bool,
    pub map_name: Option<TextDto>,
    pub map_description: Option<TextDto>,
    pub hero_count: usize,
    pub confirmed_count: usize,
    pub diagnostics: Vec<DiagnosticDto>,
}

impl SaveSummaryDto {
    pub fn from_document(file_name: &str, document: &SaveDocument) -> Self {
        let format = &document.format;
        SaveSummaryDto {
            save_disabled_reason: None,
            file_name: file_name.to_string(),
            family_title: format.family_title(),
            major: format.major,
            minor: format.minor,
            container_kind: match format.container_kind {
                ContainerKind::Regular => "regular",
                ContainerKind::Campaign => "campaign",
                ContainerKind::Chronicles => "chronicles",
            },
            compatibility: match format.compatibility {
                Compatibility::Verified => "verified",
                Compatibility::Readable => "readable",
                Compatibility::Unsupported => "unsupported",
            },
            heroes_supported: format.hero_layout_supported(),
            map_name: document
                .map
                .is_found()
                .then(|| TextDto::new(&document.map.name_raw, &document.map.name_lossy)),
            map_description: document
                .map
                .description_raw
                .as_ref()
                .zip(document.map.description_lossy.as_ref())
                .map(|(raw, lossy)| TextDto::new(raw, lossy)),
            hero_count: document.heroes.len(),
            confirmed_count: document.confirmed_heroes().count(),
            diagnostics: document
                .diagnostics
                .iter()
                .map(DiagnosticDto::from)
                .collect(),
        }
    }
}

/// Compact list-row projection; detail fields stay on `HeroDetailDto`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroListItemDto {
    /// Occupied army IDs in slot order, for the compact list preview.
    pub army_creature_ids: Vec<u32>,
    pub id: usize,
    pub name: TextDto,
    pub owner: OwnerDto,
    pub confidence: &'static str,
    pub level: u8,
    pub has_spellbook: bool,
    pub has_notes: bool,
    pub duplicate_name: bool,
}

impl HeroListItemDto {
    pub fn from_view(hero: &HeroView) -> Self {
        HeroListItemDto {
            army_creature_ids: hero
                .army
                .iter()
                .filter(|slot| !slot.is_empty())
                .map(|slot| slot.creature_id)
                .collect(),
            id: hero.id,
            name: TextDto::new(&hero.name_raw, &hero.name_lossy),
            owner: hero.owner.into(),
            confidence: confidence_label(hero.confidence),
            level: hero.level,
            has_spellbook: hero.has_spellbook,
            has_notes: !hero.notes.is_empty(),
            duplicate_name: hero
                .notes
                .iter()
                .any(|note| matches!(note, HeroNote::DuplicateName { .. })),
        }
    }
}

fn confidence_label(confidence: HeroConfidence) -> &'static str {
    match confidence {
        HeroConfidence::High => "high",
        HeroConfidence::Low => "low",
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryDto {
    pub attack: u8,
    pub defense: u8,
    pub spell_power: u8,
    pub knowledge: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroSkillDto {
    pub skill_id: u8,
    pub name: Option<&'static str>,
    pub mastery: u8,
    pub mastery_name: Option<&'static str>,
    pub slot: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpellDto {
    pub id: usize,
    pub name: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArmySlotDto {
    pub creature_id: u32,
    pub count: u32,
    pub name: Option<&'static str>,
    pub empty: bool,
    pub editable: bool,
}

impl ArmySlotDto {
    pub fn new(slot: &h3forge_save::ArmySlot, supported: bool, editable: bool) -> Self {
        Self {
            creature_id: slot.creature_id,
            count: slot.count,
            name: supported
                .then(|| h3forge_save::creatures::name(slot.creature_id))
                .flatten(),
            empty: slot.is_empty(),
            editable: editable && supported && slot.is_supported(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatureDto {
    pub id: u32,
    pub name: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArmyEditDto {
    pub slot: u8,
    pub creature_id: Option<u32>,
    pub count: u32,
}

/// One labeled equipment slot; `bytes` are the raw 8-byte word (artifact IDs
/// decode in a later milestone, so the UI must explain rather than interpret).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipmentDto {
    pub slot: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroNoteDto {
    pub kind: &'static str,
    pub message: String,
}

/// Full read-only inspection view of one hero record.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroDetailDto {
    pub id: usize,
    /// Current save position, not the original H3M position. No write support.
    pub coordinates: Option<[u16; 3]>,
    pub span_start: usize,
    pub span_end: usize,
    pub name: TextDto,
    pub owner: OwnerDto,
    pub confidence: &'static str,
    pub level: u8,
    pub level_from_experience: u8,
    pub experience: u32,
    pub spell_points: u16,
    pub movement_total: u32,
    pub movement_left: u32,
    pub primary: PrimaryDto,
    pub skills: Vec<HeroSkillDto>,
    pub spells_learned: Vec<SpellDto>,
    pub spells_available: Vec<SpellDto>,
    pub has_spellbook: bool,
    pub army: Vec<ArmySlotDto>,
    pub equipment: Vec<EquipmentDto>,
    /// Flat 64 x 8 original backpack bytes for advanced inspection.
    pub inventory: Vec<u8>,
    /// Decoded backpack view (empty/artifact/scroll/unknown per slot).
    pub backpack: Vec<BackpackItemDto>,
    pub notes: Vec<HeroNoteDto>,
    /// Initial-map reference facts (H3F-021); `None` without a linked map.
    /// Never a substitute for the current values above.
    pub map: Option<crate::map_link::HeroMapInfoDto>,
}

impl HeroDetailDto {
    pub fn from_view(hero: &HeroView) -> Self {
        let flag_ids = |flags: &[u8]| -> Vec<SpellDto> {
            flags
                .iter()
                .enumerate()
                .filter(|(_, flag)| **flag == 1)
                .map(|(id, _)| SpellDto {
                    id,
                    name: names::spell_name(id),
                })
                .collect()
        };
        HeroDetailDto {
            id: hero.id,
            coordinates: None,
            span_start: hero.span.start,
            span_end: hero.span.end,
            name: TextDto::new(&hero.name_raw, &hero.name_lossy),
            owner: hero.owner.into(),
            confidence: confidence_label(hero.confidence),
            level: hero.level,
            level_from_experience: hero.level_from_experience,
            experience: hero.experience,
            spell_points: hero.spell_points,
            movement_total: hero.movement_total,
            movement_left: hero.movement_left,
            primary: PrimaryDto {
                attack: hero.primary.attack,
                defense: hero.primary.defense,
                spell_power: hero.primary.spell_power,
                knowledge: hero.primary.knowledge,
            },
            skills: hero
                .skills
                .iter()
                .map(|skill| HeroSkillDto {
                    skill_id: skill.skill_id,
                    name: names::skill_name(skill.skill_id as usize),
                    mastery: skill.mastery,
                    mastery_name: names::mastery_name(skill.mastery),
                    slot: skill.slot,
                })
                .collect(),
            spells_learned: flag_ids(hero.spells_learned.as_slice()),
            spells_available: flag_ids(hero.spells_available.as_slice()),
            has_spellbook: hero.has_spellbook,
            army: hero
                .army
                .iter()
                .map(|slot| ArmySlotDto::new(slot, false, false))
                .collect(),
            equipment: hero
                .equipment
                .iter()
                .enumerate()
                .map(|(index, bytes)| EquipmentDto {
                    slot: names::EQUIPMENT_SLOT_NAMES
                        .get(index)
                        .copied()
                        .unwrap_or("unknown"),
                    bytes: bytes.to_vec(),
                })
                .collect(),
            inventory: hero.inventory.iter().flatten().copied().collect(),
            backpack: hero
                .backpack
                .iter()
                .enumerate()
                .map(|(slot, item)| BackpackItemDto::from_item(slot as u8, *item))
                .collect(),
            notes: hero.notes.iter().map(note_dto).collect(),
            map: None,
        }
    }
}

fn note_dto(note: &HeroNote) -> HeroNoteDto {
    match note {
        HeroNote::LevelExperienceMismatch {
            stored_level,
            derived_level,
            experience,
        } => HeroNoteDto {
            kind: "level-experience-mismatch",
            message: format!(
                "Stored level {stored_level} differs from level {derived_level} implied by \
                 {experience} experience; scripted maps set levels directly."
            ),
        },
        HeroNote::DuplicateName { other_id } => HeroNoteDto {
            kind: "duplicate-name",
            message: format!(
                "Record #{other_id} has identical name bytes; always select records by ID."
            ),
        },
        HeroNote::Rejected(reason) => reject_note(*reason),
    }
}

fn reject_note(reason: RejectReason) -> HeroNoteDto {
    match reason {
        RejectReason::NameNotStrippable => HeroNoteDto {
            kind: "rejected-name",
            message: "Name bytes failed the printable-content check; this is a low-confidence \
                      candidate, not a confirmed hero."
                .into(),
        },
        RejectReason::EquipmentInvalid => HeroNoteDto {
            kind: "rejected-equipment",
            message: "Equipment section failed the slot grammar check; this is a low-confidence \
                      candidate, not a confirmed hero."
                .into(),
        },
        RejectReason::OwnerOutOfRange => HeroNoteDto {
            kind: "rejected-owner",
            message: "Owner byte is neither a player index (0-7) nor neutral (255); this is a \
                      low-confidence candidate, not a confirmed hero."
                .into(),
        },
    }
}

// ----------------------------------------------------------- H3F-005 DTOs --

/// Installation description for the settings/status area. Read-only; the
/// frontend never receives filesystem write access.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationDto {
    /// Directory the user picked (or would re-pick); may be the parent of
    /// the directory that actually holds the archives.
    pub picked_root: String,
    /// Directory whose archives validated.
    pub archive_root: String,
    pub source: &'static str,
    pub resource_sets: Vec<&'static str>,
    pub has_base_artwork: bool,
    pub archives: Vec<ArchiveStatusDto>,
}

impl InstallationDto {
    pub fn new(picked_root: &std::path::Path, state: &crate::session::InstallationState) -> Self {
        use crate::session::InstallationSource;
        let installation = &state.installation;
        InstallationDto {
            picked_root: picked_root.display().to_string(),
            archive_root: installation.root.display().to_string(),
            source: match state.source {
                InstallationSource::ExeParent => "exe-parent",
                InstallationSource::Development => "development",
                InstallationSource::Stored => "stored",
                InstallationSource::Manual => "manual",
            },
            resource_sets: installation.resource_sets(),
            has_base_artwork: installation.has_base_artwork(),
            archives: installation
                .archives
                .iter()
                .map(|status| ArchiveStatusDto {
                    name: status.name.clone(),
                    state: match &status.state {
                        ArchiveState::Readable { entries } => {
                            ArchiveStateDto::Readable { entries: *entries }
                        }
                        ArchiveState::Unsupported { reason } => ArchiveStateDto::Unsupported {
                            reason: reason.clone(),
                        },
                        ArchiveState::Unreadable { reason } => ArchiveStateDto::Unreadable {
                            reason: reason.clone(),
                        },
                    },
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveStatusDto {
    pub name: String,
    pub state: ArchiveStateDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ArchiveStateDto {
    Readable { entries: usize },
    Unsupported { reason: String },
    Unreadable { reason: String },
}

/// Portrait identity bytes from the hero record, exposed for display/tooltip
/// purposes; the portrait itself is served through [`PortraitDto`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortraitIdentityDto {
    pub hero_type: u8,
    pub hero_class: u8,
    pub portrait_id: u8,
}

impl From<PortraitIdentity> for PortraitIdentityDto {
    fn from(identity: PortraitIdentity) -> Self {
        PortraitIdentityDto {
            hero_type: identity.hero_type,
            hero_class: identity.hero_class,
            portrait_id: identity.portrait_id,
        }
    }
}

/// One portrait image response. `png_base64` is `None` when the portrait
/// could not be resolved; `reason` explains why and the frontend renders a
/// generic placeholder instead.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortraitDto {
    pub resolved: bool,
    pub identity: Option<PortraitIdentityDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub png_base64: Option<String>,
    pub reason: Option<String>,
}

impl PortraitDto {
    pub fn resolved_png(png: &[u8], identity: Option<PortraitIdentity>) -> Self {
        PortraitDto {
            resolved: true,
            identity: identity.map(PortraitIdentityDto::from),
            png_base64: Some(base64_encode(png)),
            reason: None,
        }
    }

    pub fn unresolved(reason: &str, identity: Option<PortraitIdentity>) -> Self {
        PortraitDto {
            resolved: false,
            identity: identity.map(PortraitIdentityDto::from),
            png_base64: None,
            reason: Some(reason.to_string()),
        }
    }
}

/// Standard-alphabet base64 (RFC 4648, no padding variants) - small enough to
/// hand-roll, avoiding a new dependency for one encoder.
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

// ----------------------------------------------------------- H3F-007/008 --

/// One learned-spell toggle inside an edit request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpellToggleDto {
    pub id: u8,
    pub learned: bool,
}

/// Semantic hero edit request from the frontend. Every field is optional;
/// present fields are absolute desired values, never offsets or raw bytes.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeroEditRequestDto {
    pub coordinates: Option<[u16; 3]>,
    pub attack: Option<u8>,
    pub defense: Option<u8>,
    pub spell_power: Option<u8>,
    pub knowledge: Option<u8>,
    pub experience: Option<u32>,
    pub level: Option<u8>,
    pub spell_points: Option<u16>,
    pub spell: Option<SpellToggleDto>,
    pub army: Option<ArmyEditDto>,
    /// Absolute ordered secondary-skill list replacing the current set.
    pub skills: Option<Vec<SkillEditDto>>,
    /// One backpack slot replacement.
    pub backpack_slot: Option<BackpackSlotEditDto>,
    /// Second position for an atomic backpack move; validated in the same batch.
    pub backpack_destination: Option<BackpackSlotEditDto>,
    /// One equipment slot replacement.
    pub equipment_slot: Option<EquipmentSlotEditDto>,
    /// Second equipment slot changed in the same batch (clearing the clicked
    /// slot when a combination artifact is equipped through it).
    pub equipment_second: Option<EquipmentSlotEditDto>,
}

/// One backpack slot edit request: the slot index and its desired content.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackpackSlotEditDto {
    pub slot: u8,
    pub item: BackpackItemEditDto,
}

/// One secondary skill entry of the absolute `skills` request list.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillEditDto {
    pub id: u8,
    pub mastery: u8,
}

/// One selectable secondary skill in the editor dropdowns.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillChoiceDto {
    pub id: u8,
    pub name: Option<&'static str>,
}

/// Decoded content of one equipment slot.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipmentItemDto {
    /// "empty" | "artifact" | "scroll" | "unknown".
    pub kind: &'static str,
    pub artifact_id: Option<u32>,
    pub name: Option<String>,
    pub spell_id: Option<u8>,
    pub spell_name: Option<&'static str>,
}

impl EquipmentItemDto {
    pub fn from_item(item: h3forge_save::EquipmentItem) -> Self {
        match item {
            h3forge_save::EquipmentItem::Empty => EquipmentItemDto {
                kind: "empty",
                artifact_id: None,
                name: None,
                spell_id: None,
                spell_name: None,
            },
            h3forge_save::EquipmentItem::Artifact(id) => EquipmentItemDto {
                kind: "artifact",
                artifact_id: Some(id),
                name: h3forge_save::artifacts::name(id).map(str::to_owned),
                spell_id: None,
                spell_name: None,
            },
            h3forge_save::EquipmentItem::Scroll { spell_id } => EquipmentItemDto {
                kind: "scroll",
                artifact_id: None,
                name: Some("Spell Scroll".into()),
                spell_id: Some(spell_id),
                spell_name: h3forge_save::definitions::names::spell_name(spell_id as usize),
            },
            h3forge_save::EquipmentItem::Unknown => EquipmentItemDto {
                kind: "unknown",
                artifact_id: None,
                name: None,
                spell_id: None,
                spell_name: None,
            },
        }
    }
}

/// One labeled equipment slot of the effective hero state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipmentSlotDto {
    pub slot: u8,
    /// Display label, e.g. "Weapon" or "Side 5".
    pub name: &'static str,
    /// Slot category for artifact compatibility: "helm" | "cloak" | "neck" |
    /// "weapon" | "shield" | "armor" | "hand" | "feet" | "side" for wear
    /// slots, "machine" and "spellbook" for the read-only special slots.
    pub kind: &'static str,
    pub item: EquipmentItemDto,
    /// Name of the worn combination artifact that takes this empty slot.
    pub reserved_by: Option<&'static str>,
    /// Whether this slot accepts equipping/unequipping right now.
    pub editable: bool,
    /// Why this slot is read-only, when it is (unsupported format, record
    /// gating, or an unmodeled occupant).
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub read_only_reason: Option<String>,
}

/// Slot category string for one equipment slot index.
pub fn equipment_kind_label(slot_index: u8) -> &'static str {
    use h3forge_save::artifacts::EquipmentSlotKind;
    match h3forge_save::artifacts::wear_slot_kind(slot_index) {
        Some(EquipmentSlotKind::Helm) => "helm",
        Some(EquipmentSlotKind::Cloak) => "cloak",
        Some(EquipmentSlotKind::Neck) => "neck",
        Some(EquipmentSlotKind::Weapon) => "weapon",
        Some(EquipmentSlotKind::Shield) => "shield",
        Some(EquipmentSlotKind::Armor) => "armor",
        Some(EquipmentSlotKind::Hand) => "hand",
        Some(EquipmentSlotKind::Feet) => "feet",
        Some(EquipmentSlotKind::Side) => "side",
        None if slot_index == h3forge_save::definitions::names::SPELLBOOK_SLOT as u8 => "spellbook",
        None => "machine",
    }
}

/// One equipment slot edit request: the slot index and its desired content.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EquipmentSlotEditDto {
    pub slot: u8,
    pub item: EquipmentItemEditDto,
}

/// One equipment slot replacement request. `kind` selects the variant and
/// `value` carries the artifact ID for the non-empty kind.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum EquipmentItemEditDto {
    Empty,
    Artifact(u32),
}

/// Decoded content of one backpack slot.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackpackItemDto {
    pub slot: u8,
    /// "empty" | "artifact" | "scroll" | "unknown".
    pub kind: &'static str,
    pub artifact_id: Option<u32>,
    pub name: Option<String>,
    pub spell_id: Option<u8>,
    pub spell_name: Option<&'static str>,
    pub editable: bool,
}

impl BackpackItemDto {
    pub fn from_item(slot: u8, item: h3forge_save::BackpackItem) -> Self {
        match item {
            h3forge_save::BackpackItem::Empty => BackpackItemDto {
                slot,
                kind: "empty",
                editable: true,
                artifact_id: None,
                name: None,
                spell_id: None,
                spell_name: None,
            },
            h3forge_save::BackpackItem::Artifact(id) => BackpackItemDto {
                slot,
                kind: "artifact",
                editable: h3forge_save::artifacts::writable(id),
                artifact_id: Some(id),
                name: h3forge_save::artifacts::name(id).map(str::to_owned),
                spell_id: None,
                spell_name: None,
            },
            h3forge_save::BackpackItem::Scroll { spell_id } => BackpackItemDto {
                slot,
                kind: "scroll",
                editable: true,
                artifact_id: None,
                name: Some("Spell Scroll".into()),
                spell_id: Some(spell_id),
                spell_name: h3forge_save::definitions::names::spell_name(spell_id as usize),
            },
            h3forge_save::BackpackItem::Unknown => BackpackItemDto {
                slot,
                kind: "unknown",
                editable: false,
                artifact_id: None,
                name: None,
                spell_id: None,
                spell_name: None,
            },
        }
    }
}

/// One selectable verified artifact in the backpack editor.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactChoiceDto {
    pub id: u32,
    pub name: &'static str,
    /// Equipment slot category for equipping ("helm".."side"), `None` for
    /// inventory-only artifacts (The Grail) and spell scrolls.
    pub equip_kind: Option<&'static str>,
    /// Short verified effect text: primary-stat bonus and/or granted
    /// spells, and for combination artifacts the components and the extra
    /// slots taken; `None` when the artifact has no modeled effects.
    pub effect: Option<String>,
    /// Combination artifact (assembled set such as Power of the Dragon Father).
    pub combination: bool,
    /// Slot kinds a combination artifact takes besides its primary slot
    /// ("helm".."side", repeated per slot); empty for ordinary artifacts.
    pub also_takes: Vec<&'static str>,
    /// Name of the combination artifact this one is a component of.
    pub part_of: Option<&'static str>,
}

/// Builds the effect text for one artifact from the verified tables.
pub fn artifact_effect(id: u32) -> Option<String> {
    use h3forge_save::artifacts;
    use h3forge_save::definitions::names::spell_name;
    let mut parts: Vec<String> = Vec::new();
    if let Some(bonus) = artifacts::stat_bonus(id) {
        let labels = ["Attack", "Defense", "Spell Power", "Knowledge"];
        let text: Vec<String> = bonus
            .iter()
            .zip(labels)
            .filter(|(value, _)| **value != 0)
            .map(|(value, label)| format!("{value:+} {label}"))
            .collect();
        if !text.is_empty() {
            parts.push(text.join(", "));
        }
    }
    let grants = artifacts::spell_grants(id);
    if !grants.is_empty() {
        if grants.len() <= 3 {
            let names: Vec<&str> = grants
                .iter()
                .filter_map(|&spell| spell_name(spell as usize))
                .collect();
            parts.push(format!("Grants: {}", names.join(", ")));
        } else {
            parts.push(format!("Grants {} spells while equipped", grants.len()));
        }
    }
    if let Some(name) = artifacts::combination_of(id).and_then(artifacts::name) {
        parts.push(format!("Part of {name}"));
    }
    let components = artifacts::components(id);
    if !components.is_empty() {
        let names: Vec<&str> = components
            .iter()
            .filter_map(|&part| artifacts::name(part))
            .collect();
        parts.push(format!("Combination of {}", names.join(", ")));
        let mut kinds: Vec<String> = Vec::new();
        for kind in artifacts::EquipmentSlotKind::ALL {
            let count = artifacts::reserved_slots(id)
                .iter()
                .filter(|&&reserved| reserved == kind)
                .count();
            match count {
                0 => {}
                1 => kinds.push(kind.label().to_owned()),
                _ => kinds.push(format!("{count} {}", kind.label())),
            }
        }
        parts.push(format!("Also takes: {}", kinds.join(", ")));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

/// One backpack slot replacement request. `kind` selects the variant;
/// `value` carries the artifact ID or spell ID for the non-empty kinds.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum BackpackItemEditDto {
    Empty,
    Artifact(u32),
    Scroll(u8),
}

impl HeroEditRequestDto {
    /// Converts the request into core edit operations; absent fields
    /// contribute nothing.
    pub fn to_edits(&self) -> Vec<h3forge_save::HeroEdit> {
        use h3forge_save::HeroEdit as Edit;
        let mut edits = Vec::new();
        if let Some(value) = self.coordinates {
            edits.push(Edit::SetCoordinates(value));
        }
        if let Some(value) = self.attack {
            edits.push(Edit::SetAttack(value));
        }
        if let Some(value) = self.defense {
            edits.push(Edit::SetDefense(value));
        }
        if let Some(value) = self.spell_power {
            edits.push(Edit::SetSpellPower(value));
        }
        if let Some(value) = self.knowledge {
            edits.push(Edit::SetKnowledge(value));
        }
        if let Some(value) = self.experience {
            edits.push(Edit::SetExperience(value));
        }
        if let Some(value) = self.level {
            edits.push(Edit::SetLevel(value));
        }
        if let Some(value) = self.spell_points {
            edits.push(Edit::SetSpellPoints(value));
        }
        if let Some(toggle) = &self.spell {
            edits.push(Edit::SetSpellLearned {
                spell_id: toggle.id,
                learned: toggle.learned,
            });
        }
        if let Some(army) = &self.army {
            edits.push(Edit::SetArmySlot {
                slot: army.slot,
                creature_id: army.creature_id,
                count: army.count,
            });
        }
        if let Some(skills) = &self.skills {
            edits.push(Edit::SetSkills {
                skills: skills
                    .iter()
                    .map(|skill| (skill.id, skill.mastery))
                    .collect(),
            });
        }
        for change in self
            .backpack_slot
            .iter()
            .chain(self.backpack_destination.iter())
        {
            let item = match change.item {
                BackpackItemEditDto::Empty => h3forge_save::BackpackEdit::Empty,
                BackpackItemEditDto::Artifact(id) => h3forge_save::BackpackEdit::Artifact(id),
                BackpackItemEditDto::Scroll(spell_id) => {
                    h3forge_save::BackpackEdit::Scroll { spell_id }
                }
            };
            edits.push(Edit::SetBackpackSlot {
                slot: change.slot,
                item,
            });
        }
        for change in self
            .equipment_slot
            .iter()
            .chain(self.equipment_second.iter())
        {
            let item = match change.item {
                EquipmentItemEditDto::Empty => h3forge_save::EquipmentEdit::Empty,
                EquipmentItemEditDto::Artifact(id) => h3forge_save::EquipmentEdit::Artifact(id),
            };
            edits.push(Edit::SetEquipmentSlot {
                slot: change.slot,
                item,
            });
        }
        edits
    }
}

/// Effective editable state of one hero: base values overlaid with pending
/// overrides, the dirty field list and any warnings (e.g. learning a spell
/// without a spellbook).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroEditResponseDto {
    pub coordinates: Option<[u16; 3]>,
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub coordinates_read_only_reason: Option<String>,
    pub id: usize,
    pub editable: bool,
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub read_only_reason: Option<String>,
    pub primary: PrimaryDto,
    pub experience: u32,
    pub level: u8,
    pub spell_points: u16,
    /// Effective learned spells (base flags plus pending overrides).
    pub spells_learned: Vec<SpellDto>,
    /// All toggleable spell choices, names included; slot 69 carries a
    /// `None` name and stays labeled by ID.
    pub spell_choices: Vec<SpellDto>,
    /// Effective secondary skills in display order (pending override applied).
    pub skills: Vec<HeroSkillDto>,
    /// All 30 selectable secondary skills for the editor, by binary ID.
    pub skill_choices: Vec<SkillChoiceDto>,
    /// Why skill editing is unavailable, when it is.
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub skills_read_only_reason: Option<String>,
    /// Effective decoded backpack (base entries plus pending slot overrides).
    pub backpack: Vec<BackpackItemDto>,
    /// Verified selectable artifacts (scrolls and the spellbook excluded).
    pub artifact_choices: Vec<ArtifactChoiceDto>,
    /// Why backpack editing is unavailable, when it is.
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub backpack_read_only_reason: Option<String>,
    /// Effective decoded equipment (base slots plus pending overrides).
    pub equipment: Vec<EquipmentSlotDto>,
    /// Why equipment editing is unavailable, when it is.
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub equipment_read_only_reason: Option<String>,
    /// Equipment gating diagnostics, including unsupported occupants and the
    /// reservation offsets and values.
    pub equipment_limitations: Vec<String>,
    pub army: Vec<ArmySlotDto>,
    pub creature_choices: Vec<CreatureDto>,
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub army_read_only_reason: Option<String>,
    /// Whether this hero is marked for dismissal (H3F-026).
    pub dismissal_pending: bool,
    /// Why the hero cannot be dismissed; `None` when the action is available
    /// or already pending.
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub dismissal_blocked_reason: Option<String>,
    pub dirty_fields: Vec<String>,
    pub warnings: Vec<String>,
}

/// One row of the unsaved-changes summary.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingEditDto {
    /// "hero" | "town".
    pub record_kind: &'static str,
    pub record_id: usize,
    /// Record name with its raw bytes, decoded by the frontend.
    pub record_name: TextDto,
    pub field: String,
    pub before: String,
    pub after: String,
}

// ----------------------------------------------------------- H3F-013 towns --

/// A hero reference for town relationships: opaque ID plus the referenced
/// hero record's name. The raw name bytes travel with the lossy decode so
/// the frontend applies the same legacy-text decoding as in the hero list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroRefDto {
    pub id: usize,
    pub name: TextDto,
}

impl HeroRefDto {
    pub fn from_view(hero: &h3forge_save::HeroView) -> Self {
        HeroRefDto {
            id: hero.id,
            name: TextDto::new(&hero.name_raw, &hero.name_lossy),
        }
    }
}

/// Compact town list-row projection; detail fields stay on `TownDetailDto`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TownListItemDto {
    pub id: usize,
    pub name: TextDto,
    pub owner: OwnerDto,
    pub confidence: &'static str,
    pub town_type: u8,
    pub town_type_name: Option<&'static str>,
    /// Occupied garrison creature IDs in slot order, for the compact preview.
    pub garrison_creature_ids: Vec<u32>,
    /// Whether a garrisoned hero owns the garrison (edits redirect there).
    pub has_garrison_hero: bool,
    pub has_visiting_hero: bool,
    pub has_notes: bool,
    pub duplicate_name: bool,
    /// Whether the Fort is built; selects the fort or village town icon.
    pub has_fort: bool,
}

impl TownListItemDto {
    pub fn from_view(town: &h3forge_save::TownView) -> Self {
        TownListItemDto {
            id: town.id,
            name: TextDto::new(&town.name_raw, &town.name_lossy),
            owner: town.owner.into(),
            confidence: town_confidence_label(town.confidence),
            town_type: town.town_type,
            town_type_name: h3forge_save::definitions::towns::town_type_name(town.town_type),
            garrison_creature_ids: town
                .garrison
                .iter()
                .filter(|slot| !slot.is_empty())
                .map(|slot| slot.creature_id)
                .collect(),
            has_garrison_hero: town.garrison_hero.is_some(),
            has_visiting_hero: town.visiting_hero.is_some(),
            has_notes: !town.notes.is_empty(),
            duplicate_name: town
                .notes
                .iter()
                .any(|note| matches!(note, h3forge_save::TownNote::DuplicateName { .. })),
            has_fort: town_has_fort(town),
        }
    }
}

/// Fort state for the town icon. Records without decoded building state
/// (layouts other than exact 44/11) show the fort picture of their faction.
fn town_has_fort(town: &h3forge_save::TownView) -> bool {
    town.state
        .as_ref()
        .is_none_or(h3forge_save::TownState::has_fort)
}

pub fn town_confidence_label(confidence: h3forge_save::TownConfidence) -> &'static str {
    match confidence {
        h3forge_save::TownConfidence::High => "high",
        h3forge_save::TownConfidence::Low => "low",
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TownNoteDto {
    pub kind: &'static str,
    pub message: String,
}

fn town_note_dto(note: &h3forge_save::TownNote) -> TownNoteDto {
    use h3forge_save::TownNote;
    match note {
        TownNote::Rejected(reason) => match reason {
            h3forge_save::TownRejectReason::GarrisonInconsistent => TownNoteDto {
                kind: "rejected-garrison",
                message: "Garrison ID and count words disagree about emptiness; this is a \
                          low-confidence candidate, not a confirmed town."
                    .into(),
            },
            h3forge_save::TownRejectReason::UnknownCreature => TownNoteDto {
                kind: "rejected-creature",
                message: "A garrison stack carries a creature ID outside the verified 44/11 \
                          table; this is a low-confidence candidate, not a confirmed town."
                    .into(),
            },
        },
        TownNote::DuplicateName { other_id } => TownNoteDto {
            kind: "duplicate-name",
            message: format!(
                "Town #{other_id} has identical name bytes; always select records by ID."
            ),
        },
        TownNote::GarrisonHeroAmbiguous { hero_type } => TownNoteDto {
            kind: "garrison-hero-ambiguous",
            message: format!(
                "The garrisoned hero type {hero_type} matches no single hero record; the \
                 garrison relationship is unresolved and stays read-only."
            ),
        },
        TownNote::GarrisonHeroLocationMismatch { hero_id } => TownNoteDto {
            kind: "garrison-hero-location",
            message: format!(
                "The garrisoned hero record #{hero_id} does not carry this town's \
                 coordinates; the link is shown but flagged."
            ),
        },
        TownNote::UnknownGarrisonCreature { slot } => TownNoteDto {
            kind: "unknown-garrison-creature",
            message: format!(
                "Garrison slot {} carries a creature ID outside the verified 44/11 table; \
                 the stack stays read-only and preserved.",
                *slot + 1
            ),
        },
    }
}

/// Full read-only inspection view of one town record.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TownDetailDto {
    pub id: usize,
    pub span_start: usize,
    pub span_end: usize,
    pub name: TextDto,
    pub owner: OwnerDto,
    pub confidence: &'static str,
    pub town_type: u8,
    pub town_type_name: Option<&'static str>,
    pub x: u8,
    pub y: u8,
    pub z: u8,
    /// Mage guild level 0-5 as stored in the base document.
    pub mage_guild_level: u8,
    /// Whether the Fort is built; selects the fort or village town icon.
    pub has_fort: bool,
    pub garrison: Vec<ArmySlotDto>,
    /// Garrisoned hero owning the garrison (edit that hero instead), if any.
    pub garrisoned_hero: Option<HeroRefDto>,
    /// Visiting hero standing at the town, if any.
    pub visiting_hero: Option<HeroRefDto>,
    pub notes: Vec<TownNoteDto>,
    /// Initial-map reference facts (H3F-021); `None` without a linked map.
    pub map: Option<crate::map_link::TownMapInfoDto>,
}

impl TownDetailDto {
    pub fn from_view(town: &h3forge_save::TownView, heroes: &[h3forge_save::HeroView]) -> Self {
        let hero_ref =
            |id: Option<usize>| id.map(|hero_id| HeroRefDto::from_view(&heroes[hero_id]));
        TownDetailDto {
            id: town.id,
            span_start: town.span.start,
            span_end: town.span.end,
            name: TextDto::new(&town.name_raw, &town.name_lossy),
            owner: town.owner.into(),
            confidence: town_confidence_label(town.confidence),
            town_type: town.town_type,
            town_type_name: h3forge_save::definitions::towns::town_type_name(town.town_type),
            x: town.x,
            y: town.y,
            z: town.z,
            mage_guild_level: town.mage_guild_level,
            has_fort: town_has_fort(town),
            garrison: town
                .garrison
                .iter()
                .map(|slot| ArmySlotDto::new(slot, false, false))
                .collect(),
            garrisoned_hero: hero_ref(town.garrison_hero),
            visiting_hero: hero_ref(town.visiting_hero),
            notes: town.notes.iter().map(town_note_dto).collect(),
            map: None,
        }
    }
}

/// Semantic town edit request from the frontend. Present fields are absolute
/// desired values, never offsets or raw bytes; the garrison is the town's
/// army, so the field keeps the hero editor's `army` shape.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TownEditRequestDto {
    /// One garrison slot replacement.
    pub army: Option<ArmyEditDto>,
    /// Construct or remove one building (H3F-022).
    pub building: Option<BuildingEditDto>,
    /// Desired spell list of one Mage Guild level (H3F-022).
    pub guild: Option<GuildEditDto>,
}

/// Construct (`built: true`) or remove one building by its building ID.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildingEditDto {
    pub id: u8,
    pub built: bool,
}

/// Absolute spell list of one Mage Guild level 1-5, in slot order.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuildEditDto {
    pub level: u8,
    pub spells: Vec<u8>,
}

/// Effective army of a town's garrisoned hero (base stacks plus that hero's
/// pending edits). Edited through the hero's own army operation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GarrisonHeroArmyDto {
    pub hero_id: usize,
    pub slots: Vec<ArmySlotDto>,
    /// Why the hero's army cannot be edited, when it cannot.
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub read_only_reason: Option<String>,
    /// The hero's pending army field labels ("Army slot N").
    pub dirty_fields: Vec<String>,
}

/// One building of a town with its effective and base state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TownBuildingDto {
    pub id: u8,
    pub name: String,
    /// "halls" | "fortifications" | "mageGuild" | "economy" | "dwellings" |
    /// "special" | "other".
    pub group: &'static str,
    /// Effective state (base plus pending edits).
    pub built: bool,
    /// The saved game allows constructing it in this town.
    pub allowed: bool,
    pub editable: bool,
    /// Why it cannot be toggled, when it cannot.
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub lock_reason: Option<String>,
    pub dirty: bool,
}

/// Building state of one town (H3F-022).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TownBuildingsDto {
    /// Town type, for building pictures.
    pub town_type: u8,
    /// Why no building of this town can be edited, when that is the case.
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub read_only_reason: Option<String>,
    pub items: Vec<TownBuildingDto>,
}

/// One Mage Guild level of a town.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuildLevelDto {
    pub level: u8,
    /// The Mage Guild building of this level is built (effective state).
    pub built: bool,
    /// The level can exist in this town (faction limit and map restrictions).
    pub available: bool,
    pub capacity: usize,
    /// Effective spells in slot order.
    pub spells: Vec<SpellDto>,
    pub editable: bool,
    pub dirty: bool,
}

/// A spell the guild editor may offer for one level.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuildSpellChoiceDto {
    pub id: u8,
    pub name: &'static str,
    pub level: u8,
    /// Banned in this town by the saved game; cannot be added.
    pub banned: bool,
}

/// Mage Guild state of one town (H3F-022): the current saved-game spells,
/// never the source map's must/may masks.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TownGuildDto {
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub read_only_reason: Option<String>,
    /// Effective built guild level 0-5.
    pub built_level: u8,
    pub max_level: u8,
    pub has_library: bool,
    pub levels: Vec<GuildLevelDto>,
    pub spell_choices: Vec<GuildSpellChoiceDto>,
}

impl TownEditRequestDto {
    pub fn to_edits(&self) -> Vec<h3forge_save::TownEdit> {
        let mut edits = Vec::new();
        if let Some(army) = &self.army {
            edits.push(h3forge_save::TownEdit::SetGarrisonSlot {
                slot: army.slot,
                creature_id: army.creature_id,
                count: army.count,
            });
        }
        if let Some(guild) = &self.guild {
            edits.push(h3forge_save::TownEdit::SetGuildSpells {
                level: guild.level,
                spells: guild.spells.clone(),
            });
        }
        edits
    }
}

/// Effective editable state of one town: base values overlaid with pending
/// garrison overrides, the dirty field list and the hero relationships.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TownEditResponseDto {
    pub id: usize,
    pub editable: bool,
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub read_only_reason: Option<String>,
    /// Effective garrison (base stacks plus pending overrides).
    pub garrison: Vec<ArmySlotDto>,
    pub creature_choices: Vec<CreatureDto>,
    /// Why garrison editing is unavailable, when it is.
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub garrison_read_only_reason: Option<String>,
    /// The hero whose army owns this garrison; the UI redirects editing there.
    pub garrisoned_hero: Option<HeroRefDto>,
    pub visiting_hero: Option<HeroRefDto>,
    /// The garrisoned hero's own army, which is the town's garrison while
    /// that hero is inside; `None` without a garrisoned hero.
    pub garrison_hero_army: Option<GarrisonHeroArmyDto>,
    /// Building state; `None` when the layout has no verified offsets.
    pub buildings: Option<TownBuildingsDto>,
    /// Mage Guild state; `None` when the layout has no verified offsets.
    pub guild: Option<TownGuildDto>,
    pub dirty_fields: Vec<String>,
    pub warnings: Vec<String>,
}

/// Player state projection; no binary offsets cross the command boundary.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    pub status: &'static str,
    pub status_reason: &'static str,
    pub details: crate::player_details::PlayerDetailsDto,
    pub id: usize,
    pub color: u8,
    pub name: String,
    pub active: bool,
    pub hero_count: u8,
    pub town_count: u8,
    pub original: [i32; 7],
    pub resources: [i32; 7],
    #[serde(serialize_with = "crate::localization::serialize_read_only")]
    pub read_only_reason: Option<String>,
    pub dirty_resources: Vec<u8>,
}
