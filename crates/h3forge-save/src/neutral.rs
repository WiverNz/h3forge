//! Current adventure-map armies for exact HotA 44/11 (H3F-027).
//! H3M data is never used to locate a writable field. See SAVEGAME_FORMAT.md.
use crate::{ArmySlot, BytePatch, EditError, SaveDocument};

#[derive(Debug, Clone)]
pub struct NeutralArmy {
    /// Current save object index, not an H3M ordinal.
    pub id: usize,
    pub class: u16,
    pub subtype: u16,
    pub position: [u8; 3],
    pub anchor: [u8; 3],
    pub creature_id: Option<u32>,
    /// None means that current quantities are unverified, not an empty army.
    pub army: Option<[ArmySlot; 7]>,
    pub owner: Option<u8>,
    pub read_only_reason: Option<String>,
    pub allowed_creatures: Vec<u32>,
    record: Option<usize>,
    tile: usize,
    object: usize,
    replacements: Vec<(u32, u16)>,
}

#[derive(Debug, Clone, Default)]
pub struct NeutralArmies {
    pub groups: Vec<NeutralArmy>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct NeutralEdit {
    pub slot: u8,
    pub creature_id: Option<u32>,
    pub count: u32,
}

fn word(p: &[u8], a: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        p.get(a..a.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn dword(p: &[u8], a: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        p.get(a..a.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// Current object anchor, class, subtype and template action mask.
type ObjectEntry = ([u8; 3], u16, u16, [u8; 6]);
pub(crate) struct ObjectTable {
    pub(crate) objects: Vec<ObjectEntry>,
    pub(crate) start: usize,
    /// Full names and fixed template bytes, retained for compatible replacement.
    pub(crate) templates: Vec<(Vec<u8>, [u8; 33])>,
}

/// Render references bind a candidate tile walk to actual map coordinates.
/// Header bytes can otherwise accidentally form a second length-only walk.
fn render_matches(
    p: &[u8],
    at: usize,
    count: usize,
    index: usize,
    size: usize,
    objects: &ObjectTable,
) -> bool {
    let position = [index % size, index / size % size, index / (size * size)];
    (0..count).all(|i| {
        let entry = at + 22 + i * 4;
        let Some(id) = word(p, entry) else {
            return false;
        };
        let Some(&(anchor, _, _, _)) = objects.objects.get(id as usize) else {
            return false;
        };
        let dx = (anchor[0] as usize).checked_sub(position[0]);
        let dy = (anchor[1] as usize).checked_sub(position[1]);
        matches!((dx, dy), (Some(dx @ 0..=7), Some(dy @ 0..=5))
            if anchor[2] as usize == position[2]
                && p.get(entry + 2).copied() == Some((dy << 4 | dx) as u8))
    })
}

/// Validate the complete template/object arrays, not just a .def signature.
fn object_table(p: &[u8], start: usize) -> Option<ObjectTable> {
    let count = dword(p, start)? as usize;
    if !(1..=65535).contains(&count) {
        return None;
    }
    let mut at = start + 4;
    let mut templates = Vec::new();
    let mut full_templates = Vec::new();
    for _ in 0..count {
        let len = word(p, at)? as usize;
        if !(5..=64).contains(&len) {
            return None;
        }
        let name = p.get(at + 2..at + 2 + len)?;
        if !name.iter().all(u8::is_ascii_graphic) || !name[len - 4..].eq_ignore_ascii_case(b".def")
        {
            return None;
        }
        let name = name.to_vec();
        at += 2 + len;
        full_templates.push((name, p.get(at..at + 33)?.try_into().ok()?));
        templates.push((
            word(p, at + 26)?,
            word(p, at + 28)?,
            p.get(at + 20..at + 26)?.try_into().ok()?,
        ));
        at += 33;
    }
    let count = dword(p, at)? as usize;
    if !(1..=65535).contains(&count) {
        return None;
    }
    at += 4;
    let object_start = at;
    let mut objects = Vec::new();
    for _ in 0..count {
        let b = p.get(at..at + 5)?;
        let &(class, sub, mask) = templates.get(word(p, at + 3)? as usize)?;
        objects.push(([b[0], b[1], b[2]], class, sub, mask));
        at += 5;
    }
    Some(ObjectTable {
        objects,
        start: object_start,
        templates: full_templates,
    })
}

pub(crate) fn current_map(doc: &SaveDocument) -> Result<(Vec<usize>, ObjectTable), String> {
    if !doc.format.army_edit_supported() {
        return Err("Current map armies require HotA 44/11.".into());
    }
    let size = doc.map.size.unwrap_or(0) as usize;
    if !(1..=252).contains(&size) {
        return Err("Map dimensions are unavailable.".into());
    }
    let levels = match doc.map.has_underground {
        Some(true) => 2,
        Some(false) => 1,
        None => return Err("Map levels are unavailable.".into()),
    };
    let total = size * size * levels;
    let p = doc.payload();
    // Locate a complete template table after the minimum possible tile size.
    // A table candidate must also admit exactly one complete, bounded tile walk.
    // Bound both discovery work and the reverse-walk allocation. Larger or
    // heavily decorated layouts stay unavailable rather than exhausting memory.
    let ceiling = doc
        .players
        .first()
        .map(|v| v.start)
        .unwrap_or(p.len())
        .min(p.len())
        .min(16 * 1024 * 1024);
    let mut found = None;
    for start in total * 22..ceiling.saturating_sub(10) {
        let Some(len) = word(p, start + 4) else {
            continue;
        };
        let len = len as usize;
        if !(5..=64).contains(&len)
            || !p
                .get(start + 6 + len - 4..start + 6 + len)
                .is_some_and(|s| s.eq_ignore_ascii_case(b".def"))
        {
            continue;
        }
        let Some(objects) = object_table(p, start) else {
            continue;
        };
        let mut distances = vec![0u32; start + 1];
        distances[start] = 1;
        let mut beginnings = Vec::new();
        for at in (0..start.saturating_sub(21)).rev() {
            if p[at] > 11 {
                continue;
            }
            let render = dword(p, at + 18).unwrap_or(u32::MAX) as usize;
            if render > 4096 {
                continue;
            }
            let next = at + 22 + render * 4;
            if next <= start && distances[next] != 0 {
                let remaining = distances[next] as usize;
                if remaining > total
                    || !render_matches(p, at, render, total - remaining, size, &objects)
                {
                    continue;
                }
                distances[at] = distances[next] + 1;
                if distances[at] as usize == total + 1 && at < 30_000 {
                    beginnings.push(at);
                }
            }
        }
        if beginnings.len() != 1 {
            continue;
        }
        if found.is_some() {
            return Err("Current map tile boundaries are ambiguous.".into());
        }
        found = Some((beginnings[0], objects));
    }
    let (mut at, objects) =
        found.ok_or("Current map tile and object arrays could not be verified.")?;
    let mut tiles = Vec::with_capacity(total);
    for _ in 0..total {
        tiles.push(at);
        let render = dword(p, at + 18).ok_or("Truncated tile.")? as usize;
        at += 22 + render * 4;
    }
    Ok((tiles, objects))
}

fn discover(doc: &SaveDocument) -> Result<Vec<NeutralArmy>, String> {
    let (tiles, objects) = current_map(doc)?;
    let size = doc.map.size.unwrap_or(0) as usize;
    let p = doc.payload();
    let mut groups = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (index, &at) in tiles.iter().enumerate() {
        let class = word(p, at + 8).ok_or("Truncated map tile.")?;
        if p[at + 7] & 0x10 != 0 && matches!(class, 54 | 33 | 219) {
            let id = word(p, at + 12).ok_or("Missing map object index.")? as usize;
            let sub = word(p, at + 10).ok_or("Missing map object subtype.")?;
            let position = [
                (index % size) as u8,
                (index / size % size) as u8,
                (index / (size * size)) as u8,
            ];
            let &(anchor, object_class, object_sub, mask) = objects
                .objects
                .get(id)
                .ok_or("Invalid current object index.")?;
            // Sprite masks are 8 columns by 6 rows, anchored at bottom right.
            let dx = anchor[0].checked_sub(position[0]);
            let dy = anchor[1].checked_sub(position[1]);
            let actionable = match (dx, dy) {
                (Some(dx @ 0..=7), Some(dy @ 0..=5)) => {
                    mask[5 - dy as usize] & (1 << (7 - dx)) != 0
                }
                _ => false,
            };
            if !actionable
                || anchor[2] != position[2]
                || class != object_class
                || sub != object_sub
                || !seen.insert(id)
            {
                return Err("Current army object identity is inconsistent or duplicated.".into());
            }
            let garrison_index = dword(p, at + 14).map(|v| v as usize);
            let object = objects.start + id * 5;
            let replacements = if class == 54 {
                compatible_templates(p, object, &objects.templates)
            } else {
                Vec::new()
            };
            groups.push(NeutralArmy {
                id, class, subtype:sub, position, anchor, creature_id: (class == 54).then_some(u32::from(sub)),
                army: None, owner: None, record: garrison_index,
                tile:at, object, allowed_creatures:replacements.iter().map(|&(id,_)|id).collect(), replacements,
                read_only_reason: Some(if class == 54 {
                    "No unique validated current monster table was found. Initial H3M quantities are not current counts."
                } else { "The current garrison army table could not be verified." }.into()),
            });
        }
    }
    let garrisons: Vec<usize> = groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.class != 54)
        .map(|(i, _)| i)
        .collect();
    if !garrisons.is_empty()
        && let Some(start) = garrison_table(doc, &groups, &garrisons)
    {
        for &i in &garrisons {
            let g = &mut groups[i];
            let Some(index) = g.record else {
                continue;
            };
            let record = start + index * 61;
            let army = std::array::from_fn(|slot| ArmySlot {
                creature_id: dword(p, record + 1 + slot * 4).unwrap_or(u32::MAX),
                count: dword(p, record + 29 + slot * 4).unwrap_or(0),
            });
            g.army = Some(army);
            g.owner = Some(p[record]);
            g.record = Some(record);
            g.read_only_reason = if g.class != 33 || g.subtype > 1 {
                Some("This current garrison class/subtype has no verified editing fixture.".into())
            } else {
                army.iter().any(|s| !valid_slot(*s)).then(|| {
                    "Unknown creature IDs or inconsistent empty slots: this army is read-only."
                        .into()
                })
            };
        }
    } else {
        for &i in &garrisons {
            groups[i].record = None;
        }
    }
    if let Some(start) = monster_table(doc, &groups) {
        for g in groups.iter_mut().filter(|g| g.class == 54) {
            let Some(index) = g.record else {
                continue;
            };
            let record = start + 33 * index;
            let count = dword(p, record).unwrap_or(0);
            let slot = ArmySlot {
                creature_id: u32::from(g.subtype),
                count,
            };
            g.army = Some(std::array::from_fn(|i| {
                if i == 0 {
                    slot
                } else {
                    ArmySlot {
                        creature_id: u32::MAX,
                        count: 0,
                    }
                }
            }));
            g.record = Some(record);
            g.read_only_reason = (!valid_slot(slot))
                .then(|| "This monster has an unsupported creature or quantity.".into());
        }
    }
    Ok(groups)
}

fn compatible_templates(
    p: &[u8],
    object: usize,
    templates: &[(Vec<u8>, [u8; 33])],
) -> Vec<(u32, u16)> {
    let Some(index) = word(p, object + 3) else {
        return Vec::new();
    };
    let Some((_, base)) = templates.get(index as usize) else {
        return Vec::new();
    };
    let mut candidates = std::collections::BTreeMap::<u32, Vec<usize>>::new();
    for (i, (_, tail)) in templates.iter().enumerate() {
        let id = u16::from_le_bytes([tail[28], tail[29]]) as u32;
        if tail[..28] == base[..28]
            && tail[30..] == base[30..]
            && crate::creatures::name(id).is_some()
        {
            candidates.entry(id).or_default().push(i);
        }
    }
    candidates
        .into_iter()
        .filter_map(|(id, items)| {
            let first = items[0];
            items
                .iter()
                .all(|&i| templates[i] == templates[first])
                .then_some((id, first as u16))
        })
        .collect()
}

/// HotA extension after the replay section. A complete count-prefixed chain
/// is required; the native 72 -> 56 pair establishes +0 as current quantity.
fn monster_table(doc: &SaveDocument, groups: &[NeutralArmy]) -> Option<usize> {
    let p = doc.payload();
    let monsters: Vec<_> = groups.iter().filter(|g| g.class == 54).collect();
    let mut ids = std::collections::BTreeSet::new();
    for g in &monsters {
        if !ids.insert(g.record?) {
            return None;
        }
    }
    let minimum = ids.last()?.checked_add(1)?;
    let floor = doc.heroes.last()?.span.end;
    let mut found = None;
    for at in floor..p.len().saturating_sub(4) {
        let count = dword(p, at)? as usize;
        if count < minimum || count > 65535 {
            continue;
        }
        let start = at + 4;
        let Some(end) = start.checked_add(count * 33).filter(|&end| end <= p.len()) else {
            continue;
        };
        let valid = p[start..end].as_chunks::<33>().0.iter().all(|r| {
            let disposition = i32::from_le_bytes(r[5..9].try_into().unwrap_or([0; 4]));
            r[4] <= 15
                && (disposition == -4 || (1..=10).contains(&disposition))
                && r[9] <= 1
                && r[10] <= 1
                && r[15] <= 1
                && r[16..20] == 100u32.to_le_bytes()
                && matches!(
                    i32::from_le_bytes(r[20..24].try_into().unwrap_or([0; 4])),
                    -1..=1
                )
                && matches!(
                    i32::from_le_bytes(r[24..28].try_into().unwrap_or([0; 4])),
                    -1..=7
                )
                && r[28] <= 1
                && r[32] < 128
        });
        if valid {
            if found.is_some() {
                return None;
            }
            found = Some(start);
        }
    }
    found
}

fn valid_slot(s: ArmySlot) -> bool {
    if s.creature_id == u32::MAX {
        s.count == 0
    } else {
        crate::creatures::HOTA_44_11
            .iter()
            .any(|&(id, _)| id == s.creature_id)
            && (1..=i32::MAX as u32).contains(&s.count)
    }
}

fn garrison_table(doc: &SaveDocument, groups: &[NeutralArmy], indexes: &[usize]) -> Option<usize> {
    let p = doc.payload();
    let end = doc.players.first()?.start.checked_sub(49)?;
    let mut candidates = Vec::new();
    for boats in 0..=255 {
        let Some(boat_start) = end.checked_sub(28 * boats + 1) else {
            continue;
        };
        if p.get(boat_start).copied()? as usize != boats
            || !(0..boats).all(|i| p.get(boat_start + 2 + i * 28).copied() == Some(i as u8))
        {
            continue;
        }
        let count = indexes.len();
        let Some(start) = boat_start.checked_sub(count * 61) else {
            continue;
        };
        if start == 0 || p.get(start - 1).copied()? as usize != count {
            continue;
        }
        let mut ids = std::collections::BTreeSet::new();
        if indexes.iter().all(|&i| {
            let g = &groups[i];
            let Some(id) = g.record else {
                return false;
            };
            if id >= count || !ids.insert(id) {
                return false;
            }
            let a = start + 61 * id;
            (p[a] <= 7 || p[a] == 255) && p[a + 60] <= 1 && p[a + 57..a + 60] == g.position
        }) {
            candidates.push(start);
        }
    }
    if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    }
}

impl SaveDocument {
    pub fn neutral_armies(&self) -> NeutralArmies {
        match discover(self) {
            Ok(groups) => NeutralArmies {
                groups,
                unavailable_reason: None,
            },
            Err(reason) => NeutralArmies {
                groups: Vec::new(),
                unavailable_reason: Some(reason),
            },
        }
    }

    pub fn plan_neutral_edits(
        &self,
        id: usize,
        edits: &[NeutralEdit],
    ) -> Result<Vec<BytePatch>, EditError> {
        let refuse = |reason: String| EditError::InvalidValue {
            field: "map army",
            reason,
        };
        let groups = discover(self).map_err(refuse)?;
        let g = groups
            .iter()
            .find(|g| g.id == id)
            .ok_or_else(|| refuse("Current army object was not found.".into()))?;
        if let Some(reason) = &g.read_only_reason {
            return Err(refuse(reason.clone()));
        }
        let start = g
            .record
            .ok_or_else(|| refuse("No verified army record.".into()))?;
        if g.class == 54 {
            if edits.len() != 1
                || edits[0].slot != 0
                || edits[0].creature_id.is_none()
                || edits[0].count == 0
                || edits[0].count > i32::MAX as u32
            {
                return Err(refuse("A wandering group has one positive stack; adding, removing or splitting groups is unsupported.".into()));
            }
            let edit = edits[0];
            let id = edit.creature_id.unwrap_or(u32::MAX);
            let mut patches = Vec::new();
            if id != u32::from(g.subtype) {
                let &(_, template) = g.replacements.iter()
                    .find(|&&(creature, _)| creature == id)
                    .ok_or_else(|| refuse(
                        "No unambiguous existing creature template with the same footprint is available.".into()
                    ))?;
                patches.push(BytePatch::new(
                    g.tile + 10,
                    self.payload()[g.tile + 10..g.tile + 12].to_vec(),
                    (id as u16).to_le_bytes().to_vec(),
                    "Wandering creature type",
                )?);
                patches.push(BytePatch::new(
                    g.object + 3,
                    self.payload()[g.object + 3..g.object + 5].to_vec(),
                    template.to_le_bytes().to_vec(),
                    "Wandering creature sprite reference",
                )?);
            }
            if dword(self.payload(), start) != Some(edit.count) {
                patches.push(BytePatch::new(
                    start,
                    self.payload()[start..start + 4].to_vec(),
                    edit.count.to_le_bytes().to_vec(),
                    "Wandering creature quantity",
                )?);
            }
            return Ok(patches);
        }
        let mut slots = std::collections::BTreeSet::new();
        let mut patches = Vec::new();
        for edit in edits {
            let slot = ArmySlot {
                creature_id: edit.creature_id.unwrap_or(u32::MAX),
                count: edit.count,
            };
            if edit.slot >= 7 || !slots.insert(edit.slot) || !valid_slot(slot) {
                return Err(refuse("Use distinct slots 1-7 and a known creature with a positive count, or an empty slot with count zero.".into()));
            }
            for (base, value) in [(1, slot.creature_id), (29, slot.count)] {
                let offset = start + base + edit.slot as usize * 4;
                let bytes = value.to_le_bytes();
                if self.payload()[offset..offset + 4] != bytes {
                    patches.push(BytePatch::new(
                        offset,
                        self.payload()[offset..offset + 4].to_vec(),
                        bytes.to_vec(),
                        format!("Map garrison {id} slot {}", edit.slot + 1),
                    )?);
                }
            }
        }
        Ok(patches)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_links_validate_tile_coordinates_not_just_record_lengths() {
        let objects = ObjectTable {
            objects: vec![([3, 4, 1], 54, 0, [0; 6])],
            start: 0,
            templates: Vec::new(),
        };
        let mut tile = vec![0; 26];
        tile[24] = 0x21; // Anchor (3,4,1) minus tile (2,2,1).
        let index = 10 * 10 + 2 * 10 + 2;
        assert!(render_matches(&tile, 0, 1, index, 10, &objects));
        assert!(!render_matches(&tile, 0, 1, index + 1, 10, &objects));
        assert!(!render_matches(&tile, 0, 1, index - 100, 10, &objects));
        tile[22] = 1;
        assert!(!render_matches(&tile, 0, 1, index, 10, &objects));
        tile[22] = 0;
        tile[24] = 0;
        assert!(!render_matches(&tile, 0, 1, index, 10, &objects));
        assert!(!render_matches(&tile[..23], 0, 1, index, 10, &objects));
        assert!(render_matches(&tile, 0, 0, index, 10, &objects));
    }

    #[test]
    fn template_and_object_arrays_require_complete_bounded_references() {
        let mut bytes = 1u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&5u16.to_le_bytes());
        bytes.extend_from_slice(b"A.def");
        let tail = bytes.len();
        bytes.resize(tail + 33, 0);
        bytes[tail + 25] = 0x40;
        bytes[tail + 26..tail + 28].copy_from_slice(&54u16.to_le_bytes());
        bytes[tail + 28..tail + 30].copy_from_slice(&13u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&[10, 20, 0, 0, 0]);
        let parsed = object_table(&bytes, 0).unwrap();
        assert_eq!(parsed.objects[0].0, [10, 20, 0]);
        assert_eq!((parsed.objects[0].1, parsed.objects[0].2), (54, 13));
        for len in 0..bytes.len() {
            assert!(object_table(&bytes[..len], 0).is_none());
        }
        let last = bytes.len() - 1;
        bytes[last] = 1;
        assert!(object_table(&bytes, 0).is_none());
        bytes[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(object_table(&bytes, 0).is_none());
    }
}
