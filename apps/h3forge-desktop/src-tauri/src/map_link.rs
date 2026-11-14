//! Links the open savegame to the `.h3m` it was started from (H3F-021).
//!
//! Discovery is bounded to `Maps` folders next to the save and inside the
//! selected installation; matching uses the map's internal name plus every
//! other header field the save retains. The linked map is **reference
//! data**: it describes the scenario before turn one and is only ever
//! shown beside the save's current values, never written into them.
//!
//! No Tauri types here, so the whole flow is testable without a runtime.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use h3forge_map::{
    FileStamp, MapHeader, MapHeroKind, MapLimits, MatchOutcome, MatchStrength, ParsedMap,
    SaveMapKey, ScanLimits,
};
use h3forge_save::{HeroView, SaveDocument, decode_legacy_text};
use serde::Serialize;

use crate::dto::TextDto;

/// Headers remembered between scans, keyed by path + size + mtime so an
/// edited or replaced map is re-read. Bounded; cleared when full.
#[derive(Debug, Default)]
pub struct MapHeaderCache {
    entries: HashMap<FileStamp, Option<MapHeader>>,
}

const HEADER_CACHE_CAP: usize = 8_192;

impl MapHeaderCache {
    fn header(&mut self, path: &Path) -> Option<MapHeader> {
        let stamp = FileStamp::capture(path)?;
        if let Some(cached) = self.entries.get(&stamp) {
            return cached.clone();
        }
        let header =
            h3forge_map::read_map_header(path, MapLimits::default(), ScanLimits::default()).ok();
        if self.entries.len() >= HEADER_CACHE_CAP {
            self.entries.clear();
        }
        self.entries.insert(stamp, header.clone());
        header
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// How the current link came about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    /// Discovery found exactly one compatible map.
    Matched,
    /// The user picked the map (file dialog or candidate list).
    Manual,
    /// Several maps are indistinguishable by header; the user must choose.
    Ambiguous,
    /// No compatible map in the searched folders.
    NotFound,
    /// The save carries no map name to match on.
    NoMapName,
}

impl LinkStatus {
    fn code(self) -> &'static str {
        match self {
            LinkStatus::Matched => "matched",
            LinkStatus::Manual => "manual",
            LinkStatus::Ambiguous => "ambiguous",
            LinkStatus::NotFound => "not-found",
            LinkStatus::NoMapName => "no-map-name",
        }
    }
}

/// A map selected for the open save, with its hero-type index.
#[derive(Debug)]
pub struct LinkedMap {
    pub path: PathBuf,
    pub strength: MatchStrength,
    pub map: ParsedMap,
}

/// Link state stored on the save session.
#[derive(Debug)]
pub struct MapLink {
    pub status: LinkStatus,
    pub searched: Vec<PathBuf>,
    pub scanned_files: usize,
    /// Compatible candidates when the match is ambiguous.
    pub candidates: Vec<PathBuf>,
    pub linked: Option<LinkedMap>,
}

/// The save's retained map header fields as a match key.
pub fn save_key(document: &SaveDocument) -> SaveMapKey<'_> {
    SaveMapKey {
        name_raw: &document.map.name_raw,
        description_raw: document.map.description_raw.as_deref(),
        size: document.map.size,
        has_underground: document.map.has_underground,
        difficulty: document.map.difficulty,
    }
}

/// `Maps` folders worth searching for a save at `save_path`: beside the
/// save's folder (the game keeps `games/` and `Maps/` side by side, as does
/// the development workspace) and inside each given installation root.
/// Only existing directories are returned, without duplicates.
pub fn search_dirs(save_path: &Path, installation_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Some(folder) = save_path.parent() {
        if let Some(parent) = folder.parent() {
            bases.push(parent.to_path_buf());
        }
        bases.push(folder.to_path_buf());
    }
    for root in installation_roots {
        bases.push(root.clone());
        // Discovery may report the `Data` folder itself as the root.
        if let Some(parent) = root.parent() {
            bases.push(parent.to_path_buf());
        }
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    for base in bases {
        let candidate = base.join("Maps");
        let Ok(resolved) = candidate.canonicalize() else {
            continue;
        };
        if resolved.is_dir() && !dirs.contains(&resolved) {
            dirs.push(resolved);
        }
    }
    dirs
}

/// Runs discovery and matching for one save.
pub fn discover(
    document: &SaveDocument,
    dirs: Vec<PathBuf>,
    cache: &mut MapHeaderCache,
) -> MapLink {
    let key = save_key(document);
    if key.name_raw.is_empty() {
        return MapLink {
            status: LinkStatus::NoMapName,
            searched: dirs,
            scanned_files: 0,
            candidates: Vec::new(),
            linked: None,
        };
    }
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in &dirs {
        for file in h3forge_map::list_map_files(dir, ScanLimits::default()) {
            if !files.contains(&file) {
                files.push(file);
            }
        }
    }
    let mut readable: Vec<(PathBuf, MapHeader)> = Vec::new();
    for file in &files {
        if let Some(header) = cache.header(file) {
            readable.push((file.clone(), header));
        }
    }
    let outcome = h3forge_map::match_candidates(&key, readable.iter().map(|(_, header)| header));
    let mut link = MapLink {
        status: LinkStatus::NotFound,
        searched: dirs,
        scanned_files: files.len(),
        candidates: Vec::new(),
        linked: None,
    };
    match outcome {
        MatchOutcome::NotFound => {}
        MatchOutcome::Unique { index, strength } => {
            let path = &readable[index].0;
            if let Ok(map) = h3forge_map::read_map(path, MapLimits::default()) {
                link.status = LinkStatus::Matched;
                link.linked = Some(LinkedMap {
                    path: path.clone(),
                    strength,
                    map,
                });
            }
        }
        MatchOutcome::Ambiguous { indexes } => {
            link.status = LinkStatus::Ambiguous;
            link.candidates = indexes
                .into_iter()
                .map(|index| readable[index].0.clone())
                .collect();
        }
    }
    link
}

/// Loads a user-chosen map for the save. A map whose header contradicts
/// the save is refused: enriching by hero type from an unrelated map would
/// present wrong data as fact.
pub fn link_manual(document: &SaveDocument, path: &Path) -> Result<LinkedMap, String> {
    let map = h3forge_map::read_map(path, MapLimits::default()).map_err(|e| e.to_string())?;
    let key = save_key(document);
    let Some(strength) = h3forge_map::match_strength(&key, &map.header) else {
        return Err(format!(
            "\"{}\" is a different map: its name, size, levels or description do not match the map this save was started from (\"{}\").",
            decode_legacy_text(&map.header.name_raw),
            document.map.name_lossy
        ));
    };
    Ok(LinkedMap {
        path: path.to_path_buf(),
        strength,
        map,
    })
}

// ------------------------------------------------------------ enrichment --

/// What the map says about one hero type, gathered from every place the
/// format can customize a hero.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapHeroFacts {
    /// Placement, when exactly one map object declares this hero type.
    pub placement: Option<Placement>,
    /// More than one object declares the type; no placement is reported.
    pub ambiguous_placement: bool,
    /// Portrait the map assigns (object first, then the header entry).
    pub portrait: Option<u8>,
    /// Name the map assigns (object first, then the header entries).
    pub name_raw: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub kind: MapHeroKind,
    pub x: u8,
    pub y: u8,
    pub z: u8,
    pub owner: Option<u8>,
}

impl LinkedMap {
    /// Map facts for one hero type. The hero type is the only identity the
    /// save and the map share; names and record order are never used.
    pub fn hero_facts(&self, hero_type: u8) -> MapHeroFacts {
        let mut facts = MapHeroFacts::default();
        let header = &self.map.header;
        if let Some(disposed) = header
            .disposed_heroes
            .iter()
            .find(|hero| hero.hero_type == hero_type)
        {
            facts.portrait = disposed.portrait;
            if !disposed.name_raw.is_empty() {
                facts.name_raw = Some(disposed.name_raw.clone());
            }
        }
        for player in &header.players {
            if let Some(main) = player
                .main_hero
                .as_ref()
                .filter(|main| main.hero_type == hero_type)
            {
                facts.portrait = main.portrait.or(facts.portrait);
                if !main.name_raw.is_empty() {
                    facts.name_raw = Some(main.name_raw.clone());
                }
            }
        }
        if let Ok(body) = &self.map.body {
            let mut objects = body
                .heroes
                .iter()
                .filter(|hero| hero.hero_type == Some(hero_type));
            if let Some(object) = objects.next() {
                if objects.next().is_some() {
                    facts.ambiguous_placement = true;
                } else {
                    let tile = object.tile();
                    facts.placement = Some(Placement {
                        kind: object.kind,
                        x: tile.x,
                        y: tile.y,
                        z: tile.z,
                        owner: object.owner,
                    });
                    facts.portrait = object.portrait.or(facts.portrait);
                    if let Some(name) = object.name_raw.as_ref().filter(|name| !name.is_empty()) {
                        facts.name_raw = Some(name.clone());
                    }
                }
            }
        }
        facts
    }

    /// Map facts for one save town, matched by verified coordinates: the
    /// save town (x, y, z) equals the map town entrance (anchor x-2, y, z;
    /// sample-verified for all 37 towns, H3F-013). Names and ownership are
    /// never used to match; mismatches are reported, not silently resolved.
    pub fn town_info(&self, town: &h3forge_save::TownView) -> TownMapInfoDto {
        let body = self.map.body.as_ref();
        let candidates: Vec<&h3forge_map::MapTown> = body
            .map(|body| {
                body.towns
                    .iter()
                    .filter(|candidate| {
                        candidate.anchor.x.saturating_sub(2) == town.x
                            && candidate.anchor.y == town.y
                            && candidate.anchor.z == town.z
                    })
                    .collect()
            })
            .unwrap_or_default();
        match candidates.as_slice() {
            [map_town] => {
                let name_matches = map_town.name_raw.as_deref() == Some(town.name_raw.as_slice());
                TownMapInfoDto {
                    matched: true,
                    ambiguous: false,
                    map_name: map_town
                        .name_raw
                        .as_ref()
                        .map(|raw| TextDto::new(raw, &decode_legacy_text(raw))),
                    name_matches,
                    map_owner: map_town.owner,
                    owner_matches: map_town
                        .owner
                        .map(|owner| owner == town.owner.byte() || map_town.owner.is_none()),
                    map_faction: map_town.faction.map(|faction| faction as u8),
                    faction_matches: map_town
                        .faction
                        .map(|faction| faction as u8 == town.town_type),
                    map_has_garrison: map_town.has_garrison,
                }
            }
            [] => TownMapInfoDto {
                matched: body.is_err(),
                ambiguous: false,
                map_name: None,
                name_matches: false,
                map_owner: None,
                owner_matches: None,
                map_faction: None,
                faction_matches: None,
                map_has_garrison: false,
            },
            _ => TownMapInfoDto {
                matched: false,
                ambiguous: true,
                map_name: None,
                name_matches: false,
                map_owner: None,
                owner_matches: None,
                map_faction: None,
                faction_matches: None,
                map_has_garrison: false,
            },
        }
    }

    /// Cross-check of portrait IDs: for every save hero whose type the map
    /// gives an explicit portrait, does the save carry the same ID?
    /// Returns `(checked, consistent)`.
    pub fn portrait_consistency(&self, heroes: &[HeroView]) -> (usize, usize) {
        let mut checked = 0;
        let mut consistent = 0;
        for hero in heroes {
            let Some(identity) = hero.portrait else {
                continue;
            };
            let Some(expected) = self.hero_facts(identity.hero_type).portrait else {
                continue;
            };
            checked += 1;
            if expected == identity.portrait_id {
                consistent += 1;
            }
        }
        (checked, consistent)
    }
}

// ------------------------------------------------------------------ DTOs --

/// Initial-map facts for one save town (H3F-013); matched by verified
/// coordinates, never by name. `matched` false with `ambiguous` false means
/// the linked map has no town object at this town's position.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TownMapInfoDto {
    /// A map town object exists at this town's coordinates.
    pub matched: bool,
    /// More than one map town occupies the position; reported, not chosen.
    pub ambiguous: bool,
    pub map_name: Option<TextDto>,
    /// Whether the map's town name equals the save name bytes.
    pub name_matches: bool,
    /// Starting owner 0-7 from the map; `None` when the map leaves it unset.
    pub map_owner: Option<u8>,
    /// Whether the starting owner equals the current save owner; `None`
    /// when the map declares no owner.
    pub owner_matches: Option<bool>,
    /// Faction the map template declares (`None` for a random town).
    pub map_faction: Option<u8>,
    /// Whether the map faction equals the save town type byte.
    pub faction_matches: Option<bool>,
    /// Whether the map gave this town a starting garrison.
    pub map_has_garrison: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapCandidateDto {
    /// Index to pass back to `select_map_candidate`.
    pub index: usize,
    pub file_name: String,
    pub folder: String,
}

/// The linked map's own facts (all from the `.h3m`, none from the save).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedMapDto {
    pub file_name: String,
    pub folder: String,
    pub name: TextDto,
    pub format: String,
    pub hota_release: Option<String>,
    pub size: u32,
    pub has_underground: bool,
    /// "full" | "name-and-dimensions" | "name-only".
    pub match_strength: &'static str,
    pub playable_players: usize,
    pub human_players: usize,
    /// Object-level data; `None` when the map body could not be read.
    pub towns: Option<usize>,
    pub placed_heroes: Option<usize>,
    pub random_heroes: Option<usize>,
    pub objects: Option<usize>,
    /// Why object-level data is unavailable, when it is.
    pub body_error: Option<String>,
    /// Save heroes whose type has a single placement on the map.
    pub heroes_placed_in_save: usize,
    /// Save heroes whose type the map gives an explicit portrait.
    pub portraits_checked: usize,
    /// Of those, how many carry the same portrait ID in the save.
    pub portraits_consistent: usize,
    /// Save towns matched to exactly one map town object by coordinates.
    pub towns_matched_in_save: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapLinkDto {
    /// "matched" | "manual" | "ambiguous" | "not-found" | "no-map-name".
    pub status: &'static str,
    pub message: String,
    pub searched: Vec<String>,
    pub scanned_files: usize,
    pub candidates: Vec<MapCandidateDto>,
    pub map: Option<LinkedMapDto>,
}

/// Initial-map facts for one hero, shown beside the save's current state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroMapInfoDto {
    /// "hero" | "prison" | "not-placed" | "ambiguous" | "unknown".
    pub placement: &'static str,
    pub x: Option<u8>,
    pub y: Option<u8>,
    /// 0 surface, 1 underground.
    pub level: Option<u8>,
    /// Starting owner 0-7; `None` for neutral, prison or unplaced heroes.
    pub owner: Option<u8>,
    /// Name the map assigns to this hero type, when it customizes one.
    pub map_name: Option<TextDto>,
    /// Portrait the map assigns to this hero type, when it customizes one.
    pub map_portrait_id: Option<u8>,
    /// Whether the save's portrait ID equals `map_portrait_id`.
    pub portrait_matches: Option<bool>,
}

fn folder_text(path: &Path) -> String {
    path.parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn file_text(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl MapLink {
    pub fn to_dto(&self, document: &SaveDocument) -> MapLinkDto {
        let map = self.linked.as_ref().map(|linked| {
            let header = &linked.map.header;
            let body = linked.map.body.as_ref();
            let (portraits_checked, portraits_consistent) =
                linked.portrait_consistency(&document.heroes);
            let heroes_placed_in_save = document
                .heroes
                .iter()
                .filter_map(|hero| hero.portrait)
                .filter(|identity| linked.hero_facts(identity.hero_type).placement.is_some())
                .count();
            let towns_matched_in_save = document
                .towns
                .iter()
                .filter(|town| linked.town_info(town).matched)
                .count();
            LinkedMapDto {
                file_name: file_text(&linked.path),
                folder: folder_text(&linked.path),
                name: TextDto::new(&header.name_raw, &decode_legacy_text(&header.name_raw)),
                format: header.format.label(),
                hota_release: header
                    .hota_release
                    .map(|(major, minor, patch)| format!("{major}.{minor}.{patch}")),
                size: header.size,
                has_underground: header.has_underground,
                match_strength: match linked.strength {
                    MatchStrength::Full => "full",
                    MatchStrength::NameAndDimensions => "name-and-dimensions",
                    MatchStrength::NameOnly => "name-only",
                },
                playable_players: header.players.iter().filter(|p| p.is_playable()).count(),
                human_players: header.players.iter().filter(|p| p.can_human_play).count(),
                towns: body.ok().map(|body| body.towns.len()),
                placed_heroes: body.ok().map(|body| {
                    body.heroes
                        .iter()
                        .filter(|hero| hero.kind != MapHeroKind::Random)
                        .count()
                }),
                random_heroes: body.ok().map(|body| {
                    body.heroes
                        .iter()
                        .filter(|hero| hero.kind == MapHeroKind::Random)
                        .count()
                }),
                objects: body.ok().map(|body| body.object_count),
                body_error: body.err().map(|error| error.to_string()),
                heroes_placed_in_save,
                portraits_checked,
                portraits_consistent,
                towns_matched_in_save,
            }
        });
        let message = match self.status {
            LinkStatus::Matched => "Map found automatically.".to_string(),
            LinkStatus::Manual => "Map chosen manually.".to_string(),
            LinkStatus::Ambiguous => format!(
                "{} maps have this name, size and description. Choose the one this save was started from.",
                self.candidates.len()
            ),
            LinkStatus::NotFound => format!(
                "No matching map among {} files in {} Maps folder(s). Choose the .h3m manually.",
                self.scanned_files,
                self.searched.len()
            ),
            LinkStatus::NoMapName => {
                "This save carries no map name, so a map cannot be matched automatically.".into()
            }
        };
        MapLinkDto {
            status: self.status.code(),
            message,
            searched: self
                .searched
                .iter()
                .map(|dir| dir.to_string_lossy().into_owned())
                .collect(),
            scanned_files: self.scanned_files,
            candidates: self
                .candidates
                .iter()
                .enumerate()
                .map(|(index, path)| MapCandidateDto {
                    index,
                    file_name: file_text(path),
                    folder: folder_text(path),
                })
                .collect(),
            map,
        }
    }

    /// Initial-map facts for one save hero; `None` without a linked map or
    /// when the record carries no identity bytes.
    pub fn hero_info(&self, hero: &HeroView) -> Option<HeroMapInfoDto> {
        let linked = self.linked.as_ref()?;
        let identity = hero.portrait?;
        let facts = linked.hero_facts(identity.hero_type);
        let placement = match (&facts.placement, facts.ambiguous_placement) {
            (_, true) => "ambiguous",
            (Some(place), _) if place.kind == MapHeroKind::Prison => "prison",
            (Some(_), _) => "hero",
            (None, _) if linked.map.body.is_err() => "unknown",
            (None, _) => "not-placed",
        };
        Some(HeroMapInfoDto {
            placement,
            x: facts.placement.map(|place| place.x),
            y: facts.placement.map(|place| place.y),
            level: facts.placement.map(|place| place.z),
            owner: facts.placement.and_then(|place| place.owner),
            map_name: facts
                .name_raw
                .as_ref()
                .map(|raw| TextDto::new(raw, &decode_legacy_text(raw))),
            map_portrait_id: facts.portrait,
            portrait_matches: facts
                .portrait
                .map(|portrait| portrait == identity.portrait_id),
        })
    }

    /// Initial-map facts for one save town; `None` without a linked map.
    pub fn town_info(&self, town: &h3forge_save::TownView) -> Option<TownMapInfoDto> {
        self.linked.as_ref().map(|linked| linked.town_info(town))
    }
}
