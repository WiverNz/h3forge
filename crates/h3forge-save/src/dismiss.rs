//! Hero dismissal for HotA 44/11 (H3F-026).
//!
//! Evidence: a real before/after pair saved around one native "Dismiss Hero"
//! action (SAVEGAME_FORMAT.md section I). The operation reproduces the
//! structural part of that diff as same-length patches:
//!
//! - hero front matter: `is_visible` 1 -> 0;
//! - hero record: owner -> 0xFF, temporary morale/luck -> 0, destination
//!   X/Y -> -1, visited-bonus block -> 0, army -> one creature in slot 0,
//!   spell points -> maximum, movement words -> bonus-free values (each
//!   only in the verified cases);
//! - hero owner table (one byte per hero type after the hero array) -> 0xFF;
//! - player record: hero count, selected hero and the dense hero slot list;
//! - adventure map tile under the hero: object restored from the hero's
//!   "object under" fields;
//! - boat record of a boarded boat: `exists` 1 -> 0.
//!
//! Deliberately NOT reproduced (documented deviations of a record that
//! stays valid for recruitment): movement and spell points of heroes whose
//! bonuses are not verified, and the replay event log, whose native update
//! is a length-changing insertion.
//!
//! Every structure is re-derived from the immutable payload and cross-checked
//! before a single patch is planned; anything outside the verified shape is
//! refused with an explanation instead of guessed.

use std::collections::BTreeMap;

use crate::{BytePatch, EditError, HeroConfidence, HeroOwner, SaveDocument, TownConfidence};

/// Fixed part of one adventure map tile; a `u32` count at +18 is followed by
/// that many 4-byte render entries.
const TILE_FIXED: usize = 22;
/// Sanity cap for one tile's render list during the validation walk.
const TILE_MAX_RENDER: usize = 4096;
const OBJECT_CLASS_HERO: u16 = 34;
const TERRAIN_WATER: u8 = 8;
/// Tile flag (second flag byte, tile +7): an actionable object sits here.
const TILE_FLAG_ACTIONABLE: u8 = 0x10;
const BOAT_LEN: usize = 28;
/// Obelisk count byte plus 48 masks directly before `Player[8]`.
const OBELISK_TABLE_LEN: usize = 49;
/// Tail bytes of the last hero struct between the located record end and the
/// hero owner table.
const OWNER_TABLE_GAP: usize = 12;
/// Bytes of fixed front matter before the biography text.
const FRONT_LEN: usize = 26;
const MAX_BIO: usize = 8 * 1024;
const ARMY_IDS: usize = 113;
const ARMY_COUNTS: usize = 141;
const SPELL_POINTS: usize = 47;
const KNOWLEDGE: usize = 241;
/// 30 mastery bytes indexed by skill ID.
const SKILL_MASTERY: usize = 182;
const SKILL_LOGISTICS: usize = 2;
const SKILL_NAVIGATION: usize = 5;
const SKILL_INTELLIGENCE: usize = 24;
const EQUIPMENT: usize = 382;
/// HotA words after the reservation counters: land and sea movement.
const LAND_WORD: usize = 1076;
const SEA_WORD: usize = 1080;
/// Land movement the game stores for a dismissed hero without bonuses.
const LAND_MOVEMENT: u32 = 2000;
/// Equestrian's Gloves, Boots of Speed, Wayfarer's Boots.
const LAND_MOVEMENT_ARTIFACTS: [u8; 3] = [70, 98, 151];
/// Necklace of Ocean Guidance, Sea Captain's Hat, Admiral's Hat.
const SEA_MOVEMENT_ARTIFACTS: [u8; 3] = [71, 123, 136];

/// Document-wide anchors every dismissal depends on.
struct Context {
    owner_table: usize,
    boats_start: usize,
    boat_count: usize,
    map_size: usize,
    total_tiles: usize,
    tile_search_end: usize,
}

/// Verified locations for one dismissible hero.
struct HeroPlan {
    id: usize,
    hero_type: u8,
    color: u8,
    start: usize,
    front: usize,
    tile: usize,
    props_under: [u8; 4],
    boat: Option<usize>,
}

fn refuse(reason: impl Into<String>) -> EditError {
    EditError::InvalidValue {
        field: "hero dismissal",
        reason: reason.into(),
    }
}

fn u32_at(payload: &[u8], at: usize) -> Option<u32> {
    let bytes = payload.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Walks `count` tiles forward from `start`; `None` on any implausible tile.
fn walk_tiles(payload: &[u8], mut at: usize, count: usize) -> Option<usize> {
    for _ in 0..count {
        let render = u32_at(payload, at.checked_add(18)?)? as usize;
        if render > TILE_MAX_RENDER {
            return None;
        }
        at = at.checked_add(TILE_FIXED + 4 * render)?;
    }
    Some(at)
}

/// The tile array is followed by `u32 template_count` and a `String16`
/// sprite name ending in ".def"; a walk that lands anywhere else was not
/// started on a tile boundary with the right index.
fn looks_like_template_table(payload: &[u8], at: usize) -> bool {
    let Some(count) = u32_at(payload, at) else {
        return false;
    };
    let Some(len_bytes) = payload.get(at + 4..at + 6) else {
        return false;
    };
    let len = u16::from_le_bytes([len_bytes[0], len_bytes[1]]) as usize;
    if count == 0 || count > 1_000_000 || !(5..=64).contains(&len) {
        return false;
    }
    let Some(name) = payload.get(at + 6..at + 6 + len) else {
        return false;
    };
    name.iter().all(|b| b.is_ascii_graphic()) && name[len - 4..].eq_ignore_ascii_case(b".def")
}

impl SaveDocument {
    fn dismissal_context(&self) -> Result<Context, String> {
        if !self.format.army_edit_supported() {
            return Err("Hero dismissal is verified for HotA 44/11 saves only.".into());
        }
        let payload = self.payload();
        if self.players.len() != 8 {
            return Err("The player table could not be verified in this save.".into());
        }
        if self.heroes.is_empty()
            || self.heroes.len() > 255
            || self.heroes.iter().enumerate().any(|(index, hero)| {
                hero.confidence != HeroConfidence::High
                    || payload.get(hero.span.start + 10).copied() != Some(index as u8)
            })
        {
            return Err(
                "Hero records are not a complete verified roster ordered by hero type.".into(),
            );
        }
        if self
            .towns
            .iter()
            .any(|town| town.confidence != TownConfidence::High)
        {
            return Err("Town records could not all be verified in this save.".into());
        }
        // Hero owner table: one byte per hero type, 12 bytes after the last
        // located record. It must agree with every record's owner byte.
        let owner_table = self.heroes.last().map(|h| h.span.end).unwrap_or(0) + OWNER_TABLE_GAP;
        let Some(table) = payload.get(owner_table..owner_table + self.heroes.len()) else {
            return Err("The hero owner table is out of bounds.".into());
        };
        let consistent = self
            .heroes
            .iter()
            .zip(table)
            .all(|(hero, &entry)| match hero.owner {
                HeroOwner::Player(color) => entry == color,
                HeroOwner::Neutral => entry > 7,
            });
        if !consistent {
            return Err("The hero owner table does not match the hero records.".into());
        }
        // Boat table: `u8 count` + 28-byte records ending at the obelisk
        // table that precedes Player[8]. The count is accepted only when the
        // stored id bytes enumerate the records and exactly one count fits.
        let players_start = self.players[0].start;
        let boats_end = players_start
            .checked_sub(OBELISK_TABLE_LEN)
            .ok_or("The boat table is out of bounds.")?;
        let fits = |count: usize| {
            let Some(start) = boats_end.checked_sub(count * BOAT_LEN) else {
                return false;
            };
            start >= 1
                && payload[start - 1] as usize == count
                && (0..count).all(|i| payload[start + i * BOAT_LEN + 1] as usize == i)
        };
        // A nonzero count is backed by its whole id chain. Zero boats also
        // "fits" whenever the last boat record ends in a zero byte, so it is
        // accepted only when no nonzero count fits.
        let mut nonzero = (1usize..=255).filter(|&count| fits(count));
        let boat_count = match (nonzero.next(), nonzero.next()) {
            (Some(count), None) => count,
            (None, _) if fits(0) => 0,
            _ => return Err("The boat table could not be located unambiguously.".into()),
        };
        let map_size = self.map.size.unwrap_or(0) as usize;
        let levels = match self.map.has_underground {
            Some(true) => 2,
            Some(false) => 1,
            None => 0,
        };
        if !(1..=252).contains(&map_size) || levels == 0 {
            return Err("Map size is unknown, so map tiles cannot be located.".into());
        }
        Ok(Context {
            owner_table,
            boats_start: boats_end - boat_count * BOAT_LEN,
            boat_count,
            map_size,
            total_tiles: map_size * map_size * levels,
            tile_search_end: boats_end,
        })
    }

    fn dismissal_hero(&self, ctx: &Context, id: usize) -> Result<HeroPlan, String> {
        let payload = self.payload();
        let hero = self
            .heroes
            .get(id)
            .ok_or("This hero record does not exist.")?;
        let HeroOwner::Player(color) = hero.owner else {
            return Err("This hero does not belong to a player.".into());
        };
        let start = hero.span.start;
        let hero_type = id as u8;
        // Towns: a garrisoned or visiting hero has town links and no map
        // tile of its own; no before/after evidence covers that case.
        for town in &self.towns {
            let bytes = &payload[town.span.clone()];
            if bytes[65] == hero_type {
                return Err(
                    "The hero is garrisoned in a town; dismissal from a garrison is not verified."
                        .into(),
                );
            }
            if bytes[66] == hero_type {
                return Err(
                    "The hero is visiting a town; dismissal from a town is not verified.".into(),
                );
            }
        }
        // Player roster: the hero must be a listed adventuring hero.
        let player = self
            .players
            .iter()
            .find(|p| p.color == color)
            .ok_or("The owning player record was not found.")?;
        // An invalid roster is never active; a valid one may still carry a
        // resource-only read-only reason, which does not concern dismissal.
        if let Some(reason) = &player.read_only_reason
            && !player.active
        {
            return Err(reason.clone());
        }
        let record = &payload[player.start..player.start + crate::players::RECORD_LEN];
        if record[3..11].iter().filter(|&&t| t == hero_type).count() != 1 {
            return Err("The hero is not in the owner's adventuring hero list.".into());
        }
        // Front matter: 26 fixed bytes, then the length-prefixed biography.
        let floor = start.saturating_sub(MAX_BIO);
        let mut bio = start;
        while bio > floor && payload[bio - 1] != 0 {
            bio -= 1;
        }
        let front = bio
            .checked_sub(FRONT_LEN)
            .filter(|_| u32_at(payload, bio.wrapping_sub(4)) == Some((start - bio) as u32))
            .ok_or("The hero's map position block could not be verified.")?;
        let f = &payload[front..front + FRONT_LEN];
        let (x, y, z) = (
            u16::from_le_bytes([f[0], f[1]]) as usize,
            u16::from_le_bytes([f[2], f[3]]) as usize,
            u16::from_le_bytes([f[4], f[5]]) as usize,
        );
        let levels = ctx.total_tiles / (ctx.map_size * ctx.map_size);
        if f[6] != 1 {
            return Err("The hero is not standing on the adventure map.".into());
        }
        let packed = (x as u32) | ((y as u32) << 16) | ((z as u32) << 26);
        if x >= ctx.map_size
            || y >= ctx.map_size
            || z >= levels
            || u32_at(f, 7) != Some(packed)
            || f[20] > 1
            || f[21] > 1
        {
            return Err("The hero's map position block could not be verified.".into());
        }
        let props_under = [f[16], f[17], f[18], f[19]];
        if u32_at(f, 11) != Some(0)
            || f[15] != 0
            || (props_under != [0; 4] && props_under != [0xFF; 4])
        {
            return Err(
                "The hero stands on a map object; move the hero to an empty tile in the game first."
                    .into(),
            );
        }
        // Map tile: signature candidates are validated by walking the rest
        // of the tile array from the hero's index to the template table.
        let index = z * ctx.map_size * ctx.map_size + y * ctx.map_size + x;
        let type_word = (hero_type as u32).to_le_bytes();
        let mut tile = None;
        let end = ctx.tile_search_end.saturating_sub(TILE_FIXED);
        for at in 0..end {
            let t = &payload[at..at + TILE_FIXED];
            if t[8..10] != OBJECT_CLASS_HERO.to_le_bytes()
                || t[12..14] != [0xFF, 0xFF]
                || t[14..18] != type_word
                || t[7] & TILE_FLAG_ACTIONABLE == 0
            {
                continue;
            }
            let landed = walk_tiles(payload, at, ctx.total_tiles - index);
            if landed.is_some_and(|landed| {
                landed < ctx.tile_search_end && looks_like_template_table(payload, landed)
            }) {
                if tile.is_some() {
                    return Err("The hero's map tile is ambiguous.".into());
                }
                tile = Some(at);
            }
        }
        let tile = tile.ok_or("The hero's map tile could not be located.")?;
        // Boat: a boarded boat disappears with its hero.
        let mut boat = None;
        for i in 0..ctx.boat_count {
            let at = ctx.boats_start + i * BOAT_LEN;
            let b = &payload[at..at + BOAT_LEN];
            if b[0] == 0 || b[5] != hero_type || b[7] != 1 {
                continue;
            }
            if boat.is_some() || b[0] != 1 || b[4] != color || b[14] != 0 {
                return Err("The hero's boat record is ambiguous.".into());
            }
            boat = Some(at);
        }
        let on_water = payload[tile] == TERRAIN_WATER;
        if on_water != boat.is_some() {
            return Err(if on_water {
                "The hero is on water without a boarded boat (Fly or Water Walk); this case is not verified."
                    .into()
            } else {
                "The hero's boat record does not match the map tile.".into()
            });
        }
        Ok(HeroPlan {
            id,
            hero_type,
            color,
            start,
            front,
            tile,
            props_under,
            boat,
        })
    }

    /// Why this hero cannot be dismissed, or `None` when every structure the
    /// operation touches is verified. Read-only; computed from the immutable
    /// payload on each call.
    pub fn hero_dismissal_blocker(&self, id: usize) -> Option<String> {
        let ctx = match self.dismissal_context() {
            Ok(ctx) => ctx,
            Err(reason) => return Some(reason),
        };
        let plan = match self.dismissal_hero(&ctx, id) {
            Ok(plan) => plan,
            Err(reason) => return Some(reason),
        };
        self.last_hero_blocker(plan.color, 1)
    }

    /// A player without towns must keep one adventuring hero: an empty
    /// roster with no town is an eliminated player, which the game settles
    /// with state this operation does not model.
    fn last_hero_blocker(&self, color: u8, removed: usize) -> Option<String> {
        let player = self.players.iter().find(|p| p.color == color)?;
        (player.town_count == 0 && usize::from(player.hero_count) <= removed).then(|| {
            "This is the last hero of a player without towns; dismissing it would eliminate the player."
                .to_string()
        })
    }

    /// Plans the dismissal of the given heroes as checked same-length
    /// patches. All heroes are planned together because heroes of one player
    /// share that player's roster bytes. Refuses the whole set when any hero
    /// is outside the verified shape.
    pub fn plan_hero_dismissals(&self, ids: &[usize]) -> Result<Vec<BytePatch>, EditError> {
        if !self.format.army_edit_supported() {
            return Err(EditError::UnsupportedFormat {
                family: self.format.family_title(),
                major: self.format.major,
                minor: self.format.minor,
            });
        }
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ctx = self.dismissal_context().map_err(refuse)?;
        let payload = self.payload();
        let mut plans: BTreeMap<usize, HeroPlan> = BTreeMap::new();
        for &id in ids {
            if id >= self.heroes.len() {
                return Err(EditError::HeroNotFound { id });
            }
            let plan = self.dismissal_hero(&ctx, id).map_err(refuse)?;
            if plans.insert(id, plan).is_some() {
                return Err(refuse("A hero is listed twice."));
            }
        }
        let mut patches = Vec::new();
        let mut put = |offset: usize, replacement: &[u8], reason: String| {
            // Split into changed runs so untouched bytes carry no patch.
            let current = &payload[offset..offset + replacement.len()];
            let mut i = 0;
            while i < replacement.len() {
                if current[i] == replacement[i] {
                    i += 1;
                    continue;
                }
                let run = i;
                while i < replacement.len() && current[i] != replacement[i] {
                    i += 1;
                }
                patches.push(BytePatch::new(
                    offset + run,
                    current[run..i].to_vec(),
                    replacement[run..i].to_vec(),
                    reason.clone(),
                ));
            }
        };
        let mut by_color: BTreeMap<u8, Vec<u8>> = BTreeMap::new();
        for plan in plans.values() {
            let who = format!("Dismiss hero {}", plan.id);
            by_color.entry(plan.color).or_default().push(plan.hero_type);
            put(plan.front + 6, &[0], format!("{who}: off the map"));
            put(plan.start, &[0xFF], format!("{who}: owner"));
            put(
                plan.start + 2,
                &[0, 0],
                format!("{who}: temporary morale and luck"),
            );
            put(plan.start + 19, &[0xFF; 8], format!("{who}: destination"));
            // Only bytes 109-111 of the visited block are cleared; the
            // land pair shows the game keeping other visited bytes (53, 81,
            // 85) of a dismissed hero.
            put(
                plan.start + 109,
                &[0; 3],
                format!("{who}: temporary visit bonuses"),
            );
            // Army: the game leaves one creature in slot 0 and blanks the
            // rest (`FF FF FF FF` id, zero count). The kept creature is the
            // first occupied stack's; see SAVEGAME_FORMAT.md I.3.
            let hero = &payload[plan.start..plan.start + ARMY_COUNTS + 28];
            let first = (0..7).find_map(|slot| {
                let id = &hero[ARMY_IDS + 4 * slot..ARMY_IDS + 4 * slot + 4];
                let count = &hero[ARMY_COUNTS + 4 * slot..ARMY_COUNTS + 4 * slot + 4];
                (id != [0xFF; 4] && count != [0; 4]).then(|| [id[0], id[1], id[2], id[3]])
            });
            if let Some(creature) = first {
                let mut army = [0xFFu8; 56];
                army[..4].copy_from_slice(&creature);
                army[28..].fill(0);
                army[28] = 1;
                put(plan.start + ARMY_IDS, &army, format!("{who}: army"));
            }
            // Spell points refill to the maximum: knowledge x 10, plus 20%
            // with Basic Intelligence. Higher Intelligence levels have no
            // sample, so those heroes keep their stored points.
            let knowledge = u16::from(payload[plan.start + KNOWLEDGE]);
            let per_knowledge = match payload[plan.start + SKILL_MASTERY + SKILL_INTELLIGENCE] {
                0 => Some(10),
                1 => Some(12),
                _ => None,
            };
            if let (Some(points), true) = (per_knowledge, (1..=99).contains(&knowledge)) {
                put(
                    plan.start + SPELL_POINTS,
                    &(knowledge * points).to_le_bytes(),
                    format!("{who}: spell points"),
                );
            }
            // Movement: the game recomputes it for the unowned hero. Only
            // the bonus-free cases are verified, so a hero with Logistics,
            // deeper Navigation or a worn movement artifact keeps its words.
            let wears = |ids: &[u8]| {
                (0..19).any(|slot| {
                    let at = plan.start + EQUIPMENT + 8 * slot;
                    payload[at + 1..at + 4] == [0, 0, 0] && ids.contains(&payload[at])
                })
            };
            if payload[plan.start + SKILL_MASTERY + SKILL_LOGISTICS] == 0
                && !wears(&LAND_MOVEMENT_ARTIFACTS)
            {
                let land = LAND_MOVEMENT.to_le_bytes();
                put(plan.start + 31, &land, format!("{who}: movement total"));
                put(plan.start + 35, &land, format!("{who}: movement left"));
                put(
                    plan.start + LAND_WORD,
                    &land,
                    format!("{who}: land movement"),
                );
            }
            let sea = match payload[plan.start + SKILL_MASTERY + SKILL_NAVIGATION] {
                0 => Some(1500u32),
                1 => Some(2250),
                _ => None,
            };
            if let (Some(sea), false) = (sea, wears(&SEA_MOVEMENT_ARTIFACTS)) {
                put(
                    plan.start + SEA_WORD,
                    &sea.to_le_bytes(),
                    format!("{who}: sea movement"),
                );
            }
            put(
                ctx.owner_table + plan.hero_type as usize,
                &[0xFF],
                format!("{who}: hero owner table"),
            );
            // Tile: clear the actionable flag, restore the (empty) object
            // class and the saved "object under" properties.
            let flags = payload[plan.tile + 7] & !TILE_FLAG_ACTIONABLE;
            put(
                plan.tile + 7,
                &[flags, 0, 0],
                format!("{who}: map tile object"),
            );
            put(
                plan.tile + 14,
                &plan.props_under,
                format!("{who}: map tile properties"),
            );
            if let Some(boat) = plan.boat {
                put(boat, &[0], format!("{who}: boarded boat"));
            }
        }
        for (color, removed) in by_color {
            if let Some(reason) = self.last_hero_blocker(color, removed.len()) {
                return Err(refuse(reason));
            }
            let player = self
                .players
                .iter()
                .find(|p| p.color == color)
                .ok_or_else(|| refuse("The owning player record was not found."))?;
            let record = &payload[player.start..player.start + 11];
            let mut roster = [0xFFu8; 10];
            let kept: Vec<u8> = record[3..11]
                .iter()
                .copied()
                .filter(|t| *t != 0xFF && !removed.contains(t))
                .collect();
            roster[0] = kept.len() as u8;
            roster[1] = if removed.contains(&record[2]) {
                0xFF
            } else {
                record[2]
            };
            roster[2..2 + kept.len()].copy_from_slice(&kept);
            put(
                player.start + 1,
                &roster,
                format!("Player {color}: adventuring hero list"),
            );
        }
        patches.into_iter().collect()
    }
}
