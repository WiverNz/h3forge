//! Application session state: one open save, its pending edits and the
//! picker grant.
//!
//! The grant indirection keeps the frontend from naming arbitrary paths: a
//! save can be opened only through the Rust-owned picker, and only the most
//! recent grant is valid. Opening is read-only. Edits are semantic values
//! held per hero (never offsets); saving writes a new file through a
//! verified temporary file, leaving the source untouched (H3F-007/008).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use h3forge_resources::{HeroPortraitFile, Installation, LodArchive, encode_png};
use h3forge_save::definitions::names;
use h3forge_save::{
    HeroEdit, HeroView, Limits, SaveDocument, TownEdit, apply_patches, compress_gzip,
};

use crate::dto::{
    AppErrorDto, HeroDetailDto, HeroEditResponseDto, HeroListItemDto, PendingEditDto, PortraitDto,
    SaveGrantDto, SaveSummaryDto,
};

/// A dismissal resets the hero's army and spell points, so pending edits
/// of those same bytes cannot be combined with it. Refused up front instead
/// of failing as overlapping patches at save time.
fn dismissal_conflict(
    session: &SaveSession,
    hero_patches: &[h3forge_save::BytePatch],
) -> Result<(), AppErrorDto> {
    let ids: Vec<usize> = session.pending_dismissals.iter().copied().collect();
    let dismissal = session
        .document
        .plan_hero_dismissals(&ids)
        .map_err(AppErrorDto::edit_failed)?;
    let overlaps = hero_patches.iter().any(|edit| {
        dismissal
            .iter()
            .any(|patch| edit.offset < patch.end() && patch.offset < edit.end())
    });
    if overlaps {
        return Err(AppErrorDto::edit_failed(
            h3forge_save::EditError::InvalidValue {
                field: "hero dismissal",
                reason: "Dismissal resets this hero's army and spell points; revert the pending army or spell point edits of this hero first, or keep the hero.".into(),
            },
        ));
    }
    Ok(())
}

/// Pending-summary and dirty-field label of a hero dismissal (H3F-026).
const DISMISSAL_FIELD: &str = "Dismissal";

/// Mutable application state behind a single mutex (managed by Tauri).
#[derive(Debug, Default)]
pub struct AppState {
    grant: Option<SaveGrant>,
    session: Option<SaveSession>,
    next_grant: u64,
    installation: Option<InstallationState>,
    /// Root the user picked manually; persisted by the caller.
    stored_installation_root: Option<PathBuf>,
    creature_art: Option<(SourceIdentity, h3forge_resources::CreaturePortraits)>,
    hota_creature_art: Option<(SourceIdentity, h3forge_resources::HotACreaturePortraits)>,
    keymaster_icons: Option<(SourceIdentity, h3forge_resources::KeymasterIcons)>,
    player_flags: Option<(SourceIdentity, h3forge_resources::PlayerFlags)>,
    resource_icons: Option<(SourceIdentity, h3forge_resources::ResourceIcons)>,
    spell_icons: Option<(SourceIdentity, h3forge_resources::SpellIcons)>,
    skill_icons: Option<(SourceIdentity, h3forge_resources::SkillIcons)>,
    hota_skill_icons: Option<(SourceIdentity, h3forge_resources::HotASkillIcons)>,
    town_icons: Option<(SourceIdentity, h3forge_resources::TownIcons)>,
    hota_town_icons: Option<(SourceIdentity, h3forge_resources::HotATownIcons)>,
    /// Open archive plus per-faction hall containers, parsed on first use.
    building_icons: Option<(SourceIdentity, LodArchive, h3forge_resources::BuildingIcons)>,
    hota_building_icons: Option<(
        SourceIdentity,
        LodArchive,
        h3forge_resources::HotABuildingIcons,
    )>,
    artifact_icons: Option<(SourceIdentity, h3forge_resources::ArtifactIcons)>,
    hota_artifact_icons: Option<(SourceIdentity, h3forge_resources::HotAArtifactIcons)>,
    /// Map headers remembered between discovery passes (H3F-021).
    map_headers: crate::map_link::MapHeaderCache,
}

#[derive(Debug)]
struct SaveGrant {
    id: String,
    path: PathBuf,
    file_name: String,
}

#[derive(Debug)]
pub struct SaveSession {
    pub file_name: String,
    /// File this session was opened from; watched for external changes.
    pub source_path: PathBuf,
    source_identity: SourceIdentity,
    pub document: SaveDocument,
    /// Pending hero edits keyed by hero ID, then by field; values are the
    /// absolute desired states, and only entries differing from the base
    /// document are kept. The base document is immutable, so replanning is
    /// deterministic and idempotent.
    pending: BTreeMap<usize, BTreeMap<FieldKey, HeroEdit>>,
    /// Pending town garrison edits, keyed like `pending` (H3F-013).
    pending_towns: BTreeMap<usize, BTreeMap<TownFieldKey, TownEdit>>,
    pending_tents: BTreeMap<(u8, u8), bool>,
    /// Pending team moves: player color to zero-based existing team (H3F-030).
    pending_teams: BTreeMap<u8, u8>,
    pending_players: BTreeMap<usize, BTreeMap<u8, i64>>,
    pending_neutral: BTreeMap<usize, BTreeMap<u8, h3forge_save::neutral::NeutralEdit>>,
    /// Heroes marked for dismissal (H3F-026); planned jointly at save time
    /// because heroes of one player share that player's roster bytes.
    pending_dismissals: BTreeSet<usize>,
    /// Link to the source `.h3m`; `None` until discovery has run.
    map_link: Option<crate::map_link::MapLink>,
}

/// Identifies one editable field of one town record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TownFieldKey {
    Garrison(u8),
    Buildings,
    /// Mage Guild level 1-5.
    Guild(u8),
}

impl TownFieldKey {
    fn of(edit: &TownEdit) -> TownFieldKey {
        match edit {
            TownEdit::SetGarrisonSlot { slot, .. } => TownFieldKey::Garrison(*slot),
            TownEdit::SetBuildings { .. } => TownFieldKey::Buildings,
            TownEdit::SetGuildSpells { level, .. } => TownFieldKey::Guild(*level),
        }
    }

    fn label(&self) -> String {
        match self {
            TownFieldKey::Garrison(slot) => format!("Garrison slot {}", slot + 1),
            TownFieldKey::Buildings => "Buildings".into(),
            TownFieldKey::Guild(level) => format!("Mage Guild level {level}"),
        }
    }
}

/// Identifies one editable field of one hero record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FieldKey {
    Coordinates,
    Attack,
    Defense,
    SpellPower,
    Knowledge,
    Experience,
    Level,
    SpellPoints,
    Spell(u8),
    Army(u8),
    Skills,
    Backpack(u8),
    Equipment(u8),
}

impl FieldKey {
    fn of(edit: &HeroEdit) -> FieldKey {
        match edit {
            HeroEdit::SetCoordinates(_) => FieldKey::Coordinates,
            HeroEdit::SetArmySlot { slot, .. } => FieldKey::Army(*slot),
            HeroEdit::SetAttack(_) => FieldKey::Attack,
            HeroEdit::SetDefense(_) => FieldKey::Defense,
            HeroEdit::SetSpellPower(_) => FieldKey::SpellPower,
            HeroEdit::SetKnowledge(_) => FieldKey::Knowledge,
            HeroEdit::SetExperience(_) => FieldKey::Experience,
            HeroEdit::SetLevel(_) => FieldKey::Level,
            HeroEdit::SetSpellPoints(_) => FieldKey::SpellPoints,
            HeroEdit::SetSpellLearned { spell_id, .. } => FieldKey::Spell(*spell_id),
            HeroEdit::SetSkills { .. } => FieldKey::Skills,
            HeroEdit::SetBackpackSlot { slot, .. } => FieldKey::Backpack(*slot),
            HeroEdit::SetEquipmentSlot { slot, .. } => FieldKey::Equipment(*slot),
        }
    }

    fn label(&self) -> String {
        match self {
            FieldKey::Coordinates => "Coordinates".into(),
            FieldKey::Army(slot) => format!("Army slot {}", slot + 1),
            FieldKey::Attack => "Attack".into(),
            FieldKey::Defense => "Defense".into(),
            FieldKey::SpellPower => "Spell Power".into(),
            FieldKey::Knowledge => "Knowledge".into(),
            FieldKey::Experience => "Experience".into(),
            FieldKey::Level => "Level".into(),
            FieldKey::SpellPoints => "Spell points".into(),
            FieldKey::Skills => "Skills".into(),
            FieldKey::Backpack(slot) => format!("Backpack slot {}", slot + 1),
            FieldKey::Equipment(slot) => format!(
                "Equipment ({})",
                names::EQUIPMENT_SLOT_LABELS[*slot as usize]
            ),
            FieldKey::Spell(id) => match names::spell_name(*id as usize) {
                Some(name) => format!("Spell {id} ({name})"),
                None => format!("Spell {id}"),
            },
        }
    }
}

/// How the current installation was chosen, for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationSource {
    ExeParent,
    Development,
    Stored,
    Manual,
}

/// Loaded artwork source: the validated installation plus its open base
/// bitmap archive, the portrait catalog over its directory and a small
/// in-memory portrait cache. Rebuilt whenever the source identity changes.
#[derive(Debug)]
pub struct InstallationState {
    pub source: InstallationSource,
    pub picked_root: PathBuf,
    pub installation: Installation,
    bitmaps: LodArchive,
    /// `HotA.lod`, when the installation has a readable one.
    hota: Option<LodArchive>,
    identity: SourceIdentity,
    /// Encoded PNGs keyed by portrait ID and requested size.
    cache: HashMap<(u16, PortraitSize), Vec<u8>>,
}

/// Archive identity used for cache invalidation: path plus length/mtime.
/// A replaced or modified archive rebuilds the whole state.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceIdentity {
    path: PathBuf,
    len: u64,
    modified_ms: i64,
}

impl SourceIdentity {
    fn capture(path: &Path) -> Option<SourceIdentity> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified_ms = metadata
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as i64;
        Some(SourceIdentity {
            path: path.to_path_buf(),
            len: metadata.len(),
            modified_ms,
        })
    }
}

/// Portraits cached per installation; arbitrary but bounded.
const PORTRAIT_CACHE_CAP: usize = 512;

/// Preferred entry kind for one portrait size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortraitSize {
    Large,
    Small,
}

impl AppState {
    pub fn new() -> Self {
        AppState::default()
    }

    /// Records a picker-selected path as the single openable grant and
    /// returns the token the frontend sends back to open it.
    pub fn issue_grant(&mut self, path: PathBuf) -> SaveGrantDto {
        self.next_grant += 1;
        let id = format!("grant-{}", self.next_grant);
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "savegame".into());
        self.grant = Some(SaveGrant {
            id: id.clone(),
            path,
            file_name: file_name.clone(),
        });
        SaveGrantDto {
            grant_id: id,
            file_name,
        }
    }

    /// Opens the file named by a valid grant, replacing any previous
    /// session. The grant is single-use; a failed open leaves no session.
    pub fn open_granted_save(&mut self, grant_id: &str) -> Result<SaveSummaryDto, AppErrorDto> {
        let Some(grant) = self.grant.take() else {
            return Err(AppErrorDto::grant_not_found());
        };
        if grant.id != grant_id {
            self.grant = Some(grant);
            return Err(AppErrorDto::grant_not_found());
        }
        let source_path = grant
            .path
            .canonicalize()
            .map_err(|e| AppErrorDto::file_read(e.to_string()))?;
        let limits = Limits::default();
        let bytes = read_bounded(&source_path, limits.max_compressed)?;
        let document = SaveDocument::open_with_limits(bytes, limits)
            .map_err(|error| AppErrorDto::open_failed(&error))?;
        let source_identity = SourceIdentity::capture(&source_path)
            .ok_or_else(|| AppErrorDto::file_read("save metadata unavailable".into()))?;
        self.session = Some(SaveSession {
            file_name: grant.file_name,
            source_path,
            source_identity,
            document,
            pending: BTreeMap::new(),
            pending_towns: BTreeMap::new(),
            pending_tents: BTreeMap::new(),
            pending_teams: BTreeMap::new(),
            pending_players: BTreeMap::new(),
            pending_neutral: BTreeMap::new(),
            pending_dismissals: BTreeSet::new(),
            map_link: None,
        });
        self.rescan_maps()?;
        self.summary()
    }

    // ----------------------------------------------------------- map link --

    /// `Maps` folders for the open save: beside the save and inside the
    /// current installation. Installation discovery failures only narrow
    /// the search; they never fail the caller.
    fn map_search_dirs(&mut self) -> Result<Vec<PathBuf>, AppErrorDto> {
        let mut roots: Vec<PathBuf> = Vec::new();
        if let Ok(installation) = self.installation() {
            roots.push(installation.installation.root.clone());
            roots.push(installation.picked_root.clone());
        }
        let session = self.session()?;
        Ok(crate::map_link::search_dirs(&session.source_path, &roots))
    }

    /// Runs bounded map discovery for the open save and stores the result.
    /// Read-only: map files are opened for reading and never changed.
    pub fn rescan_maps(&mut self) -> Result<crate::map_link::MapLinkDto, AppErrorDto> {
        let dirs = self.map_search_dirs()?;
        let link = {
            let session = self
                .session
                .as_ref()
                .ok_or_else(AppErrorDto::no_open_save)?;
            crate::map_link::discover(&session.document, dirs, &mut self.map_headers)
        };
        let session = self.session.as_mut().expect("checked above");
        session.map_link = Some(link);
        self.map_link()
    }

    /// Current link description for the open save.
    pub fn map_link(&self) -> Result<crate::map_link::MapLinkDto, AppErrorDto> {
        let session = self.session()?;
        session
            .map_link
            .as_ref()
            .map(|link| link.to_dto(&session.document))
            .ok_or_else(|| AppErrorDto::map_failed("Map discovery has not run.".into()))
    }

    /// Links a user-chosen `.h3m`. Maps whose header contradicts the save
    /// are refused and the previous link is kept.
    pub fn set_manual_map(
        &mut self,
        path: &Path,
    ) -> Result<crate::map_link::MapLinkDto, AppErrorDto> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        let linked = crate::map_link::link_manual(&session.document, path)
            .map_err(AppErrorDto::map_failed)?;
        match session.map_link.as_mut() {
            Some(link) => {
                link.status = crate::map_link::LinkStatus::Manual;
                link.linked = Some(linked);
            }
            None => {
                session.map_link = Some(crate::map_link::MapLink {
                    status: crate::map_link::LinkStatus::Manual,
                    searched: Vec::new(),
                    scanned_files: 0,
                    candidates: Vec::new(),
                    linked: Some(linked),
                });
            }
        }
        self.map_link()
    }

    /// Links one of the candidates listed by an ambiguous discovery.
    pub fn select_map_candidate(
        &mut self,
        index: usize,
    ) -> Result<crate::map_link::MapLinkDto, AppErrorDto> {
        let path = self
            .session()?
            .map_link
            .as_ref()
            .and_then(|link| link.candidates.get(index).cloned())
            .ok_or_else(|| AppErrorDto::map_failed("Unknown map candidate.".into()))?;
        self.set_manual_map(&path)
    }

    /// Folder the map picker should start in: the linked map's folder, the
    /// first searched `Maps` folder, or the save's folder.
    pub fn map_picker_dir(&self) -> Option<PathBuf> {
        let session = self.session.as_ref()?;
        let link = session.map_link.as_ref();
        link.and_then(|link| link.linked.as_ref())
            .and_then(|linked| linked.path.parent().map(Path::to_path_buf))
            .or_else(|| link.and_then(|link| link.searched.first().cloned()))
            .or_else(|| session.source_path.parent().map(Path::to_path_buf))
    }

    pub fn summary(&self) -> Result<SaveSummaryDto, AppErrorDto> {
        let session = self.session()?;
        let mut summary = SaveSummaryDto::from_document(&session.file_name, &session.document);
        summary.save_disabled_reason = direct_save_capability(&session.source_path)
            .err()
            .map(|e| e.message);
        Ok(summary)
    }

    pub fn list_heroes(&self) -> Result<Vec<HeroListItemDto>, AppErrorDto> {
        let session = self.session()?;
        Ok(session
            .document
            .heroes
            .iter()
            .map(|hero| {
                let mut item = HeroListItemDto::from_view(hero);
                if let Some(pending) = session.pending.get(&hero.id) {
                    let mut army = hero.army;
                    for edit in pending.values() {
                        if let HeroEdit::SetArmySlot {
                            slot,
                            creature_id,
                            count,
                        } = *edit
                        {
                            if let Some(stack) = army.get_mut(slot as usize) {
                                *stack = h3forge_save::ArmySlot {
                                    creature_id: creature_id.unwrap_or(u32::MAX),
                                    count,
                                };
                            }
                        }
                    }
                    item.army_creature_ids = army
                        .iter()
                        .filter(|slot| !slot.is_empty())
                        .map(|slot| slot.creature_id)
                        .collect();
                }
                item
            })
            .collect())
    }

    /// Detail for one record by opaque ID (locator order index). Low
    /// confidence candidates are inspectable but carry explanatory notes.
    pub fn get_hero(&self, id: usize) -> Result<HeroDetailDto, AppErrorDto> {
        let session = self.session()?;
        let hero = session
            .document
            .heroes
            .get(id)
            .ok_or_else(|| AppErrorDto::hero_not_found(id))?;
        let mut detail = HeroDetailDto::from_view(hero);
        detail.coordinates = session.document.hero_coordinates(id);
        detail.map = session
            .map_link
            .as_ref()
            .and_then(|link| link.hero_info(hero));
        Ok(detail)
    }

    // ------------------------------------------------------ hero editing --

    /// Whether the record accepts edits at all (layout + confidence gate).
    fn hero_editable(session: &SaveSession, id: usize) -> Result<(), AppErrorDto> {
        if !session.document.format.hero_layout_supported() {
            return Err(AppErrorDto::edit_unsupported_format(
                &session.document.format,
            ));
        }
        let hero = session
            .document
            .heroes
            .get(id)
            .ok_or_else(|| AppErrorDto::hero_not_found(id))?;
        if hero.confidence != h3forge_save::HeroConfidence::High {
            return Err(AppErrorDto::hero_not_editable(id));
        }
        Ok(())
    }

    /// Applies one batch of semantic edit values to one hero: merges them
    /// into the pending set (dropping values that equal the base document),
    /// re-plans the whole hero against the immutable source payload, and
    /// returns the effective editable state. Nothing is written to disk.
    pub fn apply_hero_edit(
        &mut self,
        id: usize,
        edits: Vec<HeroEdit>,
        warnings: Vec<String>,
    ) -> Result<HeroEditResponseDto, AppErrorDto> {
        Self::hero_editable(self.session()?, id)?;
        let session = self.session.as_mut().expect("checked above");
        let hero = &session.document.heroes[id];
        let mut hero_pending: BTreeMap<FieldKey, HeroEdit> =
            session.pending.get(&id).cloned().unwrap_or_default();
        let mut warnings = warnings;
        for edit in edits {
            // Validate indices/values before comparing with the original view.
            // Equipment is validated as a whole below: whether one slot
            // change is legal depends on the other pending slots (a
            // combination artifact needs slots that pending edits free).
            let edit_plan = if matches!(edit, HeroEdit::SetEquipmentSlot { .. }) {
                Vec::new()
            } else {
                session
                    .document
                    .plan_hero_edits(id, std::slice::from_ref(&edit))
                    .map_err(AppErrorDto::edit_failed)?
            };
            if let HeroEdit::SetSpellLearned {
                spell_id,
                learned: true,
            } = &edit
            {
                // h3sed disables spell changes without a spellbook; H3Forge
                // records the learned flag but warns, since the spell cannot
                // be cast until a spellbook is equipped.
                if !hero.has_spellbook {
                    warnings.push(format!(
                        "This hero has no spellbook: spell {} is recorded as learned but \
                         cannot be cast until a spellbook is equipped.",
                        spell_id
                    ));
                }
            }
            let key = FieldKey::of(&edit);
            let equals_base = match &edit {
                HeroEdit::SetCoordinates(to) => session.document.hero_coordinates(id) == Some(*to),
                HeroEdit::SetArmySlot { .. } => edit_plan.is_empty(),
                HeroEdit::SetAttack(v) => *v == hero.primary.attack,
                HeroEdit::SetDefense(v) => *v == hero.primary.defense,
                HeroEdit::SetSpellPower(v) => *v == hero.primary.spell_power,
                HeroEdit::SetKnowledge(v) => *v == hero.primary.knowledge,
                HeroEdit::SetExperience(v) => *v == hero.experience,
                HeroEdit::SetLevel(v) => *v == hero.level,
                HeroEdit::SetSpellPoints(v) => *v == hero.spell_points,
                HeroEdit::SetSpellLearned { spell_id, learned } => {
                    *learned == (hero.spells_learned[*spell_id as usize] == 1)
                }
                HeroEdit::SetSkills { skills } => {
                    let base: Vec<(u8, u8)> = hero
                        .skills
                        .iter()
                        .map(|skill| (skill.skill_id, skill.mastery))
                        .collect();
                    *skills == base
                }
                HeroEdit::SetBackpackSlot { slot, item } => {
                    use h3forge_save::BackpackEdit as Edit;
                    let desired = match item {
                        Edit::Empty => h3forge_save::BackpackItem::Empty,
                        Edit::Artifact(id) => h3forge_save::BackpackItem::Artifact(*id),
                        Edit::Scroll { spell_id } => h3forge_save::BackpackItem::Scroll {
                            spell_id: *spell_id,
                        },
                    };
                    hero.backpack[*slot as usize] == desired
                }
                HeroEdit::SetEquipmentSlot { slot, item } => {
                    use h3forge_save::EquipmentEdit as Edit;
                    let desired = match item {
                        Edit::Empty => h3forge_save::EquipmentItem::Empty,
                        Edit::Artifact(id) => h3forge_save::EquipmentItem::Artifact(*id),
                    };
                    hero.equipment_items
                        .get(*slot as usize)
                        .is_some_and(|current| *current == desired)
                }
            };
            if equals_base {
                hero_pending.remove(&key);
            } else {
                hero_pending.insert(key, edit);
            }
        }
        // Validate the merged set by planning it against the source payload.
        let planned: Vec<HeroEdit> = hero_pending.values().cloned().collect();
        if let Some(HeroEdit::SetCoordinates(to)) = hero_pending.get(&FieldKey::Coordinates) {
            if session.pending_dismissals.contains(&id) {
                return Err(AppErrorDto::edit_failed(
                    h3forge_save::EditError::InvalidValue {
                        field: "coordinates",
                        reason: "Keep the hero before relocating it.".into(),
                    },
                ));
            }
            if session.pending.iter().any(|(&other, edits)| {
                other != id
                    && edits.get(&FieldKey::Coordinates) == Some(&HeroEdit::SetCoordinates(*to))
            }) {
                return Err(AppErrorDto::edit_failed(
                    h3forge_save::EditError::InvalidValue {
                        field: "coordinates",
                        reason: "Another pending hero relocation occupies this destination.".into(),
                    },
                ));
            }
        }
        let hero_patches = session
            .document
            .plan_hero_edits(id, &planned)
            .map_err(AppErrorDto::edit_failed)?;
        if session.pending_dismissals.contains(&id) {
            dismissal_conflict(session, &hero_patches)?;
        }
        if hero_pending.is_empty() {
            session.pending.remove(&id);
        } else {
            session.pending.insert(id, hero_pending);
        }
        let session = self.session.as_ref().expect("set above");
        hero_edit_response(session, id, warnings)
    }

    /// Marks or unmarks one hero for dismissal (H3F-026). Marking validates
    /// the joint plan of every marked hero against the immutable source, so
    /// an unsupported hero is refused here, not at save time. Nothing is
    /// written to disk.
    pub fn set_hero_dismissal(
        &mut self,
        id: usize,
        dismissed: bool,
    ) -> Result<HeroEditResponseDto, AppErrorDto> {
        Self::hero_editable(self.session()?, id)?;
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        if dismissed {
            if session
                .pending
                .get(&id)
                .is_some_and(|edits| edits.contains_key(&FieldKey::Coordinates))
            {
                return Err(AppErrorDto::edit_failed(
                    h3forge_save::EditError::InvalidValue {
                        field: "coordinates",
                        reason: "Revert the hero relocation before dismissing it.".into(),
                    },
                ));
            }
            let mut ids: Vec<usize> = session.pending_dismissals.iter().copied().collect();
            if !ids.contains(&id) {
                ids.push(id);
                session
                    .document
                    .plan_hero_dismissals(&ids)
                    .map_err(AppErrorDto::edit_failed)?;
                let hero_patches = match session.pending.get(&id) {
                    Some(edits) => session
                        .document
                        .plan_hero_edits(id, &edits.values().cloned().collect::<Vec<_>>())
                        .map_err(AppErrorDto::edit_failed)?,
                    None => Vec::new(),
                };
                session.pending_dismissals.insert(id);
                if let Err(error) = dismissal_conflict(session, &hero_patches) {
                    session.pending_dismissals.remove(&id);
                    return Err(error);
                }
            }
        } else {
            session.pending_dismissals.remove(&id);
        }
        hero_edit_response(session, id, Vec::new())
    }

    /// Effective editable state for one hero (base document plus pending
    /// overrides), for initial UI load of a record.
    pub fn hero_edit_state(&self, id: usize) -> Result<HeroEditResponseDto, AppErrorDto> {
        let session = self.session()?;
        if !session.document.format.hero_layout_supported() {
            return Err(AppErrorDto::edit_unsupported_format(
                &session.document.format,
            ));
        }
        session
            .document
            .heroes
            .get(id)
            .ok_or_else(|| AppErrorDto::hero_not_found(id))?;
        hero_edit_response(session, id, Vec::new())
    }

    /// Human-readable list of every pending edit, for the unsaved-changes
    /// summary.
    pub fn pending_edits(&self) -> Result<Vec<PendingEditDto>, AppErrorDto> {
        let session = self.session()?;
        let mut out = Vec::new();
        for (&hero_id, edits) in &session.pending {
            let hero = &session.document.heroes[hero_id];
            for (key, edit) in edits {
                let (before, after) = if let HeroEdit::SetCoordinates(to) = edit {
                    (
                        format!(
                            "{:?}",
                            session
                                .document
                                .hero_coordinates(hero_id)
                                .unwrap_or_default()
                        ),
                        format!("{to:?}"),
                    )
                } else {
                    edit_display(edit.clone(), hero)
                };
                out.push(PendingEditDto {
                    record_kind: "hero",
                    record_id: hero_id,
                    record_name: crate::dto::TextDto::new(&hero.name_raw, &hero.name_lossy),
                    field: key.label(),
                    before,
                    after,
                });
            }
        }
        for &hero_id in &session.pending_dismissals {
            let hero = &session.document.heroes[hero_id];
            out.push(PendingEditDto {
                record_kind: "hero",
                record_id: hero_id,
                record_name: crate::dto::TextDto::new(&hero.name_raw, &hero.name_lossy),
                field: DISMISSAL_FIELD.into(),
                before: match hero.owner {
                    h3forge_save::HeroOwner::Player(color) => {
                        names::PLAYER_COLORS[color as usize].into()
                    }
                    h3forge_save::HeroOwner::Neutral => "Neutral".into(),
                },
                after: "Dismissed".into(),
            });
        }
        for (&town_id, edits) in &session.pending_towns {
            let town = &session.document.towns[town_id];
            for (key, edit) in edits {
                let (before, after) = town_edit_display(edit.clone(), town);
                out.push(PendingEditDto {
                    record_kind: "town",
                    record_id: town_id,
                    record_name: crate::dto::TextDto::new(&town.name_raw, &town.name_lossy),
                    field: key.label(),
                    before,
                    after,
                });
            }
        }
        for (&id, edits) in &session.pending_players {
            let player = &session.document.players[id];
            let name = names::PLAYER_COLORS[player.color as usize];
            for (&resource, &value) in edits {
                out.push(PendingEditDto {
                    record_kind: "player",
                    record_id: id,
                    record_name: crate::dto::TextDto::new(name.as_bytes(), name),
                    field: h3forge_save::players::RESOURCE_NAMES[resource as usize].into(),
                    before: player.resources[resource as usize].to_string(),
                    after: value.to_string(),
                });
            }
        }
        if let Some(masks) = session.document.keymaster_masks() {
            for (&(player, color), &visited) in &session.pending_tents {
                let name = names::PLAYER_COLORS[player as usize];
                out.push(PendingEditDto {
                    record_kind: "player",
                    record_id: player as usize,
                    record_name: crate::dto::TextDto::new(name.as_bytes(), name),
                    field: format!(
                        "{} Keymaster Tent",
                        h3forge_save::keymaster::TENT_COLORS[color as usize]
                    ),
                    before: (masks[color as usize] & (1 << player) != 0).to_string(),
                    after: visited.to_string(),
                });
            }
        }
        if let Some(table) = session.document.team_table() {
            for (&color, &team) in &session.pending_teams {
                let name = names::PLAYER_COLORS[color as usize];
                out.push(PendingEditDto {
                    record_kind: "player",
                    record_id: color as usize,
                    record_name: crate::dto::TextDto::new(name.as_bytes(), name),
                    field: "Team".into(),
                    before: (table.teams[color as usize] + 1).to_string(),
                    after: (team + 1).to_string(),
                });
            }
        }
        let neutral_base =
            (!session.pending_neutral.is_empty()).then(|| session.document.neutral_armies());
        for (&id, edits) in &session.pending_neutral {
            for (&slot, edit) in edits {
                out.push(PendingEditDto {
                    record_kind: "creature",
                    record_id: id,
                    record_name: crate::dto::TextDto::new(
                        id.to_string().as_bytes(),
                        &id.to_string(),
                    ),
                    field: if neutral_base
                        .as_ref()
                        .is_some_and(|b| b.groups.iter().any(|g| g.id == id && g.class == 54))
                    {
                        "Wandering stack".into()
                    } else {
                        format!("Garrison slot {}", slot + 1)
                    },
                    before: neutral_base
                        .as_ref()
                        .and_then(|b| b.groups.iter().find(|g| g.id == id))
                        .and_then(|g| g.army)
                        .map(|a| {
                            let s = a[slot as usize];
                            if s.is_empty() {
                                "Empty".into()
                            } else {
                                format!(
                                    "{} x {}",
                                    s.count,
                                    h3forge_save::creatures::name(s.creature_id)
                                        .unwrap_or("Unknown")
                                )
                            }
                        })
                        .unwrap_or_else(|| "Unknown".into()),
                    after: edit
                        .creature_id
                        .map(|v| {
                            format!(
                                "{} x {}",
                                edit.count,
                                h3forge_save::creatures::name(v).unwrap_or("Unknown")
                            )
                        })
                        .unwrap_or_else(|| "Empty".into()),
                });
            }
        }
        Ok(out)
    }

    /// Drops every pending edit of one hero; other heroes are untouched.
    pub fn discard_hero_edits(&mut self, id: usize) -> Result<Vec<PendingEditDto>, AppErrorDto> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        session.pending.remove(&id);
        session.pending_dismissals.remove(&id);
        self.pending_edits()
    }

    /// Drops all pending edits across heroes.
    pub fn discard_all_edits(&mut self) -> Result<Vec<PendingEditDto>, AppErrorDto> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        session.pending.clear();
        session.pending_towns.clear();
        session.pending_players.clear();
        session.pending_tents.clear();
        session.pending_teams.clear();
        session.pending_neutral.clear();
        session.pending_dismissals.clear();
        self.pending_edits()
    }

    pub fn list_players(&self) -> Result<Vec<crate::dto::PlayerDto>, AppErrorDto> {
        let session = self.session()?;
        let masks = session.document.keymaster_masks();
        let teams = session.document.team_table();
        Ok(session
            .document
            .players
            .iter()
            .map(|p| {
                let team = teams.filter(|t| t.count > 0 && t.participating[p.color as usize]);
                let current = |t: &h3forge_save::TeamTable| {
                    session
                        .pending_teams
                        .get(&p.color)
                        .copied()
                        .unwrap_or(t.teams[p.color as usize])
                };
                let mut resources = p.resources;
                let pending = session.pending_players.get(&p.id);
                if let Some(edits) = pending {
                    for (&resource, &value) in edits {
                        resources[resource as usize] = value as i32;
                    }
                }
                crate::dto::PlayerDto {
                    team: team.as_ref().map(|t| current(t) + 1),
                    original_team: team.map(|t| t.teams[p.color as usize] + 1),
                    team_count: teams.map_or(0, |t| t.count),
                    // Targets that keep every team populated given the other
                    // pending moves; the current team is always listed.
                    team_options: team
                        .filter(|t| t.editable())
                        .map(|t| {
                            (0..t.count)
                                .filter(|&target| {
                                    let mut moves = session.pending_teams.clone();
                                    moves.insert(p.color, target);
                                    target == current(&t)
                                        || session
                                            .document
                                            .plan_team_edits(&team_edits(&moves))
                                            .is_ok()
                                })
                                .map(|target| target + 1)
                                .collect()
                        })
                        .unwrap_or_default(),
                    team_state: match (teams, team) {
                        (None, _) => "unsupported",
                        (Some(t), _) if !t.editable() => "single",
                        (Some(_), None) => "absent",
                        _ => "editable",
                    },
                    team_dirty: session.pending_teams.contains_key(&p.color),
                    tent_visits: masks.map(|m| {
                        std::array::from_fn(|c| {
                            session
                                .pending_tents
                                .get(&(p.color, c as u8))
                                .copied()
                                .unwrap_or(m[c] & (1 << p.color) != 0)
                        })
                    }),
                    original_tent_visits: masks
                        .map(|m| std::array::from_fn(|c| m[c] & (1 << p.color) != 0)),
                    dirty_tents: session
                        .pending_tents
                        .keys()
                        .filter_map(|&(player, color)| (player == p.color).then_some(color))
                        .collect(),
                    status: p.status.code(),
                    status_reason: p.status_reason,
                    details: crate::player_details::describe(
                        &session.document,
                        session.map_link.as_ref(),
                        p.color,
                    ),
                    id: p.id,
                    color: p.color,
                    name: names::PLAYER_COLORS[p.color as usize].into(),
                    active: p.active,
                    hero_count: p.hero_count,
                    town_count: p.town_count,
                    original: p.resources,
                    resources,
                    read_only_reason: p.read_only_reason.clone(),
                    dirty_resources: pending
                        .map(|e| e.keys().copied().collect())
                        .unwrap_or_default(),
                }
            })
            .collect())
    }

    pub fn list_neutral_armies(&self) -> Result<crate::neutral::NeutralListDto, AppErrorDto> {
        let session = self.session()?;
        let found = session.document.neutral_armies();
        let initial = session
            .map_link
            .as_ref()
            .and_then(|m| m.linked.as_ref())
            .and_then(|m| m.map.body.as_ref().ok())
            .map(|b| &b.neutral_armies);
        let groups = found
            .groups
            .into_iter()
            .map(|g| {
                let pending = session.pending_neutral.get(&g.id);
                let mut army = g.army;
                if let (Some(army), Some(pending)) = (&mut army, pending) {
                    for (&slot, edit) in pending {
                        army[slot as usize] = h3forge_save::ArmySlot {
                            creature_id: edit.creature_id.unwrap_or(u32::MAX),
                            count: edit.count,
                        };
                    }
                }
                let initial_army = initial.and_then(|items| {
                    let mut matching = items.iter().filter(|a| {
                        [a.anchor.x, a.anchor.y, a.anchor.z] == g.anchor
                            && (matches!(a.class, 33 | 219) == (g.class != 54))
                    });
                    let first = matching.next()?;
                    if matching.next().is_some() {
                        return None;
                    }
                    Some(
                        first
                            .stacks
                            .iter()
                            .map(|&(creature_id, count)| crate::neutral::InitialStackDto {
                                creature_id,
                                count,
                                name: creature_id.and_then(h3forge_save::creatures::name),
                            })
                            .collect(),
                    )
                });
                crate::neutral::NeutralDto {
                    id: g.id,
                    kind: if g.class == 54 {
                        "wandering"
                    } else {
                        "garrison"
                    },
                    position: g.position,
                    creature_id: if g.class == 54 {
                        army.map(|a| a[0].creature_id).or(g.creature_id)
                    } else {
                        g.creature_id
                    },
                    allowed_creatures: g.allowed_creatures,
                    name: if g.class == 54 {
                        army.map(|a| a[0].creature_id)
                            .or(g.creature_id)
                            .and_then(h3forge_save::creatures::name)
                    } else {
                        None
                    },
                    army: army.map(|slots| {
                        slots
                            .iter()
                            .take(if g.class == 54 { 1 } else { 7 })
                            .map(|s| {
                                crate::dto::ArmySlotDto::new(s, true, g.read_only_reason.is_none())
                            })
                            .collect()
                    }),
                    owner: g.owner,
                    read_only_reason: g.read_only_reason,
                    dirty_slots: pending
                        .map(|p| p.keys().copied().collect())
                        .unwrap_or_default(),
                    initial_army,
                }
            })
            .collect();
        Ok(crate::neutral::NeutralListDto {
            groups,
            unavailable_reason: found.unavailable_reason,
            choices: h3forge_save::creatures::HOTA_44_11
                .iter()
                .map(|&(id, name)| crate::dto::CreatureDto { id, name })
                .collect(),
            initial_groups: initial.map(Vec::len),
        })
    }

    pub fn apply_neutral_edit(
        &mut self,
        id: usize,
        army: crate::dto::ArmyEditDto,
    ) -> Result<crate::neutral::NeutralListDto, AppErrorDto> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        let edit = h3forge_save::neutral::NeutralEdit {
            slot: army.slot,
            creature_id: army.creature_id,
            count: army.count,
        };
        let patches = session
            .document
            .plan_neutral_edits(id, &[edit])
            .map_err(AppErrorDto::edit_failed)?;
        let pending = session.pending_neutral.entry(id).or_default();
        if patches.is_empty() {
            pending.remove(&edit.slot);
        } else {
            pending.insert(edit.slot, edit);
        }
        if pending.is_empty() {
            session.pending_neutral.remove(&id);
        }
        self.list_neutral_armies()
    }

    pub fn discard_neutral_edits(
        &mut self,
        id: usize,
    ) -> Result<crate::neutral::NeutralListDto, AppErrorDto> {
        self.session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?
            .pending_neutral
            .remove(&id);
        self.list_neutral_armies()
    }

    pub fn apply_tent_edit(
        &mut self,
        id: u8,
        color: u8,
        visited: bool,
    ) -> Result<Vec<crate::dto::PlayerDto>, AppErrorDto> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        let patches = session
            .document
            .plan_tent_edits(&[h3forge_save::TentEdit {
                player: id,
                color,
                visited,
            }])
            .map_err(AppErrorDto::edit_failed)?;
        if patches.is_empty() {
            session.pending_tents.remove(&(id, color));
        } else {
            session.pending_tents.insert((id, color), visited);
        }
        self.list_players()
    }

    /// Moves a player into an existing one-based team as a pending edit.
    /// The whole pending set is replanned so no team can end up empty.
    pub fn apply_team_edit(
        &mut self,
        id: u8,
        team: u8,
    ) -> Result<Vec<crate::dto::PlayerDto>, AppErrorDto> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        let invalid = |reason: &str| {
            AppErrorDto::edit_failed(h3forge_save::EditError::InvalidValue {
                field: "player team",
                reason: reason.into(),
            })
        };
        let table = session
            .document
            .team_table()
            .ok_or_else(|| invalid("Team layout is unverified for this save."))?;
        let target = team
            .checked_sub(1)
            .filter(|_| id < 8)
            .ok_or_else(|| invalid("Player must be 0-7 and team numbers start at 1."))?;
        let mut moves = session.pending_teams.clone();
        moves.insert(id, target);
        // Planned with the other pending moves, so absent players, unknown
        // teams and emptied teams are refused before anything is recorded.
        session
            .document
            .plan_team_edits(&team_edits(&moves))
            .map_err(AppErrorDto::edit_failed)?;
        if table.teams[id as usize] == target {
            moves.remove(&id);
        }
        session.pending_teams = moves;
        self.list_players()
    }

    pub fn apply_player_edit(
        &mut self,
        id: usize,
        resource: u8,
        value: i64,
    ) -> Result<Vec<crate::dto::PlayerDto>, AppErrorDto> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        let edit = h3forge_save::PlayerEdit::SetResource { resource, value };
        let patches = session
            .document
            .plan_player_edits(id, &[edit])
            .map_err(AppErrorDto::edit_failed)?;
        let pending = session.pending_players.entry(id).or_default();
        if patches.is_empty() {
            pending.remove(&resource);
        } else {
            pending.insert(resource, value);
        }
        if pending.is_empty() {
            session.pending_players.remove(&id);
        }
        self.list_players()
    }

    pub fn discard_player_edits(
        &mut self,
        id: usize,
    ) -> Result<Vec<crate::dto::PlayerDto>, AppErrorDto> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        session.pending_players.remove(&id);
        session
            .pending_tents
            .retain(|&(p, _), _| usize::from(p) != id);
        // Reverting one player's team must not leave another team empty.
        let mut moves = session.pending_teams.clone();
        if u8::try_from(id).is_ok_and(|color| moves.remove(&color).is_some()) {
            session
                .document
                .plan_team_edits(&team_edits(&moves))
                .map_err(AppErrorDto::edit_failed)?;
            session.pending_teams = moves;
        }
        self.list_players()
    }

    // ------------------------------------------------------ town editing --

    /// Compact town rows with pending garrison overrides applied.
    pub fn list_towns(&self) -> Result<Vec<crate::dto::TownListItemDto>, AppErrorDto> {
        let session = self.session()?;
        Ok(session
            .document
            .towns
            .iter()
            .map(|town| {
                let mut item = crate::dto::TownListItemDto::from_view(town);
                if let Some(pending) = session.pending_towns.get(&town.id) {
                    let mut garrison = town.garrison;
                    for edit in pending.values().cloned() {
                        let TownEdit::SetGarrisonSlot {
                            slot,
                            creature_id,
                            count,
                        } = edit
                        else {
                            continue;
                        };
                        if let Some(stack) = garrison.get_mut(slot as usize) {
                            *stack = h3forge_save::ArmySlot {
                                creature_id: creature_id.unwrap_or(u32::MAX),
                                count,
                            };
                        }
                    }
                    item.garrison_creature_ids = garrison
                        .iter()
                        .filter(|slot| !slot.is_empty())
                        .map(|slot| slot.creature_id)
                        .collect();
                }
                item
            })
            .collect())
    }

    /// Detail for one town by opaque ID, with initial-map facts attached.
    pub fn get_town(&self, id: usize) -> Result<crate::dto::TownDetailDto, AppErrorDto> {
        let session = self.session()?;
        let town = session
            .document
            .towns
            .get(id)
            .ok_or_else(|| AppErrorDto::town_not_found(id))?;
        let mut detail = crate::dto::TownDetailDto::from_view(town, &session.document.heroes);
        detail.map = session
            .map_link
            .as_ref()
            .and_then(|link| link.town_info(town));
        Ok(detail)
    }

    /// Applies one batch of semantic edits to one town: merges them into the
    /// pending set, re-plans the whole town against the immutable source
    /// payload, and returns the effective editable state. Nothing is written
    /// to disk.
    pub fn apply_town_request(
        &mut self,
        id: usize,
        request: &crate::dto::TownEditRequestDto,
    ) -> Result<crate::dto::TownEditResponseDto, AppErrorDto> {
        let session = self.session()?;
        let mut edits = request.to_edits();
        if let Some(building) = request.building {
            // A toggle becomes an absolute target mask: start from the
            // effective mask (pending target or stored), then add missing
            // prerequisites or remove dependents.
            let town = session
                .document
                .towns
                .get(id)
                .ok_or_else(|| AppErrorDto::town_not_found(id))?;
            let state = town.state.as_ref().ok_or_else(|| {
                AppErrorDto::edit_failed(h3forge_save::EditError::InvalidValue {
                    field: "buildings",
                    reason: "building state is not available for this save layout".into(),
                })
            })?;
            let current = match session
                .pending_towns
                .get(&id)
                .and_then(|pending| pending.get(&TownFieldKey::Buildings))
            {
                Some(TownEdit::SetBuildings { built }) => *built,
                _ => state.built,
            };
            let target = state
                .toggle_building(current, building.id, building.built)
                .map_err(AppErrorDto::edit_failed)?;
            edits.push(TownEdit::SetBuildings { built: target });
        }
        self.apply_town_edit(id, edits)
    }

    /// Merges absolute town edits into the pending set; see
    /// [`AppState::apply_town_request`] for the frontend entry point.
    pub fn apply_town_edit(
        &mut self,
        id: usize,
        edits: Vec<TownEdit>,
    ) -> Result<crate::dto::TownEditResponseDto, AppErrorDto> {
        let session = self.session()?;
        if !session.document.format.army_edit_supported() {
            return Err(AppErrorDto::edit_unsupported_format(
                &session.document.format,
            ));
        }
        session
            .document
            .towns
            .get(id)
            .ok_or_else(|| AppErrorDto::town_not_found(id))?;
        let session = self.session.as_mut().expect("checked above");
        let mut town_pending: BTreeMap<TownFieldKey, TownEdit> =
            session.pending_towns.get(&id).cloned().unwrap_or_default();
        for edit in edits {
            // Validate indices/values before comparing with the original view.
            session
                .document
                .plan_town_edits(id, std::slice::from_ref(&edit))
                .map_err(AppErrorDto::edit_failed)?;
            let key = TownFieldKey::of(&edit);
            // A clear of an already-empty slot plans no patches; any other
            // no-op also plans none, so an empty plan means the desired state
            // equals the base.
            let equals_base = session
                .document
                .plan_town_edits(id, std::slice::from_ref(&edit))
                .map_err(AppErrorDto::edit_failed)?
                .is_empty();
            if equals_base {
                town_pending.remove(&key);
            } else {
                town_pending.insert(key, edit);
            }
        }
        // Validate the merged set by planning it against the source payload.
        let planned: Vec<TownEdit> = town_pending.values().cloned().collect();
        session
            .document
            .plan_town_edits(id, &planned)
            .map_err(AppErrorDto::edit_failed)?;
        if town_pending.is_empty() {
            session.pending_towns.remove(&id);
        } else {
            session.pending_towns.insert(id, town_pending);
        }
        let session = self.session.as_ref().expect("set above");
        town_edit_response(session, id)
    }

    /// Effective editable state for one town (base document plus pending
    /// overrides), for initial UI load of a record.
    pub fn get_town_edit_state(
        &self,
        id: usize,
    ) -> Result<crate::dto::TownEditResponseDto, AppErrorDto> {
        let session = self.session()?;
        if !session.document.format.army_edit_supported() {
            return Err(AppErrorDto::edit_unsupported_format(
                &session.document.format,
            ));
        }
        session
            .document
            .towns
            .get(id)
            .ok_or_else(|| AppErrorDto::town_not_found(id))?;
        town_edit_response(session, id)
    }

    /// Drops every pending edit of one town; other records are untouched.
    pub fn discard_town_edits(&mut self, id: usize) -> Result<Vec<PendingEditDto>, AppErrorDto> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(AppErrorDto::no_open_save)?;
        session.pending_towns.remove(&id);
        self.pending_edits()
    }

    /// Saves the pending edits to a new file chosen by the caller (the
    /// native picker runs in the command layer; this method never receives
    /// an unvalidated frontend path). The write goes through a unique
    /// backup of the source, a verified temporary file and a no-clobber
    /// hard-link commit; the original file is never modified. On success the session
    /// re-bases onto the written file with a clean pending set; on failure
    /// the pending edits and the original are preserved.
    pub fn save_save_as(&mut self, destination: &Path) -> Result<SaveSummaryDto, AppErrorDto> {
        self.write_save(destination, false)
    }

    /// Overwrite only the current source after backup and verified same-directory output.
    pub fn save(&mut self) -> Result<SaveSummaryDto, AppErrorDto> {
        let session = self.session()?;
        if !Self::has_pending_edits(session) {
            return Err(AppErrorDto::nothing_to_save());
        }
        let destination = session.source_path.clone();
        direct_save_capability(&destination)?;
        self.write_save(&destination, true)
    }

    fn has_pending_edits(session: &SaveSession) -> bool {
        !session.pending.is_empty()
            || !session.pending_towns.is_empty()
            || !session.pending_players.is_empty()
            || !session.pending_tents.is_empty()
            || !session.pending_teams.is_empty()
            || !session.pending_neutral.is_empty()
            || !session.pending_dismissals.is_empty()
    }

    fn write_save(
        &mut self,
        destination: &Path,
        overwrite: bool,
    ) -> Result<SaveSummaryDto, AppErrorDto> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(AppErrorDto::no_open_save)?;
        if !Self::has_pending_edits(session) {
            return Err(AppErrorDto::nothing_to_save());
        }
        if overwrite {
            guard_protected(destination)?;
        } else {
            guard_destination(destination, &session.source_path)?;
        }
        // Keep a read handle that denies other writers on Windows while preparing output.
        let mut source_options = std::fs::OpenOptions::new();
        source_options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            source_options.share_mode(1 | 4); // FILE_SHARE_READ | FILE_SHARE_DELETE
        }
        let _source_guard = source_options
            .open(&session.source_path)
            .map_err(|e| AppErrorDto::write_failed(&session.source_path, &e))?;
        verify_source(session)?;

        // Plan every pending edit against the immutable source payload.
        let mut patches = Vec::new();
        for (&hero_id, edits) in &session.pending {
            let planned: Vec<HeroEdit> = edits.values().cloned().collect();
            let mut hero_patches = session
                .document
                .plan_hero_edits(hero_id, &planned)
                .map_err(AppErrorDto::edit_failed)?;
            patches.append(&mut hero_patches);
        }
        for (&town_id, edits) in &session.pending_towns {
            let planned: Vec<TownEdit> = edits.values().cloned().collect();
            let mut town_patches = session
                .document
                .plan_town_edits(town_id, &planned)
                .map_err(AppErrorDto::edit_failed)?;
            patches.append(&mut town_patches);
        }
        for (&id, edits) in &session.pending_players {
            let planned: Vec<_> = edits
                .iter()
                .map(
                    |(&resource, &value)| h3forge_save::PlayerEdit::SetResource { resource, value },
                )
                .collect();
            patches.extend(
                session
                    .document
                    .plan_player_edits(id, &planned)
                    .map_err(AppErrorDto::edit_failed)?,
            );
        }
        if !session.pending_tents.is_empty() {
            let edits: Vec<_> = session
                .pending_tents
                .iter()
                .map(|(&(player, color), &visited)| h3forge_save::TentEdit {
                    player,
                    color,
                    visited,
                })
                .collect();
            patches.extend(
                session
                    .document
                    .plan_tent_edits(&edits)
                    .map_err(AppErrorDto::edit_failed)?,
            );
        }
        if !session.pending_teams.is_empty() {
            patches.extend(
                session
                    .document
                    .plan_team_edits(&team_edits(&session.pending_teams))
                    .map_err(AppErrorDto::edit_failed)?,
            );
        }
        for (&id, edits) in &session.pending_neutral {
            patches.extend(
                session
                    .document
                    .plan_neutral_edits(id, &edits.values().copied().collect::<Vec<_>>())
                    .map_err(AppErrorDto::edit_failed)?,
            );
        }
        if !session.pending_dismissals.is_empty() {
            let ids: Vec<usize> = session.pending_dismissals.iter().copied().collect();
            patches.extend(
                session
                    .document
                    .plan_hero_dismissals(&ids)
                    .map_err(AppErrorDto::edit_failed)?,
            );
        }
        let mut payload = session.document.payload().to_vec();
        apply_patches(&mut payload, &patches).map_err(AppErrorDto::edit_failed)?;
        let output = compress_gzip(&payload);

        if !overwrite && destination.exists() {
            return Err(AppErrorDto::file_exists(destination));
        }

        // Recovery backup of the untouched original compressed source.
        let backup_path = backup_path_for(destination);
        let mut backup = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup_path)
            .map_err(|e| AppErrorDto::backup_failed(&backup_path, &e))?;
        use std::io::Write as _;
        backup
            .write_all(session.document.original_bytes())
            .and_then(|()| backup.sync_all())
            .map_err(|e| AppErrorDto::backup_failed(&backup_path, &e))?;
        drop(backup);
        if std::fs::read(&backup_path).ok().as_deref() != Some(session.document.original_bytes()) {
            return Err(AppErrorDto::backup_failed_verify(&backup_path));
        }

        let temp_path = temp_path_in(destination)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|e| AppErrorDto::write_failed(&temp_path, &e))?;
        let _temp_cleanup = TempCleanup(temp_path.clone());
        file.write_all(&output)
            .and_then(|()| file.sync_all())
            .map_err(|e| AppErrorDto::write_failed(&temp_path, &e))?;
        drop(file);

        // Reopen through the strict reader and compare the payload exactly.
        let verification = (|| -> Result<SaveDocument, AppErrorDto> {
            let bytes = std::fs::read(&temp_path)
                .map_err(|error| AppErrorDto::validation_failed(&error.to_string()))?;
            let reopened = SaveDocument::open_strict(bytes)
                .map_err(|error| AppErrorDto::validation_failed(&error.to_string()))?;
            if reopened.payload() != payload.as_slice() {
                return Err(AppErrorDto::validation_failed(
                    "written payload differs from the planned payload",
                ));
            }
            Ok(reopened)
        })();
        let reopened = match verification {
            Ok(reopened) => reopened,
            Err(error) => {
                let _ = std::fs::remove_file(&temp_path);
                return Err(error);
            }
        };

        // Capture the future baseline before committing, so no fallible read after
        // replacement can leave a successful write with stale pending state.
        let mut source_identity = SourceIdentity::capture(&temp_path)
            .ok_or_else(|| AppErrorDto::file_read("temporary metadata unavailable".into()))?;
        source_identity.path = destination.to_path_buf();
        verify_source(session)?;
        guard_protected(destination)?;
        if overwrite {
            // Rust 1.99 rename uses MoveFileExW with SetFileInformationByHandle fallback.
            // Same-directory replacement; never delete or truncate the target first.
            std::fs::rename(&temp_path, destination)
                .map_err(|e| AppErrorDto::write_failed(destination, &e))?;
        } else {
            // Atomically create a new name; unlike rename this cannot clobber a
            // destination that appears after the initial existence check.
            std::fs::hard_link(&temp_path, destination)
                .map_err(|e| AppErrorDto::write_failed(destination, &e))?;
        }
        let file_name = destination
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "savegame".into());
        let session = self.session.as_mut().expect("session present");
        session.document = reopened;
        session.file_name = file_name;
        session.source_path = destination.to_path_buf();
        session.source_identity = source_identity;
        session.pending.clear();
        session.pending_towns.clear();
        session.pending_players.clear();
        session.pending_tents.clear();
        session.pending_teams.clear();
        session.pending_neutral.clear();
        session.pending_dismissals.clear();
        drop(_source_guard);
        self.summary()
    }

    /// The open session, when there is one (used by the Save As picker to
    /// choose a starting directory and default file name).
    pub fn session_ref(&self) -> Result<&SaveSession, AppErrorDto> {
        self.session()
    }

    fn session(&self) -> Result<&SaveSession, AppErrorDto> {
        self.session.as_ref().ok_or_else(AppErrorDto::no_open_save)
    }

    // ------------------------------------------------------ installation --

    pub fn stored_installation_root(&self) -> Option<&Path> {
        self.stored_installation_root.as_deref()
    }

    pub fn set_stored_installation_root(&mut self, root: PathBuf) {
        self.stored_installation_root = Some(root);
    }

    /// Validates a manually picked root, remembers it as the stored
    /// selection and loads its artwork state.
    pub fn set_manual_installation(
        &mut self,
        root: PathBuf,
    ) -> Result<InstallationSource, AppErrorDto> {
        let installation = h3forge_resources::validate(&root)
            .map_err(|e| AppErrorDto::installation_invalid(&e))?;
        self.stored_installation_root = Some(root.clone());
        self.installation = Some(InstallationState::load(
            installation,
            root,
            InstallationSource::Manual,
        )?);
        Ok(InstallationSource::Manual)
    }

    /// Current installation, auto-discovering on first use. Discovery is
    /// metadata-only and never blocks startup.
    pub fn installation(&mut self) -> Result<&InstallationState, AppErrorDto> {
        if self.installation.is_none() {
            let stored = self.stored_installation_root.clone();
            let discovered = h3forge_resources::discover(stored.as_deref())
                .map_err(|e| AppErrorDto::installation_not_found(&e))?;
            let source = classify_source(&discovered, stored.as_deref());
            let picked = stored.unwrap_or_else(|| discovered.root.clone());
            self.installation = Some(InstallationState::load(discovered, picked, source)?);
        }
        Ok(self.installation.as_ref().expect("checked above"))
    }

    /// Validates the cached artwork state against the archive on disk and
    /// rebuilds it after any source change (cache invalidation).
    fn ensure_fresh_installation(&mut self) -> Result<(), AppErrorDto> {
        let stale = {
            let state = self.installation()?;
            let path = state
                .installation
                .archive_path("H3bitmap.lod")
                .ok_or_else(AppErrorDto::no_base_artwork)?;
            SourceIdentity::capture(&path).as_ref() != Some(&state.identity)
        };
        if stale {
            let stored = self.stored_installation_root.clone();
            let discovered = h3forge_resources::discover(stored.as_deref())
                .map_err(|e| AppErrorDto::installation_not_found(&e))?;
            let source = classify_source(&discovered, stored.as_deref());
            let picked = stored.unwrap_or_else(|| discovered.root.clone());
            self.installation = Some(InstallationState::load(discovered, picked, source)?);
        }
        Ok(())
    }

    // ---------------------------------------------------------- portraits --

    /// Static creature portraits from the standard sprite archive only.
    pub fn creature_portrait(&mut self, id: u32) -> Result<PortraitDto, AppErrorDto> {
        if h3forge_resources::creature_frame_name(id).is_none() {
            // HotA-exclusive IDs (Cove, Factory, neutrals): the full artwork
            // lives in the HotA.lod twcrport D32F container (verified id+2
            // frame mapping; see h3forge-resources creature_portraits.rs).
            let path = self.installation()?.installation.archive_path("HotA.lod");
            let Some(path) = path else {
                return Ok(PortraitDto::unresolved("HotA.lod is unavailable.", None));
            };
            let identity =
                SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
            if self
                .hota_creature_art
                .as_ref()
                .is_none_or(|(old, _)| *old != identity)
            {
                self.hota_creature_art = None;
                let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
                let portraits = h3forge_resources::HotACreaturePortraits::load(&archive)
                    .map_err(AppErrorDto::asset_failed)?;
                self.hota_creature_art = Some((identity, portraits));
            }
            let Some((_, portraits)) = self.hota_creature_art.as_mut() else {
                return Ok(PortraitDto::unresolved(
                    "HotA creature portraits unavailable.",
                    None,
                ));
            };
            return match portraits.png(id) {
                Ok(Some(png)) => Ok(PortraitDto::resolved_png(png, None)),
                Ok(None) => Ok(PortraitDto::unresolved(
                    "No verified HotA portrait for this creature ID.",
                    None,
                )),
                Err(error) => Ok(PortraitDto::unresolved(&error.to_string(), None)),
            };
        }
        let path = self
            .installation()?
            .installation
            .archive_path("H3sprite.lod");
        let Some(path) = path else {
            return Ok(PortraitDto::unresolved(
                "H3sprite.lod is unavailable.",
                None,
            ));
        };
        let identity = SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
        if self
            .creature_art
            .as_ref()
            .is_none_or(|(old, _)| *old != identity)
        {
            self.creature_art = None;
            let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
            let portraits = h3forge_resources::CreaturePortraits::load(&archive)
                .map_err(AppErrorDto::asset_failed)?;
            self.creature_art = Some((identity, portraits));
        }
        let Some((_, portraits)) = self.creature_art.as_mut() else {
            return Ok(PortraitDto::unresolved(
                "Creature portraits unavailable.",
                None,
            ));
        };
        match portraits.png(id) {
            Ok(Some(png)) => Ok(PortraitDto::resolved_png(png, None)),
            Ok(None) => Ok(PortraitDto::unresolved(
                "No verified portrait for this creature.",
                None,
            )),
            Err(error) => Ok(PortraitDto::unresolved(&error.to_string(), None)),
        }
    }

    /// Town Hall picture of one building (H3F-022): the nine original
    /// factions from `H3sprite.lod`, Cove, Factory and Bulwark from
    /// `HotA.lod`. Slots a faction does not use, unknown IDs and missing
    /// archives resolve to a placeholder reason, never to other artwork.
    pub fn building_icon(
        &mut self,
        town_type: u8,
        building: u8,
    ) -> Result<PortraitDto, AppErrorDto> {
        let hota = h3forge_resources::HotABuildingIcons::covers(town_type);
        if !hota && !h3forge_resources::BuildingIcons::covers(town_type) {
            return Ok(PortraitDto::unresolved("Unknown town type.", None));
        }
        let archive_name = if hota { "HotA.lod" } else { "H3sprite.lod" };
        let Some(path) = self.installation()?.installation.archive_path(archive_name) else {
            return Ok(PortraitDto::unresolved(
                &format!("{archive_name} is unavailable."),
                None,
            ));
        };
        let identity = SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
        let result = if hota {
            if self
                .hota_building_icons
                .as_ref()
                .is_none_or(|(old, _, _)| *old != identity)
            {
                self.hota_building_icons = None;
                let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
                self.hota_building_icons = Some((
                    identity,
                    archive,
                    h3forge_resources::HotABuildingIcons::new(),
                ));
            }
            let (_, archive, icons) = self.hota_building_icons.as_mut().expect("set above");
            icons.png(archive, town_type, building)
        } else {
            if self
                .building_icons
                .as_ref()
                .is_none_or(|(old, _, _)| *old != identity)
            {
                self.building_icons = None;
                let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
                self.building_icons =
                    Some((identity, archive, h3forge_resources::BuildingIcons::new()));
            }
            let (_, archive, icons) = self.building_icons.as_mut().expect("set above");
            icons.png(archive, town_type, building)
        };
        match result {
            Ok(Some(png)) => Ok(PortraitDto::resolved_png(png, None)),
            Ok(None) => Ok(PortraitDto::unresolved(
                "No picture for this building.",
                None,
            )),
            Err(error) => Ok(PortraitDto::unresolved(&error.to_string(), None)),
        }
    }

    /// Static town icon for one town type (H3F-022). The HotA containers
    /// cover all twelve types and are preferred; the base sprite archive
    /// serves types 0-8 when HotA.lod is unavailable. No other artwork is
    /// ever substituted.
    pub fn town_icon(
        &mut self,
        town_type: u8,
        fort: bool,
        large: bool,
    ) -> Result<PortraitDto, AppErrorDto> {
        use h3forge_resources::TownIconSize;
        let size = if large {
            TownIconSize::Large
        } else {
            TownIconSize::Small
        };
        if let Some(path) = self.installation()?.installation.archive_path("HotA.lod")
            && let Some(identity) = SourceIdentity::capture(&path)
        {
            if self
                .hota_town_icons
                .as_ref()
                .is_none_or(|(old, _)| *old != identity)
            {
                self.hota_town_icons = None;
                // A HotA.lod without the expected containers falls through
                // to the base archive instead of failing the request.
                if let Ok(icons) = LodArchive::open(&path)
                    .and_then(|archive| h3forge_resources::HotATownIcons::load(&archive))
                {
                    self.hota_town_icons = Some((identity, icons));
                }
            }
            if let Some((_, icons)) = self.hota_town_icons.as_mut()
                && let Ok(Some(png)) = icons.png(town_type, fort, size)
            {
                return Ok(PortraitDto::resolved_png(png, None));
            }
        }
        let path = self
            .installation()?
            .installation
            .archive_path("H3sprite.lod");
        let Some(path) = path else {
            return Ok(PortraitDto::unresolved(
                "H3sprite.lod is unavailable.",
                None,
            ));
        };
        let identity = SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
        if self
            .town_icons
            .as_ref()
            .is_none_or(|(old, _)| *old != identity)
        {
            self.town_icons = None;
            let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
            let icons =
                h3forge_resources::TownIcons::load(&archive).map_err(AppErrorDto::asset_failed)?;
            self.town_icons = Some((identity, icons));
        }
        let Some((_, icons)) = self.town_icons.as_mut() else {
            return Ok(PortraitDto::unresolved("Town icons unavailable.", None));
        };
        match icons.png(town_type, fort, size) {
            Ok(Some(png)) => Ok(PortraitDto::resolved_png(png, None)),
            Ok(None) => Ok(PortraitDto::unresolved(
                "No verified icon for this town type.",
                None,
            )),
            Err(error) => Ok(PortraitDto::unresolved(&error.to_string(), None)),
        }
    }

    /// Spell icon for one game spell ID (0-69), from `spells.def` in the
    /// standard sprite archive. The frame position equals the spell ID
    /// (see `h3forge-resources/src/spell_icons.rs` for the evidence); IDs
    /// outside the verified range resolve to a placeholder reason.
    pub fn spell_icon(&mut self, id: u32) -> Result<PortraitDto, AppErrorDto> {
        let path = self
            .installation()?
            .installation
            .archive_path("H3sprite.lod");
        let Some(path) = path else {
            return Ok(PortraitDto::unresolved(
                "H3sprite.lod is unavailable.",
                None,
            ));
        };
        let identity = SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
        if self
            .spell_icons
            .as_ref()
            .is_none_or(|(old, _)| *old != identity)
        {
            self.spell_icons = None;
            let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
            let icons =
                h3forge_resources::SpellIcons::load(&archive).map_err(AppErrorDto::asset_failed)?;
            self.spell_icons = Some((identity, icons));
        }
        let Some((_, icons)) = self.spell_icons.as_mut() else {
            return Ok(PortraitDto::unresolved("Spell icons unavailable.", None));
        };
        match icons.png(id) {
            Ok(Some(png)) => Ok(PortraitDto::resolved_png(png, None)),
            Ok(None) => Ok(PortraitDto::unresolved(
                "No verified icon for this spell ID.",
                None,
            )),
            Err(error) => Ok(PortraitDto::unresolved(&error.to_string(), None)),
        }
    }

    /// Artifact icon for one artifact ID, from `Artifact.def` in the standard
    /// sprite archive. Frames are mapped by their embedded `HrArt###.pcx`
    /// names (see `h3forge-resources/src/artifact_icons.rs`); IDs outside the
    /// verified base/HotA ranges resolve to a placeholder reason.
    pub fn keymaster_icon(&mut self, id: u32) -> Result<PortraitDto, AppErrorDto> {
        let Some(path) = self
            .installation()?
            .installation
            .archive_path("H3sprite.lod")
        else {
            return Ok(PortraitDto::unresolved("H3sprite.lod unavailable", None));
        };
        let identity = SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
        if self
            .keymaster_icons
            .as_ref()
            .is_none_or(|(old, _)| *old != identity)
        {
            self.keymaster_icons = None;
            let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
            self.keymaster_icons = Some((
                identity,
                h3forge_resources::KeymasterIcons::load(&archive)
                    .map_err(AppErrorDto::asset_failed)?,
            ));
        }
        match self
            .keymaster_icons
            .as_mut()
            .and_then(|(_, icons)| icons.png(id).ok().flatten())
        {
            Some(png) => Ok(PortraitDto::resolved_png(png, None)),
            None => Ok(PortraitDto::unresolved(
                "No verified Keymaster Tent icon",
                None,
            )),
        }
    }

    pub fn player_flag(&mut self, id: u32) -> Result<PortraitDto, AppErrorDto> {
        let Some(path) = self
            .installation()?
            .installation
            .archive_path("H3sprite.lod")
        else {
            return Ok(PortraitDto::unresolved("H3sprite.lod unavailable", None));
        };
        let identity = SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
        if self
            .player_flags
            .as_ref()
            .is_none_or(|(old, _)| *old != identity)
        {
            self.player_flags = None;
            let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
            self.player_flags = Some((
                identity,
                h3forge_resources::PlayerFlags::load(&archive)
                    .map_err(AppErrorDto::asset_failed)?,
            ));
        }
        match self
            .player_flags
            .as_mut()
            .and_then(|(_, icons)| icons.png(id).ok().flatten())
        {
            Some(png) => Ok(PortraitDto::resolved_png(png, None)),
            None => Ok(PortraitDto::unresolved("No verified player flag", None)),
        }
    }

    /// Portrait resolved from map-defined hero identity; frontend supplies only player ID.
    pub fn player_starting_portrait(&mut self, id: usize) -> Result<PortraitDto, AppErrorDto> {
        let portrait_id = {
            let session = self.session()?;
            let player = session
                .document
                .players
                .get(id)
                .ok_or_else(|| AppErrorDto {
                    cause: None,
                    params: std::collections::BTreeMap::new(),
                    code: "player-not-found",
                    message: "No such player".into(),
                })?;
            crate::player_details::describe(
                &session.document,
                session.map_link.as_ref(),
                player.color,
            )
            .starting_hero
            .and_then(|h| h.portrait_id)
        };
        let Some(id) = portrait_id else {
            return Ok(PortraitDto::unresolved(
                "Starting hero portrait is not identified",
                None,
            ));
        };
        self.ensure_fresh_installation()?;
        let state = self
            .installation
            .as_mut()
            .ok_or_else(AppErrorDto::no_base_artwork)?;
        let key = (u16::from(id), PortraitSize::Large);
        if let Some(png) = state.cache.get(&key) {
            return Ok(PortraitDto::resolved_png(png, None));
        }
        let Some(file) = h3forge_resources::hero_portrait_file(u16::from(id)) else {
            return Ok(PortraitDto::unresolved(
                "Portrait outside supported tables",
                None,
            ));
        };
        match state.load_portrait_png(&file, PortraitSize::Large) {
            Ok(png) => {
                if state.cache.len() >= PORTRAIT_CACHE_CAP {
                    state.cache.clear();
                }
                state.cache.insert(key, png.clone());
                Ok(PortraitDto::resolved_png(&png, None))
            }
            Err(reason) => Ok(PortraitDto::unresolved(&reason, None)),
        }
    }

    pub fn resource_icon(&mut self, id: u32) -> Result<PortraitDto, AppErrorDto> {
        let Some(path) = self
            .installation()?
            .installation
            .archive_path("H3sprite.lod")
        else {
            return Ok(PortraitDto::unresolved("H3sprite.lod unavailable", None));
        };
        let identity = SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
        if self
            .resource_icons
            .as_ref()
            .is_none_or(|(old, _)| *old != identity)
        {
            self.resource_icons = None;
            let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
            self.resource_icons = Some((
                identity,
                h3forge_resources::ResourceIcons::load(&archive)
                    .map_err(AppErrorDto::asset_failed)?,
            ));
        }
        match self
            .resource_icons
            .as_mut()
            .and_then(|(_, icons)| icons.png(id).ok().flatten())
        {
            Some(png) => Ok(PortraitDto::resolved_png(png, None)),
            None => Ok(PortraitDto::unresolved("No verified resource icon", None)),
        }
    }

    pub fn artifact_icon(&mut self, id: u32) -> Result<PortraitDto, AppErrorDto> {
        if matches!(id, 141..=143 | 147..=165) {
            let Some(path) = self.installation()?.installation.archive_path("HotA.lod") else {
                return Ok(PortraitDto::unresolved(
                    "HotA artwork archive is unavailable.",
                    None,
                ));
            };
            let identity =
                SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
            if self
                .hota_artifact_icons
                .as_ref()
                .is_none_or(|(old, _)| *old != identity)
            {
                self.hota_artifact_icons = None;
                let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
                let icons = h3forge_resources::HotAArtifactIcons::load(&archive)
                    .map_err(AppErrorDto::asset_failed)?;
                self.hota_artifact_icons = Some((identity, icons));
            }
            if let Some((_, icons)) = self.hota_artifact_icons.as_mut() {
                return match icons.png(id) {
                    Ok(Some(png)) => Ok(PortraitDto::resolved_png(png, None)),
                    Ok(None) => Ok(PortraitDto::unresolved(
                        "No verified HotA artifact frame.",
                        None,
                    )),
                    Err(error) => Ok(PortraitDto::unresolved(&error.to_string(), None)),
                };
            }
        }
        let path = self
            .installation()?
            .installation
            .archive_path("H3sprite.lod");
        let Some(path) = path else {
            return Ok(PortraitDto::unresolved(
                "H3sprite.lod is unavailable.",
                None,
            ));
        };
        let identity = SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
        if self
            .artifact_icons
            .as_ref()
            .is_none_or(|(old, _)| *old != identity)
        {
            self.artifact_icons = None;
            let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
            let icons = h3forge_resources::ArtifactIcons::load(&archive)
                .map_err(AppErrorDto::asset_failed)?;
            self.artifact_icons = Some((identity, icons));
        }
        let Some((_, icons)) = self.artifact_icons.as_mut() else {
            return Ok(PortraitDto::unresolved("Artifact icons unavailable.", None));
        };
        match icons.png(id) {
            Ok(Some(png)) => Ok(PortraitDto::resolved_png(png, None)),
            Ok(None) => Ok(PortraitDto::unresolved(
                "No verified icon for this artifact ID.",
                None,
            )),
            Err(error) => Ok(PortraitDto::unresolved(&error.to_string(), None)),
        }
    }

    /// Mastery-specific secondary-skill artwork; unverified IDs resolve to placeholders.
    pub fn skill_icon(&mut self, id: u32, mastery: u8) -> Result<PortraitDto, AppErrorDto> {
        if matches!(id, 28 | 29) {
            // HotA-only skills: artwork lives in the opaque HotA.lod
            // directory as a D32F container (H3F-014); ids 0-27 stay on the
            // standard secsk32.def path below.
            let path = self.installation()?.installation.archive_path("HotA.lod");
            let Some(path) = path else {
                return Ok(PortraitDto::unresolved("HotA.lod is unavailable.", None));
            };
            let identity =
                SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
            if self
                .hota_skill_icons
                .as_ref()
                .is_none_or(|(old, _)| *old != identity)
            {
                self.hota_skill_icons = None;
                let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
                let icons = h3forge_resources::HotASkillIcons::load(&archive)
                    .map_err(AppErrorDto::asset_failed)?;
                self.hota_skill_icons = Some((identity, icons));
            }
            let Some((_, icons)) = self.hota_skill_icons.as_mut() else {
                return Ok(PortraitDto::unresolved(
                    "HotA skill icons unavailable.",
                    None,
                ));
            };
            return match icons.png(id, mastery) {
                Ok(Some(png)) => Ok(PortraitDto::resolved_png(png, None)),
                Ok(None) => Ok(PortraitDto::unresolved(
                    "No verified icon for this skill ID and mastery.",
                    None,
                )),
                Err(error) => Ok(PortraitDto::unresolved(&error.to_string(), None)),
            };
        }
        let path = self
            .installation()?
            .installation
            .archive_path("H3sprite.lod");
        let Some(path) = path else {
            return Ok(PortraitDto::unresolved(
                "H3sprite.lod is unavailable.",
                None,
            ));
        };
        let identity = SourceIdentity::capture(&path).ok_or_else(AppErrorDto::no_base_artwork)?;
        if self
            .skill_icons
            .as_ref()
            .is_none_or(|(old, _)| *old != identity)
        {
            self.skill_icons = None;
            let archive = LodArchive::open(&path).map_err(AppErrorDto::asset_failed)?;
            let icons =
                h3forge_resources::SkillIcons::load(&archive).map_err(AppErrorDto::asset_failed)?;
            self.skill_icons = Some((identity, icons));
        }
        let Some((_, icons)) = self.skill_icons.as_mut() else {
            return Ok(PortraitDto::unresolved("Skill icons unavailable.", None));
        };
        match icons.png(id, mastery) {
            Ok(Some(png)) => Ok(PortraitDto::resolved_png(png, None)),
            Ok(None) => Ok(PortraitDto::unresolved(
                "No verified icon for this skill ID and mastery.",
                None,
            )),
            Err(error) => Ok(PortraitDto::unresolved(&error.to_string(), None)),
        }
    }

    /// PNG for one hero's portrait at the requested size. Unresolved or
    /// missing portraits return `resolved: false` with a human reason; the
    /// frontend renders its own placeholder (no artwork is invented here).
    pub fn hero_portrait(
        &mut self,
        id: usize,
        size: PortraitSize,
    ) -> Result<PortraitDto, AppErrorDto> {
        let portrait = {
            let session = self.session()?;
            session
                .document
                .heroes
                .get(id)
                .ok_or_else(|| AppErrorDto::hero_not_found(id))?
                .portrait
        };
        let Some(identity) = portrait else {
            return Ok(PortraitDto::unresolved(
                "record is too short to carry identity bytes",
                None,
            ));
        };
        self.ensure_fresh_installation()?;
        let state = self.installation.as_mut().expect("installation loaded");
        let portrait_id = identity.portrait_id as u16;
        if let Some(png) = state.cache.get(&(portrait_id, size)) {
            return Ok(PortraitDto::resolved_png(png, Some(identity)));
        }
        let Some(file) = h3forge_resources::hero_portrait_file(portrait_id) else {
            return Ok(PortraitDto::unresolved(
                &format!(
                    "portrait {portrait_id} is outside the known portrait tables (0-{})",
                    h3forge_resources::HOTA_PORTRAIT_MAX
                ),
                Some(identity),
            ));
        };
        let png = match state.load_portrait_png(&file, size) {
            Ok(png) => png,
            // A missing archive or entry is an expected, explainable gap:
            // the placeholder stays and names the resource that is needed.
            Err(reason) => return Ok(PortraitDto::unresolved(&reason, Some(identity))),
        };
        if state.cache.len() >= PORTRAIT_CACHE_CAP {
            state.cache.clear();
        }
        state.cache.insert((portrait_id, size), png.clone());
        Ok(PortraitDto::resolved_png(&png, Some(identity)))
    }
}

/// Effective editable state for one hero: base document values overlaid with
/// the pending overrides, plus the dirty field list for display.
fn hero_edit_response(
    session: &SaveSession,
    id: usize,
    warnings: Vec<String>,
) -> Result<HeroEditResponseDto, AppErrorDto> {
    let hero = &session.document.heroes[id];
    let pending = session.pending.get(&id);
    let get =
        |key: FieldKey| -> Option<HeroEdit> { pending.and_then(|map| map.get(&key).cloned()) };
    let editable = session.document.format.hero_layout_supported()
        && hero.confidence == h3forge_save::HeroConfidence::High;
    let read_only_reason = if editable {
        None
    } else if !session.document.format.hero_layout_supported() {
        Some(format!(
            "the {} {}/{} layout has no validated edit support",
            session
                .document
                .format
                .family_title()
                .unwrap_or("recognized"),
            session.document.format.major,
            session.document.format.minor
        ))
    } else {
        Some("record failed structural verification and stays read-only".into())
    };
    let spell_points = match get(FieldKey::SpellPoints) {
        Some(HeroEdit::SetSpellPoints(value)) => value,
        _ => hero.spell_points,
    };
    let experience = match get(FieldKey::Experience) {
        Some(HeroEdit::SetExperience(value)) => value,
        _ => hero.experience,
    };
    let level = match get(FieldKey::Level) {
        Some(HeroEdit::SetLevel(value)) => value,
        _ => hero.level,
    };
    // Preview the same validated patches Save uses, including equipment bonuses.
    let planned = if let Some(pending) = pending {
        session
            .document
            .plan_hero_edits(id, &pending.values().cloned().collect::<Vec<_>>())
            .map_err(AppErrorDto::edit_failed)?
    } else {
        Vec::new()
    };
    let primary_value = |key: FieldKey, base: u8| {
        use h3forge_save::definitions::hero::{ATTACK, DEFENSE, KNOWLEDGE, SPELL_POWER};
        let offset = hero.span.start
            + match key {
                FieldKey::Attack => ATTACK,
                FieldKey::Defense => DEFENSE,
                FieldKey::SpellPower => SPELL_POWER,
                FieldKey::Knowledge => KNOWLEDGE,
                _ => return base,
            };
        planned
            .iter()
            .find_map(|patch| {
                offset
                    .checked_sub(patch.offset)
                    .and_then(|index| patch.replacement.get(index))
                    .copied()
            })
            .unwrap_or(base)
    };
    // Learned spells: base flags with per-spell overrides applied.
    let mut learned_ids: Vec<usize> = hero
        .spells_learned
        .iter()
        .enumerate()
        .filter(|(_, flag)| **flag == 1)
        .map(|(id, _)| id)
        .collect();
    if let Some(pending) = pending {
        for (key, edit) in pending {
            if let (FieldKey::Spell(spell_id), HeroEdit::SetSpellLearned { learned, .. }) =
                (key, edit)
            {
                let id = *spell_id as usize;
                match (learned, learned_ids.contains(&id)) {
                    (true, false) => learned_ids.push(id),
                    (false, true) => learned_ids.retain(|&existing| existing != id),
                    _ => {}
                }
            }
        }
    }
    learned_ids.sort_unstable();
    let spells_learned = learned_ids
        .iter()
        .map(|&id| crate::dto::SpellDto {
            id,
            name: names::spell_name(id),
        })
        .collect();
    let spell_choices = (0..h3forge_save::definitions::hero::SPELL_FLAG_COUNT)
        .map(|id| crate::dto::SpellDto {
            id,
            name: names::spell_name(id),
        })
        .collect();
    // Secondary skills: the pending override replaces the whole ordered list.
    let skills_supported = session.document.format.army_edit_supported()
        && hero.span.len() == h3forge_save::definitions::hero::RECORD_LEN;
    let skills: Vec<crate::dto::HeroSkillDto> = match get(FieldKey::Skills) {
        Some(HeroEdit::SetSkills { skills }) => skills
            .iter()
            .enumerate()
            .map(|(index, &(skill_id, mastery))| crate::dto::HeroSkillDto {
                skill_id,
                name: names::skill_name(skill_id as usize),
                mastery,
                mastery_name: names::mastery_name(mastery),
                slot: (index + 1) as u8,
            })
            .collect(),
        _ => hero
            .skills
            .iter()
            .map(|skill| crate::dto::HeroSkillDto {
                skill_id: skill.skill_id,
                name: names::skill_name(skill.skill_id as usize),
                mastery: skill.mastery,
                mastery_name: names::mastery_name(skill.mastery),
                slot: skill.slot,
            })
            .collect(),
    };
    let skill_choices = (0..h3forge_save::definitions::names::SKILL_NAMES.len() as u8)
        .map(|id| crate::dto::SkillChoiceDto {
            id,
            name: names::skill_name(id as usize),
        })
        .collect();
    let skills_read_only_reason = if !skills_supported {
        Some("Skill editing is supported only for HotA 44/11 1122-byte records.".into())
    } else if !editable {
        read_only_reason.clone()
    } else {
        None
    };
    // Backpack: per-slot pending overrides over the base decode.
    let backpack_supported = session.document.format.army_edit_supported();
    let backpack: Vec<crate::dto::BackpackItemDto> = (0..h3forge_save::artifacts::BACKPACK_SLOTS
        as u8)
        .map(|slot| {
            let effective = match get(FieldKey::Backpack(slot)) {
                Some(HeroEdit::SetBackpackSlot { item, .. }) => match item {
                    h3forge_save::BackpackEdit::Empty => h3forge_save::BackpackItem::Empty,
                    h3forge_save::BackpackEdit::Artifact(id) => {
                        h3forge_save::BackpackItem::Artifact(id)
                    }
                    h3forge_save::BackpackEdit::Scroll { spell_id } => {
                        h3forge_save::BackpackItem::Scroll { spell_id }
                    }
                },
                _ => hero.backpack[slot as usize],
            };
            crate::dto::BackpackItemDto::from_item(slot, effective)
        })
        .collect();
    let artifact_choices = h3forge_save::artifacts::HOTA_44_11_ARTIFACTS
        .iter()
        .filter(|&&(id, _)| h3forge_save::artifacts::writable(id))
        .map(|&(id, name)| crate::dto::ArtifactChoiceDto {
            id,
            name,
            equip_kind: h3forge_save::artifacts::slot_kind(id).map(|kind| kind.label()),
            effect: crate::dto::artifact_effect(id),
            combination: !h3forge_save::artifacts::components(id).is_empty(),
            also_takes: h3forge_save::artifacts::reserved_slots(id)
                .iter()
                .map(|kind| kind.label())
                .collect(),
            part_of: h3forge_save::artifacts::combination_of(id)
                .and_then(h3forge_save::artifacts::name),
        })
        .collect();
    let backpack_read_only_reason = if !backpack_supported {
        Some("Backpack editing is supported only for HotA 44/11 saves.".into())
    } else if !editable {
        read_only_reason.clone()
    } else {
        None
    };
    // Equipment: per-slot pending overrides over the base decode. Gating is
    // conservative: reservations and unmodeled wear occupants
    // block equipment with an exact cause.
    let equipment_supported = session.document.format.army_edit_supported();
    let equipment_block = hero.equipment_block_reason();
    let effective_item = |index: u8| match get(FieldKey::Equipment(index)) {
        Some(HeroEdit::SetEquipmentSlot { item, .. }) => match item {
            h3forge_save::EquipmentEdit::Empty => h3forge_save::EquipmentItem::Empty,
            h3forge_save::EquipmentEdit::Artifact(id) => h3forge_save::EquipmentItem::Artifact(id),
        },
        _ => hero.equipment_items[index as usize],
    };
    // Display positions for combination reservations. The save stores only
    // per-kind counters, so positions follow the h3sed convention: each worn
    // combination takes the last free slot of every reserved kind.
    let slot_count = h3forge_save::definitions::hero::EQUIPMENT_SLOTS as u8;
    let mut reserved_by: Vec<Option<u32>> = vec![None; slot_count as usize];
    for primary in 0..slot_count {
        let h3forge_save::EquipmentItem::Artifact(combo) = effective_item(primary) else {
            continue;
        };
        if h3forge_save::artifacts::wear_slot_kind(primary).is_none() {
            continue;
        }
        for kind in h3forge_save::artifacts::reserved_slots(combo) {
            let free = (0..slot_count).rev().find(|&candidate| {
                h3forge_save::artifacts::wear_slot_kind(candidate) == Some(*kind)
                    && effective_item(candidate) == h3forge_save::EquipmentItem::Empty
                    && reserved_by[candidate as usize].is_none()
            });
            if let Some(candidate) = free {
                reserved_by[candidate as usize] = Some(combo);
            }
        }
    }
    let equipment: Vec<crate::dto::EquipmentSlotDto> = (0..slot_count)
        .map(|index| {
            let effective = effective_item(index);
            let taken_by = reserved_by[index as usize].and_then(h3forge_save::artifacts::name);
            let wear = h3forge_save::artifacts::wear_slot_kind(index).is_some();
            let occupant_writable = match effective {
                h3forge_save::EquipmentItem::Empty => true,
                h3forge_save::EquipmentItem::Artifact(id) => h3forge_save::artifacts::writable(id),
                _ => false,
            };
            let slot_read_only = if !equipment_supported {
                Some("Equipment editing is supported only for HotA 44/11 saves.".to_owned())
            } else if !editable {
                read_only_reason.clone()
            } else if equipment_block.is_some() {
                equipment_block.clone()
            } else if !wear {
                Some("War machines and the spellbook stay read-only.".to_owned())
            } else if let Some(combo) = taken_by {
                Some(format!(
                    "Taken by {combo} (combination artifact). Unequip it to use this slot."
                ))
            } else if !occupant_writable {
                let desc = match effective {
                    h3forge_save::EquipmentItem::Artifact(id) => h3forge_save::artifacts::name(id)
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("unknown artifact (ID {id})")),
                    h3forge_save::EquipmentItem::Scroll { spell_id } => {
                        format!("a spell scroll (spell {spell_id})")
                    }
                    _ => "an unrecognized item".to_owned(),
                };
                Some(format!(
                    "Occupied by {desc}, whose effects are not fully modeled; this slot stays \
                     read-only and the item is never removed or replaced."
                ))
            } else {
                None
            };
            crate::dto::EquipmentSlotDto {
                slot: index,
                name: names::EQUIPMENT_SLOT_LABELS[index as usize],
                kind: crate::dto::equipment_kind_label(index),
                item: crate::dto::EquipmentItemDto::from_item(effective),
                reserved_by: taken_by,
                editable: slot_read_only.is_none(),
                read_only_reason: slot_read_only,
            }
        })
        .collect();
    let equipment_limitations: Vec<String> = hero.equipment_block_reason().into_iter().collect();
    let equipment_read_only_reason = if !equipment_supported {
        Some("Equipment editing is supported only for HotA 44/11 saves.".into())
    } else if !editable {
        read_only_reason.clone()
    } else {
        equipment_block
    };
    let army_supported = session.document.format.army_edit_supported();
    let army = hero
        .army
        .iter()
        .enumerate()
        .map(|(slot, original)| {
            let effective = match get(FieldKey::Army(slot as u8)) {
                Some(HeroEdit::SetArmySlot {
                    creature_id, count, ..
                }) => h3forge_save::ArmySlot {
                    creature_id: creature_id.unwrap_or(u32::MAX),
                    count,
                },
                _ => *original,
            };
            crate::dto::ArmySlotDto::new(
                &effective,
                army_supported,
                editable && original.is_supported(),
            )
        })
        .collect();
    let creature_choices = if army_supported {
        h3forge_save::creatures::HOTA_44_11
            .iter()
            .map(|&(id, name)| crate::dto::CreatureDto { id, name })
            .collect()
    } else {
        Vec::new()
    };
    let army_read_only_reason = if !army_supported {
        Some("Army editing is supported only for HotA 44/11 saves.".into())
    } else if !editable {
        read_only_reason.clone()
    } else {
        None
    };
    let mut dirty_fields: Vec<String> = pending
        .map(|map| map.keys().map(FieldKey::label).collect())
        .unwrap_or_default();
    let dismissal_pending = session.pending_dismissals.contains(&id);
    if dismissal_pending {
        dirty_fields.push(DISMISSAL_FIELD.into());
    }
    // The joint plan covers cross-hero rules (a player without towns keeps
    // one hero) on top of the per-hero structural gates.
    let dismissal_blocked_reason = if dismissal_pending {
        None
    } else {
        session.document.hero_dismissal_blocker(id).or_else(|| {
            let mut ids: Vec<usize> = session.pending_dismissals.iter().copied().collect();
            ids.push(id);
            match session.document.plan_hero_dismissals(&ids) {
                Err(h3forge_save::EditError::InvalidValue { reason, .. }) => Some(reason),
                Err(other) => Some(other.to_string()),
                Ok(_) => None,
            }
        })
    };
    Ok(HeroEditResponseDto {
        coordinates: match pending.and_then(|p| p.get(&FieldKey::Coordinates)) {
            Some(HeroEdit::SetCoordinates(to)) => Some(*to),
            _ => session.document.hero_coordinates(id),
        },
        coordinates_read_only_reason: if dismissal_pending {
            Some("Keep the hero before relocating it.".into())
        } else {
            session.document.hero_relocation_blocker(id)
        },
        id,
        editable,
        read_only_reason,
        primary: crate::dto::PrimaryDto {
            attack: primary_value(FieldKey::Attack, hero.primary.attack),
            defense: primary_value(FieldKey::Defense, hero.primary.defense),
            spell_power: primary_value(FieldKey::SpellPower, hero.primary.spell_power),
            knowledge: primary_value(FieldKey::Knowledge, hero.primary.knowledge),
        },
        experience,
        level,
        spell_points,
        spells_learned,
        spell_choices,
        skills,
        skill_choices,
        skills_read_only_reason,
        backpack,
        artifact_choices,
        backpack_read_only_reason,
        equipment,
        equipment_read_only_reason,
        equipment_limitations,
        army,
        creature_choices,
        army_read_only_reason,
        dismissal_pending,
        dismissal_blocked_reason,
        dirty_fields,
        warnings,
    })
}

/// Display pair (before, after) for one pending edit against the base view.
fn edit_display(edit: HeroEdit, hero: &HeroView) -> (String, String) {
    match edit {
        HeroEdit::SetCoordinates(to) => ("Current position".into(), format!("{to:?}")),
        HeroEdit::SetArmySlot {
            slot,
            creature_id,
            count,
        } => {
            let display = |stack: h3forge_save::ArmySlot| {
                if stack.is_empty() {
                    "Empty".into()
                } else {
                    format!(
                        "{} x {}",
                        h3forge_save::creatures::name(stack.creature_id)
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("Creature #{}", stack.creature_id)),
                        stack.count
                    )
                }
            };
            (
                display(hero.army[slot as usize]),
                display(h3forge_save::ArmySlot {
                    creature_id: creature_id.unwrap_or(u32::MAX),
                    count,
                }),
            )
        }
        HeroEdit::SetAttack(v) => (hero.primary.attack.to_string(), v.to_string()),
        HeroEdit::SetDefense(v) => (hero.primary.defense.to_string(), v.to_string()),
        HeroEdit::SetSpellPower(v) => (hero.primary.spell_power.to_string(), v.to_string()),
        HeroEdit::SetKnowledge(v) => (hero.primary.knowledge.to_string(), v.to_string()),
        HeroEdit::SetExperience(v) => (hero.experience.to_string(), v.to_string()),
        HeroEdit::SetLevel(v) => (hero.level.to_string(), v.to_string()),
        HeroEdit::SetSpellPoints(v) => (hero.spell_points.to_string(), v.to_string()),
        HeroEdit::SetSpellLearned { spell_id, learned } => {
            let before = if hero.spells_learned[spell_id as usize] == 1 {
                "learned"
            } else {
                "not learned"
            };
            let after = if learned { "learned" } else { "not learned" };
            (before.into(), after.into())
        }
        HeroEdit::SetSkills { skills } => {
            let label = |pairs: &[(u8, u8)]| {
                pairs
                    .iter()
                    .map(|&(skill_id, mastery)| {
                        let name = names::skill_name(skill_id as usize)
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("Skill {skill_id}"));
                        format!("{name} ({mastery})")
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let base: Vec<(u8, u8)> = hero
                .skills
                .iter()
                .map(|skill| (skill.skill_id, skill.mastery))
                .collect();
            (label(&base), label(&skills))
        }
        HeroEdit::SetBackpackSlot { slot, item } => {
            let display = |item: h3forge_save::BackpackItem| match item {
                h3forge_save::BackpackItem::Empty => "Empty".to_owned(),
                h3forge_save::BackpackItem::Artifact(id) => h3forge_save::artifacts::name(id)
                    .map(str::to_owned)
                    .unwrap_or(id.to_string()),
                h3forge_save::BackpackItem::Scroll { spell_id } => format!(
                    "Spell Scroll of {}",
                    names::spell_name(spell_id as usize).unwrap_or("?")
                ),
                h3forge_save::BackpackItem::Unknown => "<unknown>".to_owned(),
            };
            let desired = match item {
                h3forge_save::BackpackEdit::Empty => h3forge_save::BackpackItem::Empty,
                h3forge_save::BackpackEdit::Artifact(id) => {
                    h3forge_save::BackpackItem::Artifact(id)
                }
                h3forge_save::BackpackEdit::Scroll { spell_id } => {
                    h3forge_save::BackpackItem::Scroll { spell_id }
                }
            };
            (display(hero.backpack[slot as usize]), display(desired))
        }
        HeroEdit::SetEquipmentSlot { slot, item } => {
            let display = |item: h3forge_save::EquipmentItem| match item {
                h3forge_save::EquipmentItem::Empty => "Empty".to_owned(),
                h3forge_save::EquipmentItem::Artifact(id) => h3forge_save::artifacts::name(id)
                    .map(str::to_owned)
                    .unwrap_or(id.to_string()),
                h3forge_save::EquipmentItem::Scroll { spell_id } => format!(
                    "Spell Scroll of {}",
                    names::spell_name(spell_id as usize).unwrap_or("?")
                ),
                h3forge_save::EquipmentItem::Unknown => "<unknown>".to_owned(),
            };
            let desired = match item {
                h3forge_save::EquipmentEdit::Empty => h3forge_save::EquipmentItem::Empty,
                h3forge_save::EquipmentEdit::Artifact(id) => {
                    h3forge_save::EquipmentItem::Artifact(id)
                }
            };
            (
                display(hero.equipment_items[slot as usize]),
                display(desired),
            )
        }
    }
}

/// Display pair (before, after) for one pending town edit against the base view.
fn town_edit_display(edit: TownEdit, town: &h3forge_save::TownView) -> (String, String) {
    match edit {
        TownEdit::SetBuildings { built } => {
            let base = town.state.as_ref().map_or(0, |state| state.built);
            let names = |mask: u64| -> String {
                let list: Vec<String> = (0..h3forge_save::town_state::building::COUNT)
                    .filter(|id| mask >> id & 1 != 0)
                    .map(|id| h3forge_save::town_state::building_name(town.town_type, id))
                    .collect();
                if list.is_empty() {
                    "nothing".into()
                } else {
                    list.join(", ")
                }
            };
            (
                format!("remove {}", names(base & !built)),
                format!("build {}", names(built & !base)),
            )
        }
        TownEdit::SetGuildSpells { level, spells } => {
            let names = |spells: &[u8]| -> String {
                if spells.is_empty() {
                    return "no spells".into();
                }
                spells
                    .iter()
                    .map(|&id| {
                        h3forge_save::definitions::names::spell_name(id as usize)
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("Spell #{id}"))
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let base = town
                .state
                .as_ref()
                .filter(|_| (1..=5).contains(&level))
                .map(|state| state.guild_spells(level as usize - 1))
                .unwrap_or_default();
            (names(&base), names(&spells))
        }
        TownEdit::SetGarrisonSlot {
            slot,
            creature_id,
            count,
        } => {
            let display = |stack: h3forge_save::ArmySlot| {
                if stack.is_empty() {
                    "Empty".into()
                } else {
                    format!(
                        "{} x {}",
                        h3forge_save::creatures::name(stack.creature_id)
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("Creature #{}", stack.creature_id)),
                        stack.count
                    )
                }
            };
            (
                display(town.garrison[slot as usize]),
                display(h3forge_save::ArmySlot {
                    creature_id: creature_id.unwrap_or(u32::MAX),
                    count,
                }),
            )
        }
    }
}

/// Effective editable state for one town: base values overlaid with the
/// pending garrison overrides, plus the dirty field list.
fn town_edit_response(
    session: &SaveSession,
    id: usize,
) -> Result<crate::dto::TownEditResponseDto, AppErrorDto> {
    let town = &session.document.towns[id];
    let pending = session.pending_towns.get(&id);
    let get =
        |key: TownFieldKey| -> Option<TownEdit> { pending.and_then(|map| map.get(&key).cloned()) };
    let editable = session.document.format.army_edit_supported()
        && town.confidence == h3forge_save::TownConfidence::High;
    let read_only_reason = if editable {
        None
    } else if !session.document.format.army_edit_supported() {
        Some(format!(
            "the {} {}/{} layout has no validated edit support",
            session
                .document
                .format
                .family_title()
                .unwrap_or("recognized"),
            session.document.format.major,
            session.document.format.minor
        ))
    } else {
        Some("record failed structural verification and stays read-only".into())
    };
    // A garrison owned by a garrisoned hero is edited through that hero.
    // While a hero is garrisoned the town's own garrison words are empty
    // and that hero's army is the garrison (sample: 17 of 17 such towns), so
    // the town slots stay closed and the hero's army is offered instead.
    // The message carries no hero name: backend text cannot apply the
    // frontend's legacy-text decoding, and the UI shows the decoded name.
    let garrison_block = town.garrison_hero.map(|_| {
        "While a hero is garrisoned here, that hero's army is the town's garrison.".to_owned()
    });
    let garrison_hero_army = town.garrison_hero.and_then(|hero_id| {
        let hero = hero_edit_response(session, hero_id, Vec::new()).ok()?;
        Some(crate::dto::GarrisonHeroArmyDto {
            hero_id,
            slots: hero.army,
            read_only_reason: hero.army_read_only_reason,
            dirty_fields: hero
                .dirty_fields
                .into_iter()
                .filter(|field| field.starts_with("Army slot"))
                .collect(),
        })
    });
    let garrison_supported = session.document.format.army_edit_supported();
    let garrison: Vec<crate::dto::ArmySlotDto> = town
        .garrison
        .iter()
        .enumerate()
        .map(|(slot, original)| {
            let effective = match get(TownFieldKey::Garrison(slot as u8)) {
                Some(TownEdit::SetGarrisonSlot {
                    creature_id, count, ..
                }) => h3forge_save::ArmySlot {
                    creature_id: creature_id.unwrap_or(u32::MAX),
                    count,
                },
                _ => *original,
            };
            crate::dto::ArmySlotDto::new(
                &effective,
                garrison_supported,
                editable && original.is_supported() && garrison_block.is_none(),
            )
        })
        .collect();
    let creature_choices = if garrison_supported {
        h3forge_save::creatures::HOTA_44_11
            .iter()
            .map(|&(id, name)| crate::dto::CreatureDto { id, name })
            .collect()
    } else {
        Vec::new()
    };
    let garrison_read_only_reason = if !garrison_supported {
        Some("Garrison editing is supported only for HotA 44/11 saves.".into())
    } else if !editable {
        read_only_reason.clone()
    } else {
        garrison_block.clone()
    };
    let hero_ref = |hero_id: Option<usize>| {
        hero_id.map(|hero_id| crate::dto::HeroRefDto::from_view(&session.document.heroes[hero_id]))
    };
    let dirty_fields: Vec<String> = pending
        .map(|map| map.keys().map(TownFieldKey::label).collect())
        .unwrap_or_default();
    let buildings = town
        .state
        .as_ref()
        .map(|state| town_buildings_dto(state, pending, editable, read_only_reason.as_deref()));
    let guild = town
        .state
        .as_ref()
        .map(|state| town_guild_dto(state, pending, editable, read_only_reason.as_deref()));
    Ok(crate::dto::TownEditResponseDto {
        id,
        editable,
        read_only_reason,
        garrison,
        creature_choices,
        garrison_read_only_reason,
        garrisoned_hero: hero_ref(town.garrison_hero),
        visiting_hero: hero_ref(town.visiting_hero),
        garrison_hero_army,
        buildings,
        guild,
        dirty_fields,
        warnings: Vec::new(),
    })
}

/// The stored state overlaid with the pending building target and guild
/// spell lists. Pending edits were validated when they were recorded.
fn effective_town_state(
    base: &h3forge_save::TownState,
    pending: Option<&BTreeMap<TownFieldKey, TownEdit>>,
) -> h3forge_save::TownState {
    let mut built = None;
    let mut spells: [Option<Vec<u8>>; h3forge_save::town_state::GUILD_LEVELS] = Default::default();
    for edit in pending.into_iter().flat_map(BTreeMap::values) {
        match edit {
            TownEdit::SetBuildings { built: target } => built = Some(*target),
            TownEdit::SetGuildSpells {
                level,
                spells: list,
            } => {
                if let Some(slot) = spells.get_mut(usize::from(*level).wrapping_sub(1)) {
                    *slot = Some(list.clone());
                }
            }
            TownEdit::SetGarrisonSlot { .. } => {}
        }
    }
    base.with_changes(built, &spells)
}

fn town_buildings_dto(
    base: &h3forge_save::TownState,
    pending: Option<&BTreeMap<TownFieldKey, TownEdit>>,
    editable: bool,
    read_only_reason: Option<&str>,
) -> crate::dto::TownBuildingsDto {
    use h3forge_save::BuildingGroup;
    use h3forge_save::town_state::{building_group, building_name};
    let effective = effective_town_state(base, pending);
    let read_only_reason = if !editable {
        Some(
            read_only_reason
                .unwrap_or("this town record is read-only")
                .to_owned(),
        )
    } else {
        base.buildings_issue()
    };
    let items = base
        .building_statuses()
        .into_iter()
        .map(|status| {
            let lock_reason = if editable {
                status.lock
            } else {
                read_only_reason.clone()
            };
            crate::dto::TownBuildingDto {
                id: status.id,
                name: building_name(base.town_type, status.id),
                group: match building_group(status.id) {
                    BuildingGroup::Halls => "halls",
                    BuildingGroup::Fortifications => "fortifications",
                    BuildingGroup::MageGuild => "mageGuild",
                    BuildingGroup::Economy => "economy",
                    BuildingGroup::Dwellings => "dwellings",
                    BuildingGroup::Special => "special",
                    BuildingGroup::Other => "other",
                },
                built: effective.is_built(status.id),
                allowed: status.allowed,
                editable: lock_reason.is_none(),
                lock_reason,
                dirty: effective.is_built(status.id) != status.built,
            }
        })
        .collect();
    crate::dto::TownBuildingsDto {
        town_type: base.town_type,
        read_only_reason,
        items,
    }
}

fn town_guild_dto(
    base: &h3forge_save::TownState,
    pending: Option<&BTreeMap<TownFieldKey, TownEdit>>,
    editable: bool,
    read_only_reason: Option<&str>,
) -> crate::dto::TownGuildDto {
    use h3forge_save::definitions::names::spell_name;
    use h3forge_save::town_state::{GUILD_LEVELS, SPELL_LEVELS};
    let effective = effective_town_state(base, pending);
    let read_only_reason = if !editable {
        Some(
            read_only_reason
                .unwrap_or("this town record is read-only")
                .to_owned(),
        )
    } else {
        base.guild_issue()
    };
    let max_level = base.max_guild_level();
    let levels = (0..GUILD_LEVELS)
        .map(|index| {
            let level = index as u8 + 1;
            let spells = effective.guild_spells(index);
            crate::dto::GuildLevelDto {
                level,
                built: level <= effective.mage_guild_level,
                available: level <= max_level,
                capacity: base.guild_capacity(index),
                dirty: spells != base.guild_spells(index),
                spells: spells
                    .into_iter()
                    .map(|id| crate::dto::SpellDto {
                        id: usize::from(id),
                        name: spell_name(usize::from(id)),
                    })
                    .collect(),
                editable: read_only_reason.is_none() && level <= max_level,
            }
        })
        .collect();
    let spell_choices = SPELL_LEVELS
        .iter()
        .enumerate()
        .map(|(id, &level)| crate::dto::GuildSpellChoiceDto {
            id: id as u8,
            name: spell_name(id).unwrap_or("Unknown spell"),
            level,
            banned: base.spell_banned(id as u8),
        })
        .collect();
    crate::dto::TownGuildDto {
        read_only_reason,
        built_level: effective.mage_guild_level,
        max_level,
        has_library: base.has_library(),
        levels,
        spell_choices,
    }
}

/// Hard development guard: no save destination may resolve into the
/// repository's private `games/` or `Data/` directories. Save As additionally
/// rejects the current source; direct Save alone can replace it.
fn guard_protected(destination: &Path) -> Result<(), AppErrorDto> {
    let canonical = if destination.exists() {
        destination.canonicalize()
    } else {
        destination
            .parent()
            .ok_or_else(|| AppErrorDto::forbidden_destination(destination))?
            .canonicalize()
            .map(|p| p.join(destination.file_name().unwrap_or_default()))
    }
    .map_err(|_| AppErrorDto::forbidden_destination(destination))?;
    // Find a development checkout relative to the running EXE, without embedding
    // its build-machine path. This also protects local portable release builds.
    let checkout = std::env::current_exe().ok().and_then(|exe| {
        exe.ancestors()
            .find(|dir| dir.join("Cargo.toml").is_file() && dir.join("AGENTS.md").is_file())
            .and_then(|dir| dir.canonicalize().ok())
    });
    if let Some(root) = checkout {
        for name in ["games", "Data", "Maps"] {
            if let Ok(protected) = root.join(name).canonicalize() {
                if canonical.starts_with(protected) {
                    return Err(AppErrorDto { cause: None, params: std::collections::BTreeMap::new(), code: "forbidden-destination",
                        message: "This development fixture/resource is protected. Use Save As outside the project's games/ and Data/ folders.".into() });
                }
            }
        }
    }
    Ok(())
}

fn guard_destination(destination: &Path, source: &Path) -> Result<(), AppErrorDto> {
    guard_protected(destination)?;
    if destination.canonicalize().ok().as_ref() == source.canonicalize().ok().as_ref() {
        return Err(AppErrorDto::same_file(destination));
    }
    Ok(())
}

#[test]
fn checkout_fixture_guards_do_not_require_an_embedded_build_path() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    assert!(guard_protected(&root.join("Cargo.toml")).is_ok());
    // Read-only check: never create or write anything in the private directories.
    for name in ["games", "Data", "Maps"] {
        let directory = root.join(name);
        if directory.is_dir() {
            assert_eq!(
                guard_protected(&directory.join("guard-check.GM2"))
                    .unwrap_err()
                    .code,
                "forbidden-destination"
            );
        }
    }
}

fn verify_source(session: &SaveSession) -> Result<(), AppErrorDto> {
    if SourceIdentity::capture(&session.source_path).as_ref() != Some(&session.source_identity)
        || read_bounded(&session.source_path, Limits::default().max_compressed)
            .ok()
            .as_deref()
            != Some(session.document.original_bytes())
    {
        return Err(AppErrorDto::source_changed(&session.source_path));
    }
    Ok(())
}

struct TempCleanup(PathBuf);
impl Drop for TempCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn direct_save_capability(path: &Path) -> Result<(), AppErrorDto> {
    guard_protected(path)?;
    // Opening without truncation checks file ACL/read-only flags; a create-new
    // probe checks directory access for the required backup/temp operations.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| AppErrorDto::write_failed(path, &e))?;
    drop(file);
    let probe = temp_path_in(path)?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|e| AppErrorDto::write_failed(path, &e))?;
    let _cleanup = TempCleanup(probe);
    drop(file);
    Ok(())
}

/// Unique backup path for a save destination: `<dest>.backup-<millis>` with
/// a counter when the millisecond name already exists.
fn backup_path_for(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "savegame".into());
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    for counter in 0..1000u32 {
        let suffix = if counter == 0 {
            format!("{millis}")
        } else {
            format!("{millis}-{counter}")
        };
        let candidate = parent.join(format!("{name}.backup-{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{name}.backup-{millis}-overflow"))
}

/// Unique temporary file path inside the destination directory (same volume,
/// so the commit rename cannot cross drives).
fn temp_path_in(destination: &Path) -> Result<PathBuf, AppErrorDto> {
    let parent = destination
        .parent()
        .ok_or_else(|| AppErrorDto::write_failed_plain(destination))?;
    let pid = std::process::id();
    for counter in 0..1000u32 {
        let candidate = parent.join(format!(".h3forge-{pid}-{counter}.tmp"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(AppErrorDto::write_failed_plain(destination))
}

impl InstallationState {
    /// Root the user picked (or that discovery will re-validate); suitable
    /// for persistence and picker start directories.
    pub fn picked_root(&self) -> &Path {
        &self.picked_root
    }

    fn load(
        installation: Installation,
        picked_root: PathBuf,
        source: InstallationSource,
    ) -> Result<InstallationState, AppErrorDto> {
        let bitmaps = installation
            .open_bitmaps()
            .map_err(AppErrorDto::asset_failed)?;
        let identity = SourceIdentity::capture(
            &installation
                .archive_path("H3bitmap.lod")
                .expect("open_bitmaps succeeded"),
        )
        .ok_or_else(|| {
            AppErrorDto::asset_failed(h3forge_resources::ResourceError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "archive metadata unavailable",
            )))
        })?;
        // HotA portraits live in HotA.lod; an absent or unreadable archive
        // only leaves those portraits unresolved.
        let hota = installation
            .archive_path("HotA.lod")
            .and_then(|path| LodArchive::open(path).ok())
            .filter(LodArchive::has_opaque_ids);
        Ok(InstallationState {
            source,
            picked_root,
            installation,
            bitmaps,
            hota,
            identity,
            cache: HashMap::new(),
        })
    }

    /// Reads one portrait through the name table and encodes a PNG. The
    /// error text explains which resource is missing; nothing is guessed.
    fn load_portrait_png(
        &self,
        file: &HeroPortraitFile,
        size: PortraitSize,
    ) -> Result<Vec<u8>, String> {
        let limits = h3forge_resources::ResourceLimits::default();
        let bytes = match file {
            HeroPortraitFile::Base { large, small } => {
                let (first, second) = match size {
                    PortraitSize::Large => (large, small),
                    PortraitSize::Small => (small, large),
                };
                let index = self
                    .bitmaps
                    .find_entry(first)
                    .or_else(|_| self.bitmaps.find_entry(second))
                    .map_err(|_| format!("{large} is not in this installation's H3bitmap.lod"))?;
                self.bitmaps.read_entry(index).map_err(|e| e.to_string())?
            }
            HeroPortraitFile::HotA { name, entry_id } => {
                let archive = self.hota.as_ref().ok_or_else(|| {
                    format!("{name} is a HotA portrait; this installation has no readable HotA 1.8 HotA.lod")
                })?;
                let index = archive.find_entry(entry_id).map_err(|_| {
                    format!("{name} was not found in this installation's HotA.lod (different HotA version)")
                })?;
                archive.read_entry(index).map_err(|e| e.to_string())?
            }
        };
        let image =
            h3forge_resources::decode_portrait_image(&bytes, limits).map_err(|e| e.to_string())?;
        Ok(encode_png(&image))
    }
}

/// Best-effort classification of how discovery found the installation, for
/// display only; correctness never depends on it.
fn classify_source(installation: &Installation, stored: Option<&Path>) -> InstallationSource {
    if stored.is_some_and(|stored| {
        stored == installation.root || stored.join("Data") == installation.root
    }) {
        // A stored selection that is not the exe parent counts as stored.
        let exe_parent = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        if exe_parent.as_deref() != Some(installation.root.as_path()) {
            return InstallationSource::Stored;
        }
    }
    if std::env::var_os("H3FORGE_DEV_DATA_ROOT")
        .map(PathBuf::from)
        .is_some_and(|dev| dev == installation.root || dev.join("Data") == installation.root)
    {
        return InstallationSource::Development;
    }
    InstallationSource::ExeParent
}

/// Reads a file after a metadata size check so oversized inputs fail before
/// the bytes are pulled into memory.
fn read_bounded(path: &Path, max_bytes: usize) -> Result<Vec<u8>, AppErrorDto> {
    let metadata =
        std::fs::metadata(path).map_err(|error| AppErrorDto::file_read(error.to_string()))?;
    if !metadata.is_file() {
        return Err(AppErrorDto::file_read("not a regular file".into()));
    }
    if metadata.len() > max_bytes as u64 {
        return Err(AppErrorDto::file_too_large(max_bytes));
    }
    std::fs::read(path).map_err(|error| AppErrorDto::file_read(error.to_string()))
}

fn team_edits(moves: &BTreeMap<u8, u8>) -> Vec<h3forge_save::TeamEdit> {
    moves
        .iter()
        .map(|(&color, &team)| h3forge_save::TeamEdit { color, team })
        .collect()
}
