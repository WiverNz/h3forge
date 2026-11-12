//! Current hero coordinates and conservative editor relocation for HotA 44/11.
use crate::{BytePatch, EditError, HeroConfidence, HeroOwner, SaveDocument};

impl SaveDocument {
    /// Current, visible map position for a high-confidence HotA 44/11 hero.
    /// Missing/ambiguous biography boundaries, off-map heroes, inconsistent
    /// duplicate coordinates and out-of-map positions return no location.
    /// This does not establish destination passability or relocation support.
    pub fn hero_coordinates(&self, id: usize) -> Option<[u16; 3]> {
        if !self.format.army_edit_supported() {
            return None;
        }
        let hero = self.heroes.get(id)?;
        if hero.confidence != HeroConfidence::High {
            return None;
        }
        location(
            self.payload(),
            hero.span.start,
            self.map.size?,
            if self.map.has_underground? { 2 } else { 1 },
        )
    }
}

fn location(p: &[u8], start: usize, size: u32, levels: u16) -> Option<[u16; 3]> {
    position_block(p, start, size, levels).map(|(_, xyz)| xyz)
}

fn position_block(p: &[u8], start: usize, size: u32, levels: u16) -> Option<(usize, [u16; 3])> {
    if start > p.len() || start < 26 || !(1..=252).contains(&size) {
        return None;
    }
    let mut found = None;
    // Validate the stored biography length, rather than stopping at a zero
    // inside biography text. Never choose between multiple plausible starts.
    for len in 0..=(start - 26).min(8192) {
        let front = start - 26 - len;
        let f = &p[front..front + 26];
        if u32::from_le_bytes(f[22..26].try_into().ok()?) != len as u32
            || f[6] != 1
            || f[15] > 1
            || f[20] > 1
            || f[21] > 1
        {
            continue;
        }
        let xyz = [
            u16::from_le_bytes([f[0], f[1]]),
            u16::from_le_bytes([f[2], f[3]]),
            u16::from_le_bytes([f[4], f[5]]),
        ];
        if u32::from(xyz[0]) >= size || u32::from(xyz[1]) >= size || xyz[2] >= levels {
            continue;
        }
        let packed = u32::from(xyz[0]) | (u32::from(xyz[1]) << 16) | (u32::from(xyz[2]) << 26);
        if u32::from_le_bytes(f[7..11].try_into().ok()?) != packed {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some((front, xyz));
    }
    found
}

fn refuse(reason: impl Into<String>) -> EditError {
    EditError::InvalidValue {
        field: "coordinates",
        reason: reason.into(),
    }
}

struct Source {
    front: usize,
    xyz: [u16; 3],
    hero_type: u8,
    water: bool,
}

impl SaveDocument {
    fn relocation_source(&self, id: usize, tiles: &[usize]) -> Result<Source, EditError> {
        let xyz = self
            .hero_coordinates(id)
            .ok_or_else(|| refuse("Current hero position is not verified."))?;
        let hero = &self.heroes[id];
        let HeroOwner::Player(color) = hero.owner else {
            return Err(refuse("Unowned heroes cannot be relocated."));
        };
        let p = self.payload();
        let h = hero.span.start;
        let hero_type = p[h + 10];
        if self
            .towns
            .iter()
            .any(|t| t.confidence != crate::TownConfidence::High)
        {
            return Err(refuse("Town links are not fully verified."));
        }
        if !self.players.iter().any(|player| {
            player.color == color
                && p[player.start + 3..player.start + 11]
                    .iter()
                    .filter(|&&v| v == hero_type)
                    .count()
                    == 1
        }) {
            return Err(refuse(
                "Hero is not uniquely listed in its owner's adventure roster.",
            ));
        }
        let (front, _) = position_block(
            p,
            h,
            self.map.size.unwrap_or(0),
            if self.map.has_underground == Some(true) {
                2
            } else {
                1
            },
        )
        .ok_or_else(|| refuse("Hero position block is ambiguous."))?;
        if self
            .towns
            .iter()
            .any(|t| p[t.span.start + 65] == hero_type || p[t.span.start + 66] == hero_type)
        {
            return Err(refuse("Visiting or garrisoned heroes cannot be relocated."));
        }
        // Match the complete inactive adventure-spell prefix in both native
        // pairs; do not borrow classic in-memory field offsets for HotA.
        if p[h + 1] != 255 || p[h + 5..h + 10] != [255, 255, 255, 0, 255] {
            return Err(refuse(
                "Patrol or adventure-spell state is outside the verified relocation shape.",
            ));
        }
        if p[h + 1069] > 1 || p[front + 15] != 0 {
            return Err(refuse(
                "Movement state or an actionable object under the hero is unsupported.",
            ));
        }
        let size = self.map.size.unwrap_or(0) as usize;
        let tile = tiles[(xyz[2] as usize * size + xyz[1] as usize) * size + xyz[0] as usize];
        if p[tile] > 11 || p[tile] == 9 || p[tile + 7] & !0x12 != 0 {
            return Err(refuse(
                "Source terrain or accessibility state is unsupported.",
            ));
        }
        if p[tile + 8..tile + 10] != 34u16.to_le_bytes()
            || p[tile + 12..tile + 14] != [255, 255]
            || p[tile + 14..tile + 18] != u32::from(hero_type).to_le_bytes()
            || p[tile + 7] & 0x10 == 0
        {
            return Err(refuse("Current tile does not identify this hero."));
        }
        // Current tiles, not historical object entries, establish occupancy.
        if tiles
            .iter()
            .filter(|&&a| {
                p[a + 8..a + 10] == 34u16.to_le_bytes()
                    && p[a + 14..a + 18] == u32::from(hero_type).to_le_bytes()
            })
            .count()
            != 1
        {
            return Err(refuse("Hero tile occupancy is duplicated."));
        }
        let under = u32::from_le_bytes(
            p[front + 11..front + 15]
                .try_into()
                .map_err(|_| refuse("Truncated position."))?,
        );
        if !matches!(under, 0 | 3)
            || !matches!(
                &p[front + 16..front + 20],
                [0, 0, 0, 0] | [255, 255, 255, 255]
            )
        {
            return Err(refuse(
                "The object under this hero is outside the verified relocation shape.",
            ));
        }
        let water = p[tile] == 8;
        // Boarded boat coordinates intentionally stay stale during sailing.
        let end = self
            .players
            .first()
            .and_then(|v| v.start.checked_sub(49))
            .ok_or_else(|| refuse("Boat table boundary is unavailable."))?;
        let candidates: Vec<_> = (1..=255usize)
            .filter_map(|n| {
                let start = end.checked_sub(n * 28)?;
                (start > 0
                    && p[start - 1] as usize == n
                    && (0..n).all(|i| p[start + i * 28 + 1] as usize == i))
                .then_some((start, n))
            })
            .collect();
        let (start, n) = match candidates.as_slice() {
            [one] => *one,
            [] if end > 0 && p[end - 1] == 0 => (end, 0),
            _ => return Err(refuse("Boat table is ambiguous.")),
        };
        let boarded: Vec<_> = (0..n)
            .map(|i| start + i * 28)
            .filter(|&a| p[a] != 0 && p[a + 5] == hero_type && p[a + 7] != 0)
            .collect();
        if water {
            if boarded.len() != 1
                || p[boarded[0]] != 1
                || p[boarded[0] + 4] != color
                || p[boarded[0] + 7] != 1
                || p[boarded[0] + 14] != 0
            {
                return Err(refuse(
                    "Water relocation requires exactly one verified boarded boat.",
                ));
            }
        } else if !boarded.is_empty() {
            return Err(refuse("Land hero has a conflicting boarded boat."));
        }
        Ok(Source {
            front,
            xyz,
            hero_type,
            water,
        })
    }

    /// Availability of editor relocation. Destination legality is checked again
    /// by the planner; this does not imply that every coordinate is allowed.
    pub fn hero_relocation_blocker(&self, id: usize) -> Option<String> {
        let result = crate::neutral::current_map(self)
            .map_err(refuse)
            .and_then(|(tiles, _)| self.relocation_source(id, &tiles));
        result.err().map(|e| e.to_string())
    }

    /// Teleport-style editor operation: preserve movement points, facing and
    /// exploration; clear the stale route, set the observed moved-state byte.
    /// Same level and medium only; no town, boarding, landing or object visits.
    pub fn plan_hero_relocation(
        &self,
        id: usize,
        to: [u16; 3],
    ) -> Result<Vec<BytePatch>, EditError> {
        let (tiles, objects) = crate::neutral::current_map(self).map_err(refuse)?;
        let source = self.relocation_source(id, &tiles)?;
        let size = self.map.size.unwrap_or(0) as usize;
        if to[0] as usize >= size || to[1] as usize >= size || to[2] != source.xyz[2] {
            return Err(refuse(
                "Destination is outside the map or changes level; cross-level relocation is not verified.",
            ));
        }
        if to == source.xyz {
            return Ok(Vec::new());
        }
        if self
            .heroes
            .iter()
            .any(|h| h.id != id && self.hero_coordinates(h.id) == Some(to))
        {
            return Err(refuse("Another visible hero occupies the destination."));
        }
        let index = |v: [u16; 3]| (v[2] as usize * size + v[1] as usize) * size + v[0] as usize;
        let from = tiles[index(source.xyz)];
        let dest = tiles[index(to)];
        let p = self.payload();
        let f = source.front;
        // Only the same passive underlying shape is currently evidenced. This
        // still allows arbitrary distance; it is not a one-step movement tool.
        if p[dest + 7] & !2 != 0
            || p[dest + 12..dest + 14] != [255, 255]
            || p[dest + 8..dest + 10] != p[f + 11..f + 13]
            || p[dest + 14..dest + 18] != p[f + 16..f + 20]
        {
            return Err(refuse(
                "Destination is blocked, occupied, actionable or has a different underlying object state.",
            ));
        }
        if p[dest] > 11 || p[dest] == 9 || (p[dest] == 8) != source.water {
            return Err(refuse(
                "Destination terrain is invalid or requires boarding/disembarking.",
            ));
        }
        // Validate each currently drawn sprite's own mask. The global object
        // array also retains defeated/collected entries, so it alone is unsafe.
        let count = u32::from_le_bytes(
            p[dest + 18..dest + 22]
                .try_into()
                .map_err(|_| refuse("Truncated tile."))?,
        ) as usize;
        for i in 0..count {
            let a = dest + 22 + i * 4;
            let object_id = u16::from_le_bytes([p[a], p[a + 1]]) as usize;
            let (anchor, _, _, _) = objects
                .objects
                .get(object_id)
                .ok_or_else(|| refuse("Invalid render object."))?;
            let dx = anchor[0]
                .checked_sub(to[0] as u8)
                .filter(|v| *v < 8)
                .ok_or_else(|| refuse("Invalid sprite position."))?;
            let dy = anchor[1]
                .checked_sub(to[1] as u8)
                .filter(|v| *v < 6)
                .ok_or_else(|| refuse("Invalid sprite position."))?;
            let obj = objects.start + object_id * 5;
            let template = u16::from_le_bytes([p[obj + 3], p[obj + 4]]) as usize;
            let b = &objects
                .templates
                .get(template)
                .ok_or_else(|| refuse("Invalid sprite template."))?
                .1;
            let bit = 1u8 << (7 - dx);
            if anchor[2] != to[2] as u8
                || p[a + 2] != (dy << 4 | dx)
                || b[8 + 5 - dy as usize] & bit == 0
                || b[20 + 5 - dy as usize] & bit != 0
            {
                return Err(refuse(
                    "Destination sprite is impassable, actionable or inconsistent.",
                ));
            }
        }
        // No implicit combat/visit transitions, including guarded neighbours.
        for y in to[1].saturating_sub(1)..=(to[1] + 1).min(size as u16 - 1) {
            for x in to[0].saturating_sub(1)..=(to[0] + 1).min(size as u16 - 1) {
                let a = tiles[index([x, y, to[2]])];
                if p[a + 8..a + 10] == 54u16.to_le_bytes() && p[a + 7] & 0x10 != 0 {
                    return Err(refuse("Destination is guarded by a neighbouring monster."));
                }
            }
        }
        let mut patches = Vec::new();
        let mut put = |at: usize, bytes: &[u8]| {
            if p[at..at + bytes.len()] != *bytes {
                patches.push(BytePatch::new(
                    at,
                    p[at..at + bytes.len()].to_vec(),
                    bytes.to_vec(),
                    "Hero coordinates",
                ));
            }
        };
        for (i, v) in to.iter().enumerate() {
            put(f + 2 * i, &v.to_le_bytes());
        }
        put(
            f + 7,
            &(u32::from(to[0]) | (u32::from(to[1]) << 16) | (u32::from(to[2]) << 26)).to_le_bytes(),
        );
        put(from + 7, &[p[from + 7] & !0x10]);
        put(from + 8, &p[f + 11..f + 13]);
        put(from + 14, &p[f + 16..f + 20]);
        put(dest + 7, &[p[dest + 7] | 0x10]);
        put(dest + 8, &34u16.to_le_bytes());
        put(dest + 14, &u32::from(source.hero_type).to_le_bytes());
        let h = self.heroes[id].span.start;
        put(h + 19, &[255; 8]);
        put(h + 1069, &[1]);
        patches.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::location;

    #[test]
    fn validates_bounds_levels_duplicates_and_biography() {
        let mut p = vec![0; 34];
        p[0] = 12;
        p[2] = 23;
        p[4] = 1;
        p[6] = 1;
        p[7..11].copy_from_slice(&(12u32 | (23 << 16) | (1 << 26)).to_le_bytes());
        p[22] = 8;
        p[26..34].copy_from_slice(b"bio\0text");
        assert_eq!(location(&p, 34, 36, 2), Some([12, 23, 1]));
        assert_eq!(location(&p, 34, 36, 1), None);
        assert_eq!(location(&p, 34, 23, 2), None);
        assert_eq!(location(&p, 35, 36, 2), None);
        p[7] = 13;
        assert_eq!(location(&p, 34, 36, 2), None);
        p[7] = 12;
        p[6] = 0;
        assert_eq!(location(&p, 34, 36, 2), None);
        p[6] = 1;
        p[22] = 9;
        assert_eq!(location(&p, 34, 36, 2), None);
    }
}
