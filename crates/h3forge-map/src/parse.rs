//! Sequential `.h3m` reader.
//!
//! The format has no section table and no per-object lengths: reaching the
//! hero and town objects means stepping over every earlier field with the
//! exact width its format revision uses. Layout knowledge comes from the
//! public format descriptions (h3mtxt by Alexander Belous, h3m2json) and
//! was cross-checked field by field against VCMI's `MapFormatH3M.cpp`
//! loader, which documents the HotA 1.7/1.8 additions. No code is copied;
//! only the fields H3Forge uses are kept, everything else is skipped with
//! bounds checks.
//!
//! Verified end to end (payload consumed exactly) on HotA format 10; the
//! RoE/AB/SoD and older HotA paths follow the same descriptions and are
//! exercised by the local stock maps (see `docs/issues/021-map-discovery.md`).

use std::collections::BTreeMap;

use crate::container::MapLimits;
use crate::error::MapError;
use crate::model::{
    DisposedHero, MainHero, MapBody, MapFormat, MapHeader, MapHero, MapHeroKind, MapTown,
    PlayerInfo, Position, TownBuildings,
};
use crate::reader::Reader;

const PLAYER_SLOTS: usize = 8;

/// Field widths and optional blocks that depend on the format revision.
#[derive(Debug, Clone, Copy)]
struct Features {
    /// Armageddon's Blade and later.
    ab: bool,
    /// Shadow of Death and later.
    sod: bool,
    /// HotA format subrevision; `None` for the original formats.
    hota: Option<u32>,
}

impl Features {
    fn hota_after(&self, level: u32) -> bool {
        self.hota.is_some_and(|subrevision| subrevision > level)
    }
    fn is_hota(&self) -> bool {
        self.hota.is_some()
    }
    fn faction_mask_bytes(&self) -> usize {
        if self.ab { 2 } else { 1 }
    }
    fn artifact_slots(&self) -> usize {
        if self.sod { 19 } else { 18 }
    }
}

/// Highest HotA format subrevision the body walker understands.
pub const MAX_HOTA_SUBREVISION: u32 = 10;

/// Parses the header (through the disposed-heroes block).
pub fn parse_header(payload: &[u8], limits: MapLimits) -> Result<MapHeader, MapError> {
    let mut reader = Reader::new(payload, limits.max_text);
    read_header(&mut reader).map(|(header, _)| header)
}

/// Parses header and body. A body failure is recorded, not propagated.
pub fn parse_map(payload: &[u8], limits: MapLimits) -> Result<crate::ParsedMap, MapError> {
    let mut reader = Reader::new(payload, limits.max_text);
    let (header, features) = read_header(&mut reader)?;
    let body = read_body(&mut reader, &header, features, limits);
    Ok(crate::ParsedMap { header, body })
}

fn read_header(r: &mut Reader<'_>) -> Result<(MapHeader, Features), MapError> {
    let version = r.u32("format version")?;
    let mut hota_release = None;
    let (format, features) = match version {
        0x0E => (
            MapFormat::RestorationOfErathia,
            Features {
                ab: false,
                sod: false,
                hota: None,
            },
        ),
        0x15 => (
            MapFormat::ArmageddonsBlade,
            Features {
                ab: true,
                sod: false,
                hota: None,
            },
        ),
        0x1C | 0x33 => (
            if version == 0x33 {
                MapFormat::WakeOfGods
            } else {
                MapFormat::ShadowOfDeath
            },
            Features {
                ab: true,
                sod: true,
                hota: None,
            },
        ),
        0x20 => {
            let subrevision = r.u32("HotA format subrevision")?;
            let features = Features {
                ab: true,
                sod: true,
                hota: Some(subrevision),
            };
            if features.hota_after(7) {
                hota_release = Some((
                    r.u32("HotA release")?,
                    r.u32("HotA release")?,
                    r.u32("HotA release")?,
                ));
            }
            if features.hota_after(0) {
                r.skip(2, "mirror/arena flags")?;
            }
            if features.hota_after(1) {
                r.skip(4, "terrain type count")?;
            }
            if features.hota_after(4) {
                r.skip(5, "town type count and difficulty mask")?;
            }
            if features.hota_after(6) {
                r.skip(1, "hire defeated heroes flag")?;
            }
            if features.hota_after(7) {
                r.skip(1, "force matching version flag")?;
            }
            if features.hota_after(8) {
                let variables = r.count(4096, "shared variable count")?;
                for _ in 0..variables {
                    r.skip_string("shared variable name")?;
                    r.skip(5, "shared variable")?;
                }
            }
            (MapFormat::HornOfTheAbyss { subrevision }, features)
        }
        other => return Err(MapError::UnknownFormat { version: other }),
    };

    r.skip(1, "has players flag")?;
    let size_at = r.pos();
    let size = r.u32("map size")?;
    if size == 0 || size > 255 {
        return Err(MapError::Invalid {
            offset: size_at,
            what: "map size",
        });
    }
    let has_underground = r.bool("underground flag")?;
    let name_raw = r.string("map name")?.to_vec();
    let description_raw = r.string("map description")?.to_vec();
    let difficulty = r.u8("difficulty")?;
    let level_limit = if features.ab { r.u8("level limit")? } else { 0 };

    let mut players = Vec::with_capacity(PLAYER_SLOTS);
    for _ in 0..PLAYER_SLOTS {
        players.push(read_player(r, features)?);
    }
    read_victory_loss(r, features)?;

    let team_count = r.u8("team count")?;
    if team_count > 8 {
        return Err(MapError::Invalid {
            offset: r.pos() - 1,
            what: "team count",
        });
    }
    if team_count > 0 {
        for player in &mut players {
            let team = r.u8("team")?;
            if player.is_playable() && team >= team_count {
                return Err(MapError::Invalid {
                    offset: r.pos() - 1,
                    what: "player team",
                });
            }
            player.team = (team < team_count).then_some(team);
        }
    }

    // Allowed heroes bitmask.
    if features.is_hota() {
        let heroes = r.count(4096, "allowed hero count")?;
        r.skip(heroes.div_ceil(8), "allowed heroes")?;
    } else {
        r.skip(if features.ab { 20 } else { 16 }, "allowed heroes")?;
    }
    if features.ab {
        let placeholders = r.count(4096, "placeholder hero count")?;
        r.skip(placeholders, "placeholder heroes")?;
    }

    let mut disposed_heroes = Vec::new();
    if features.sod {
        let count = r.u8("disposed hero count")?;
        for _ in 0..count {
            let hero_type = r.u8("disposed hero type")?;
            let portrait = portrait_byte(r.u8("disposed hero portrait")?);
            let name_raw = r.string("disposed hero name")?.to_vec();
            let players = r.u8("disposed hero players")?;
            disposed_heroes.push(DisposedHero {
                hero_type,
                portrait,
                name_raw,
                players,
            });
        }
    }

    Ok((
        MapHeader {
            format,
            hota_release,
            size,
            has_underground,
            name_raw,
            description_raw,
            difficulty,
            level_limit,
            players,
            disposed_heroes,
        },
        features,
    ))
}

fn portrait_byte(raw: u8) -> Option<u8> {
    (raw != 0xFF).then_some(raw)
}

fn read_position(r: &mut Reader<'_>, what: &'static str) -> Result<Position, MapError> {
    let bytes = r.bytes(3, what)?;
    Ok(Position {
        x: bytes[0],
        y: bytes[1],
        z: bytes[2],
    })
}

fn read_player(r: &mut Reader<'_>, f: Features) -> Result<PlayerInfo, MapError> {
    let mut player = PlayerInfo {
        can_human_play: r.bool("player flags")?,
        can_computer_play: r.bool("player flags")?,
        ..PlayerInfo::default()
    };
    r.skip(1, "AI tactic")?;
    if f.sod {
        r.skip(1, "faction selectable flag")?;
    }
    r.skip(f.faction_mask_bytes(), "allowed factions")?;
    r.skip(1, "random faction flag")?;
    if r.bool("main town flag")? {
        if f.ab {
            player.generate_hero_at_main_town = r.bool("generate hero flag")?;
            r.skip(1, "main town type")?;
        } else {
            player.generate_hero_at_main_town = true;
        }
        player.main_town = Some(read_position(r, "main town position")?);
    }
    player.has_random_hero = r.bool("random hero flag")?;
    let main_hero = r.u8("main hero type")?;
    if main_hero != 0xFF {
        let portrait = portrait_byte(r.u8("main hero portrait")?);
        let name_raw = r.string("main hero name")?.to_vec();
        player.main_hero = Some(MainHero {
            hero_type: main_hero,
            portrait,
            name_raw,
        });
    }
    if f.ab {
        r.skip(1, "placeholder count")?;
        let heroes = r.count(4096, "player hero count")?;
        for _ in 0..heroes {
            let hero_type = r.u8("player hero type")?;
            let name = r.string("player hero name")?.to_vec();
            player.hero_names.push((hero_type, name));
        }
    }
    Ok(player)
}

fn read_victory_loss(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    let id_width = if f.ab { 2 } else { 1 };
    let at = r.pos();
    let victory = r.u8("victory condition")?;
    if victory != 0xFF {
        r.skip(2, "victory flags")?;
        let extra = match victory {
            0 => id_width,     // acquire artifact
            1 => id_width + 4, // accumulate creatures
            2 => 5,            // accumulate resources
            3 => 5,            // upgrade town
            4..=7 => 3,        // grail / hero / town / monster position
            8 | 9 | 11 => 0,   // dwellings, mines, eliminate monsters
            10 => 4,           // transport artifact
            12 => 4,           // survive
            _ => {
                return Err(MapError::Invalid {
                    offset: at,
                    what: "victory condition",
                });
            }
        };
        r.skip(extra, "victory condition data")?;
    }
    let at = r.pos();
    let loss = r.u8("loss condition")?;
    if loss != 0xFF {
        let extra = match loss {
            0 | 1 => 3,
            2 => 2,
            _ => {
                return Err(MapError::Invalid {
                    offset: at,
                    what: "loss condition",
                });
            }
        };
        r.skip(extra, "loss condition data")?;
    }
    Ok(())
}

// ------------------------------------------------------------------ body --

struct Template {
    class: u32,
    subclass: u32,
}

fn read_body(
    r: &mut Reader<'_>,
    header: &MapHeader,
    f: Features,
    limits: MapLimits,
) -> Result<MapBody, MapError> {
    if header.format == MapFormat::WakeOfGods {
        return Err(MapError::Unsupported {
            what: "In the Wake of Gods map body".into(),
        });
    }
    if let Some(subrevision) = f.hota
        && subrevision > MAX_HOTA_SUBREVISION
    {
        return Err(MapError::Unsupported {
            what: format!(
                "HotA map format {subrevision} (newest supported: {MAX_HOTA_SUBREVISION})"
            ),
        });
    }

    // Map options.
    r.skip(31, "reserved header bytes")?;
    if f.is_hota() {
        r.skip(4, "special months flag")?;
    }
    if f.hota_after(0) {
        let combined = r.count(4096, "combined artifact count")?;
        r.skip(combined.div_ceil(8), "combined artifacts")?;
    }
    if f.hota_after(2) {
        r.skip(4, "round limit")?;
    }
    if f.hota_after(4) {
        r.skip(PLAYER_SLOTS, "hero recruitment flags")?;
    }
    if f.hota_after(8) && r.bool("event system flag")? {
        return Err(MapError::Unsupported {
            what: "HotA scripted events (map uses the 1.8 event system)".into(),
        });
    }

    // Allowed artifacts, spells and skills.
    if f.is_hota() {
        let artifacts = r.count(65_536, "allowed artifact count")?;
        r.skip(artifacts.div_ceil(8), "allowed artifacts")?;
    } else if f.ab {
        r.skip(if f.sod { 18 } else { 17 }, "allowed artifacts")?;
    }
    if f.sod {
        r.skip(9 + 4, "allowed spells and skills")?;
    }

    let rumors = r.count(100_000, "rumor count")?;
    for _ in 0..rumors {
        r.skip_string("rumor name")?;
        r.skip_string("rumor text")?;
    }

    let mut body = MapBody::default();

    // Predefined heroes.
    if f.sod {
        let heroes = if f.is_hota() {
            r.count(4096, "predefined hero count")?
        } else {
            156
        };
        for hero_type in 0..heroes {
            if !r.bool("predefined hero flag")? {
                continue;
            }
            if let Ok(hero_type) = u8::try_from(hero_type) {
                body.predefined_hero_types.push(hero_type);
            }
            if r.bool("experience flag")? {
                r.skip(4, "experience")?;
            }
            if r.bool("skills flag")? {
                let skills = r.count(64, "skill count")?;
                r.skip(skills * 2, "skills")?;
            }
            skip_hero_artifacts(r, f)?;
            if r.bool("biography flag")? {
                r.skip_string("biography")?;
            }
            r.skip(1, "gender")?;
            if r.bool("spells flag")? {
                r.skip(9, "spells")?;
            }
            if r.bool("primary skills flag")? {
                r.skip(4, "primary skills")?;
            }
        }
        if f.hota_after(4) {
            r.skip(heroes * 6, "hero level settings")?;
        }
    }

    // Terrain: seven bytes per tile.
    let levels = if header.has_underground { 2 } else { 1 };
    let tiles = (header.size as usize) * (header.size as usize) * levels;
    r.skip(tiles * 7, "terrain")?;

    // Object templates.
    let template_count = r.count(limits.max_objects, "object template count")?;
    let mut templates = Vec::with_capacity(template_count.min(4096));
    for _ in 0..template_count {
        r.skip_string("template sprite name")?;
        r.skip(6 + 6 + 2 + 2, "template masks")?;
        let class = r.u32("template class")?;
        let subclass = r.u32("template subclass")?;
        r.skip(1 + 1 + 16, "template tail")?;
        templates.push(Template { class, subclass });
    }

    // Object instances.
    let object_count = r.count(limits.max_objects, "object count")?;
    let mut classes: BTreeMap<u32, usize> = BTreeMap::new();
    for object_id in 0..object_count {
        let anchor = read_position(r, "object position")?;
        let at = r.pos();
        let index = r.u32("object template index")? as usize;
        let template = templates.get(index).ok_or(MapError::Invalid {
            offset: at,
            what: "object template index",
        })?;
        r.skip(5, "object padding")?;
        *classes.entry(template.class).or_default() += 1;
        if template.class == 57 {
            if u32::from(anchor.x) >= header.size
                || u32::from(anchor.y) >= header.size
                || usize::from(anchor.z) >= levels
            {
                return Err(MapError::Invalid {
                    offset: at,
                    what: "obelisk position",
                });
            }
            body.obelisks.push((object_id, anchor));
        }
        read_object(r, f, template, anchor, &mut body)?;
    }
    body.object_count = object_count;
    body.object_classes = classes.into_iter().collect();

    // Global timed events.
    let events = r.count(100_000, "event count")?;
    for _ in 0..events {
        skip_event_common(r, f)?;
        if f.hota_after(4) && !f.hota_after(6) {
            r.skip(14, "event tail")?;
        }
    }
    body.trailing_bytes = r.remaining();
    Ok(body)
}

fn artifact_width(f: Features) -> usize {
    if f.ab { 2 } else { 1 }
}

/// Artifact ID plus, from HotA format 5, the scroll spell word.
fn skip_artifact(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    let width = artifact_width(f) + if f.hota_after(4) { 2 } else { 0 };
    r.skip(width, "artifact")
}

fn skip_hero_artifacts(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    if !r.bool("artifacts flag")? {
        return Ok(());
    }
    for _ in 0..f.artifact_slots() {
        skip_artifact(r, f)?;
    }
    let backpack = r.u16("backpack count")?;
    for _ in 0..backpack {
        skip_artifact(r, f)?;
    }
    Ok(())
}

/// Seven `(creature, count)` stacks.
fn skip_creature_set(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    r.skip(7 * (artifact_width(f) + 2), "creature stacks")
}

fn skip_message_and_guards(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    if r.bool("message flag")? {
        r.skip_string("message")?;
        if r.bool("guards flag")? {
            skip_creature_set(r, f)?;
        }
        r.skip(4, "message padding")?;
    }
    Ok(())
}

/// HotA 1.8 script reference: handler ID and a flag.
fn skip_event_handler(r: &mut Reader<'_>) -> Result<(), MapError> {
    r.skip(5, "event handler")
}

fn skip_box_content(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    skip_message_and_guards(r, f)?;
    r.skip(4 + 4 + 1 + 1 + 28 + 4, "box rewards")?;
    let skills = r.u8("box skill count")? as usize;
    r.skip(skills * 2, "box skills")?;
    let artifacts = r.u8("box artifact count")?;
    for _ in 0..artifacts {
        skip_artifact(r, f)?;
    }
    let spells = r.u8("box spell count")? as usize;
    r.skip(spells, "box spells")?;
    let creatures = r.u8("box creature count")? as usize;
    r.skip(creatures * (artifact_width(f) + 2), "box creatures")?;
    r.skip(8, "box padding")
}

fn skip_box_hota(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    if f.hota_after(4) {
        r.skip(8, "box movement")?;
    }
    if f.hota_after(5) {
        r.skip(4, "box difficulty mask")?;
    }
    if f.hota_after(8) && r.bool("box script flag")? {
        skip_event_handler(r)?;
    }
    Ok(())
}

fn skip_event_common(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    r.skip_string("event name")?;
    r.skip_string("event message")?;
    r.skip(28 + 1, "event resources and players")?;
    if f.sod {
        r.skip(1, "event human flag")?;
    }
    r.skip(1 + 2 + 2 + 16, "event schedule")?;
    if f.hota_after(6) {
        r.skip(4, "event difficulty mask")?;
    }
    if f.hota_after(8) && r.bool("event script flag")? {
        skip_event_handler(r)?;
    }
    Ok(())
}

/// Reads a quest; returns whether it carries a mission.
fn skip_quest(r: &mut Reader<'_>, f: Features) -> Result<bool, MapError> {
    let at = r.pos();
    let mission = r.u8("quest mission")?;
    match mission {
        0 => return Ok(false),
        1 => r.skip(4, "quest primary skills")?,
        2..=4 => r.skip(4, "quest target")?,
        5 => {
            let artifacts = r.u8("quest artifact count")?;
            for _ in 0..artifacts {
                skip_artifact(r, f)?;
            }
        }
        6 => {
            let stacks = r.u8("quest creature count")? as usize;
            r.skip(stacks * (artifact_width(f) + 2), "quest creatures")?;
        }
        7 => r.skip(28, "quest resources")?,
        8 | 9 => r.skip(1, "quest hero or player")?,
        10 if f.is_hota() => {
            let kind_at = r.pos();
            match r.u32("quest kind")? {
                0 => {
                    let classes = r.count(4096, "quest hero class count")?;
                    r.skip(classes.div_ceil(8), "quest hero classes")?;
                }
                1 | 2 => r.skip(4, "quest value")?,
                3 => skip_event_handler(r)?,
                _ => {
                    return Err(MapError::Invalid {
                        offset: kind_at,
                        what: "quest kind",
                    });
                }
            }
        }
        _ => {
            return Err(MapError::Invalid {
                offset: at,
                what: "quest mission",
            });
        }
    }
    r.skip(4, "quest deadline")?;
    for _ in 0..3 {
        r.skip_string("quest text")?;
    }
    Ok(true)
}

fn skip_quest_timeout(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    if f.hota_after(9) {
        r.skip_string("quest timeout text")?;
    }
    Ok(())
}

fn skip_seer_quest(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    let has_mission = if f.ab {
        skip_quest(r, f)?
    } else {
        r.u8("seer hut artifact")? != 0xFF
    };
    if !has_mission {
        return r.skip(1, "seer hut padding");
    }
    skip_quest_timeout(r, f)?;
    let at = r.pos();
    let extra = match r.u8("seer hut reward")? {
        0 => 0,
        1 | 2 => 4,
        3 | 4 | 9 => 1,
        5 => 5,
        6 | 7 => 2,
        8 => artifact_width(f) + if f.hota_after(4) { 2 } else { 0 },
        10 => artifact_width(f) + 2,
        _ => {
            return Err(MapError::Invalid {
                offset: at,
                what: "seer hut reward",
            });
        }
    };
    r.skip(extra, "seer hut reward data")
}

/// The 18-byte HotA reward block shared by several small objects.
fn skip_hota_reward(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    if f.hota_after(4) {
        r.skip(18, "reward block")?;
    }
    Ok(())
}

fn skip_content_pair(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    if f.hota_after(4) {
        r.skip(8, "reward content")?;
    }
    Ok(())
}

fn owner_byte(raw: u8) -> Option<u8> {
    (raw < PLAYER_SLOTS as u8).then_some(raw)
}

fn read_hero(
    r: &mut Reader<'_>,
    f: Features,
    kind: MapHeroKind,
    anchor: Position,
) -> Result<MapHero, MapError> {
    let identifier = if f.ab {
        Some(r.u32("hero identifier")?)
    } else {
        None
    };
    let owner = owner_byte(r.u8("hero owner")?);
    let hero_type = portrait_byte(r.u8("hero type")?);
    let name_raw = if r.bool("hero name flag")? {
        Some(r.string("hero name")?.to_vec())
    } else {
        None
    };
    let experience = if f.sod {
        if r.bool("hero experience flag")? {
            Some(r.u32("hero experience")?)
        } else {
            None
        }
    } else {
        Some(r.u32("hero experience")?)
    };
    let portrait = if r.bool("hero portrait flag")? {
        portrait_byte(r.u8("hero portrait")?)
    } else {
        None
    };
    if r.bool("hero skills flag")? {
        let skills = r.count(64, "hero skill count")?;
        r.skip(skills * 2, "hero skills")?;
    }
    if r.bool("hero garrison flag")? {
        skip_creature_set(r, f)?;
    }
    r.skip(1, "hero formation")?;
    skip_hero_artifacts(r, f)?;
    r.skip(1, "hero patrol radius")?;
    if f.ab {
        if r.bool("hero biography flag")? {
            r.skip_string("hero biography")?;
        }
        r.skip(1, "hero gender")?;
    }
    if f.sod {
        if r.bool("hero spells flag")? {
            r.skip(9, "hero spells")?;
        }
        if r.bool("hero primary skills flag")? {
            r.skip(4, "hero primary skills")?;
        }
    } else if f.ab {
        r.skip(1, "hero spell")?;
    }
    r.skip(16, "hero padding")?;
    if f.hota_after(4) {
        r.skip(6, "hero level settings")?;
    }
    Ok(MapHero {
        kind,
        anchor,
        hero_type,
        owner,
        identifier,
        name_raw,
        portrait,
        experience,
    })
}

fn read_town(
    r: &mut Reader<'_>,
    f: Features,
    faction: Option<u32>,
    anchor: Position,
) -> Result<MapTown, MapError> {
    let identifier = if f.ab {
        Some(r.u32("town identifier")?)
    } else {
        None
    };
    let owner = owner_byte(r.u8("town owner")?);
    let name_raw = if r.bool("town name flag")? {
        Some(r.string("town name")?.to_vec())
    } else {
        None
    };
    let has_garrison = r.bool("town garrison flag")?;
    if has_garrison {
        skip_creature_set(r, f)?;
    }
    r.skip(1, "town formation")?;
    let buildings = if r.bool("town buildings flag")? {
        let built = r.bytes(6, "town built buildings")?;
        let forbidden = r.bytes(6, "town forbidden buildings")?;
        Some(TownBuildings {
            built: built.try_into().expect("length 6"),
            forbidden: forbidden.try_into().expect("length 6"),
        })
    } else {
        r.skip(1, "town fort flag")?;
        None
    };
    if f.ab {
        r.skip(9, "town obligatory spells")?;
    }
    r.skip(9, "town possible spells")?;
    if f.hota_after(0) {
        r.skip(1, "town spell research flag")?;
    }
    if f.hota_after(4) {
        let special = r.count(4096, "town special building count")?;
        r.skip(special, "town special buildings")?;
    }
    let event_count = r.count(100_000, "town event count")?;
    for _ in 0..event_count {
        skip_event_common(r, f)?;
        if f.hota_after(4) {
            r.skip(4 + 4 + 6, "town event HotA data")?;
        }
        if f.hota_after(6) {
            r.skip(1, "town event neutral flag")?;
        }
        r.skip(6 + 14 + 4, "town event buildings and creatures")?;
    }
    if f.sod {
        r.skip(1, "town alignment")?;
    }
    r.skip(3, "town padding")?;
    Ok(MapTown {
        anchor,
        faction,
        owner,
        identifier,
        name_raw,
        has_garrison,
        buildings,
        event_count: event_count as u32,
    })
}

fn read_object(
    r: &mut Reader<'_>,
    f: Features,
    template: &Template,
    anchor: Position,
    body: &mut MapBody,
) -> Result<(), MapError> {
    let sub = template.subclass;
    match template.class {
        // Event.
        26 => {
            skip_box_content(r, f)?;
            r.skip(1 + 1 + 1 + 4, "event flags")?;
            if f.hota_after(2) {
                r.skip(1, "event human flag")?;
            }
            skip_box_hota(r, f)
        }
        // Pandora's box.
        6 => {
            skip_box_content(r, f)?;
            if f.hota_after(4) {
                r.skip(1, "pandora padding")?;
            }
            skip_box_hota(r, f)
        }
        // Hero, prison, random hero.
        34 | 62 | 70 => {
            let kind = match template.class {
                34 => MapHeroKind::Hero,
                62 => MapHeroKind::Prison,
                _ => MapHeroKind::Random,
            };
            let hero = read_hero(r, f, kind, anchor)?;
            body.heroes.push(hero);
            Ok(())
        }
        // Town, random town.
        98 | 77 => {
            let faction = (template.class == 98).then_some(sub);
            let town = read_town(r, f, faction, anchor)?;
            body.towns.push(town);
            Ok(())
        }
        // Monsters, fixed and random.
        54 | 71..=75 | 162..=164 => {
            if f.ab {
                r.skip(4, "monster identifier")?;
            }
            let count = r.u16("monster count")?;
            r.skip(1, "monster disposition")?;
            body.neutral_armies.push(crate::MapArmy {
                class: template.class,
                anchor,
                stacks: vec![((template.class == 54).then_some(sub), u32::from(count))],
            });
            if r.bool("monster message flag")? {
                r.skip_string("monster message")?;
                r.skip(28 + artifact_width(f), "monster reward")?;
            }
            r.skip(1 + 1 + 2, "monster flags")?;
            if f.hota_after(2) {
                r.skip(4 + 1 + 4 + 4 + 4, "monster HotA settings")?;
            }
            if f.hota_after(4) {
                r.skip(1 + 4, "monster value settings")?;
            }
            Ok(())
        }
        // Ocean bottle, sign.
        59 | 91 => {
            r.skip_string("sign text")?;
            r.skip(4, "sign padding")
        }
        // Seer's hut.
        83 => {
            let quests = if f.hota_after(2) {
                r.count(4096, "seer hut quest count")?
            } else {
                1
            };
            for _ in 0..quests {
                skip_seer_quest(r, f)?;
            }
            if f.hota_after(2) {
                let repeatable = r.count(4096, "seer hut repeatable quest count")?;
                for _ in 0..repeatable {
                    skip_seer_quest(r, f)?;
                }
            }
            if f.hota_after(9) && r.bool("seer name flag")? {
                r.skip_string("seer name")?;
            }
            r.skip(2, "seer hut padding")
        }
        // Witch hut.
        113 => {
            if f.ab {
                r.skip(4, "witch hut skills")?;
            }
            Ok(())
        }
        // Scholar.
        81 => r.skip(1 + 1 + 6, "scholar"),
        // Garrisons.
        33 | 219 => {
            r.skip(4, "garrison owner")?;
            let mut stacks = Vec::new();
            for _ in 0..7 {
                let id = if f.ab {
                    u32::from(r.u16("garrison creature")?)
                } else {
                    u32::from(r.u8("garrison creature")?)
                };
                let empty = if f.ab { 0xFFFF } else { 0xFF };
                stacks.push((
                    (id != empty).then_some(id),
                    u32::from(r.u16("garrison count")?),
                ));
            }
            body.neutral_armies.push(crate::MapArmy {
                class: template.class,
                anchor,
                stacks,
            });
            if f.ab {
                r.skip(1, "garrison removable flag")?;
            }
            r.skip(8, "garrison padding")
        }
        // Artifacts, fixed and random.
        5 | 65..=69 => {
            skip_message_and_guards(r, f)?;
            if f.hota_after(4) {
                r.skip(5, "artifact pickup settings")?;
            }
            Ok(())
        }
        // Spell scroll.
        93 => {
            skip_message_and_guards(r, f)?;
            r.skip(4, "scroll spell")
        }
        // Resources, fixed and random.
        76 | 79 => {
            skip_message_and_guards(r, f)?;
            r.skip(4 + 4, "resource amount")
        }
        // Mine; subclass 7 and the dedicated class are abandoned mines.
        53 if sub < 7 => r.skip(4, "mine owner"),
        53 | 220 => {
            r.skip(4, "abandoned mine resources")?;
            if f.hota_after(4) {
                r.skip(1 + 12, "abandoned mine guards")?;
            }
            Ok(())
        }
        // Creature dwellings, shipyard, lighthouse: owner only.
        17..=20 | 87 | 42 => r.skip(4, "owner"),
        // Shrines.
        88..=90 => r.skip(4, "shrine spell"),
        // Grail (HotA reuses the class with subclass >= 1000 for markers).
        36 => {
            if sub < 1000 {
                body.grails.push((anchor, r.u32("grail radius")?));
            }
            Ok(())
        }
        // Random dwellings.
        216..=218 => {
            r.skip(4, "dwelling owner")?;
            if template.class != 218 {
                let identifier = r.u32("dwelling town identifier")?;
                if identifier == 0 {
                    r.skip(f.faction_mask_bytes(), "dwelling factions")?;
                }
            }
            if template.class != 217 {
                r.skip(2, "dwelling level range")?;
            }
            Ok(())
        }
        // Quest guard.
        215 => {
            skip_quest(r, f)?;
            skip_quest_timeout(r, f)
        }
        // Hero placeholder.
        214 => {
            r.skip(1, "placeholder owner")?;
            if r.u8("placeholder hero type")? == 0xFF {
                r.skip(1, "placeholder power rank")?;
            }
            if f.hota_after(4) {
                r.skip(1 + 7 * 8, "placeholder units")?;
                let artifacts = r.count(100, "placeholder artifact count")?;
                r.skip(artifacts * 4, "placeholder artifacts")?;
            }
            Ok(())
        }
        // Creature banks.
        16 | 24 | 25 | 84 | 85 => {
            if f.hota_after(2) {
                r.skip(4 + 1, "bank settings")?;
                let artifacts = r.count(4096, "bank artifact count")?;
                r.skip(artifacts * 4, "bank artifacts")?;
            }
            Ok(())
        }
        // Pyramid, chests, corpse, tomb, survivor, flotsam, tree of knowledge.
        63 | 101 | 22 | 108 | 86 | 82 | 29 | 102 => skip_content_pair(r, f),
        // Campfire, lean-to, wagon.
        12 | 39 | 105 => skip_hota_reward(r, f),
        // Border gate; HotA quest gate and grave share the class.
        212 => match sub {
            1000 => {
                skip_quest(r, f)?;
                skip_quest_timeout(r, f)
            }
            1001 => skip_hota_reward(r, f),
            _ => Ok(()),
        },
        // HotA collectible objects.
        145 => match sub {
            0 | 1 => skip_hota_reward(r, f),
            _ => skip_content_pair(r, f),
        },
        // HotA visitable objects; subclass 0 is the Seafaring Academy.
        146 if sub == 0 => skip_university(r, f),
        // HotA decorative/visitable objects; subclass 12 is the Trapper Lodge.
        144 if sub == 12 => {
            if f.hota_after(8) {
                r.skip(16, "trapper lodge")?;
            }
            Ok(())
        }
        // Black market.
        7 => {
            if f.hota_after(4) {
                r.skip(7 * 4, "black market artifacts")?;
            }
            Ok(())
        }
        // University.
        104 => skip_university(r, f),
        // Everything else carries no instance data.
        _ => Ok(()),
    }
}

fn skip_university(r: &mut Reader<'_>, f: Features) -> Result<(), MapError> {
    if f.hota_after(4) {
        r.skip(4 + 4, "university skills")?;
    }
    Ok(())
}
