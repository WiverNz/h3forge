//! HotA 44/11 team table (TeamsInfo). See SAVEGAME_FORMAT.md H3F-030.
//!
//! The table is found by a bounded structural walk of the embedded map
//! header, never by an absolute offset, and the walk must end on the fixed
//! starting-settings anchors. Only one operation is supported: moving a
//! participating player into another team that already exists, while every
//! team keeps at least one participant. That is the operation verified by
//! loading edited saves in HotA. The team count never changes.
use crate::{BytePatch, EditError, SaveDocument};

/// The map name length prefix; everything before it is fixed in 44/11.
const NAME_AT: usize = 95;
/// The whole walked header must stay inside this window.
const WINDOW: usize = 64 * 1024;
const MAX_HERO_NAME: usize = 255;

/// Current team assignments of a save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamTable {
    /// Number of teams; zero means the save stores no assignment bytes.
    pub count: u8,
    /// Zero-based team per player color. Meaningless for absent colors.
    pub teams: [u8; 8],
    /// Colors that take part in this game according to the starting settings.
    pub participating: [bool; 8],
    /// Payload offset of the eight assignment bytes; `None` without teams.
    assignments_at: Option<usize>,
}

impl TeamTable {
    /// Payload offset of the eight assignment bytes, for diagnostics and tests.
    pub fn assignments_offset(&self) -> Option<usize> {
        self.assignments_at
    }

    /// Whether assignments can be changed at all.
    pub fn editable(&self) -> bool {
        self.assignments_at.is_some() && self.count >= 2
    }
}

/// Move one player color into an existing zero-based team.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamEdit {
    pub color: u8,
    pub team: u8,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, len: usize) -> Option<&[u8]> {
        let end = self.at.checked_add(len)?;
        let out = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(out)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn flag(&mut self) -> Option<bool> {
        match self.u8()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    fn skip_text16(&mut self) -> Option<()> {
        let len = self.take(2)?;
        let len = usize::from(u16::from_le_bytes([len[0], len[1]]));
        self.take(len).map(|_| ())
    }
}

fn locate(payload: &[u8]) -> Option<TeamTable> {
    if !crate::detect::detect(payload).ok()?.army_edit_supported() || payload.get(..5)? != b"H3SVG"
    {
        return None;
    }
    let fixed = payload.get(89..NAME_AT)?;
    let size = u32::from_le_bytes([fixed[1], fixed[2], fixed[3], fixed[4]]);
    if fixed[0] > 1 || fixed[5] > 1 || !(1..=252).contains(&size) {
        return None;
    }
    let mut r = Reader {
        bytes: &payload[..payload.len().min(WINDOW)],
        at: NAME_AT,
    };
    r.skip_text16()?; // map name
    r.skip_text16()?; // description
    r.take(2)?; // difficulty, maximum hero level
    let mut eligible = [false; 8];
    for slot in &mut eligible {
        let human = r.flag()?;
        let computer = r.flag()?;
        r.take(4)?; // behavior, allowed factions, random faction
        if r.flag()? {
            r.take(3)?; // main town coordinates
        }
        r.take(1)?;
        if r.u8()? != 0xff {
            r.take(1)?; // portrait
            r.skip_text16()?;
        }
        *slot = human || computer;
    }
    let victory = r.u8()?;
    if victory != 0xff {
        r.take(2)?; // allow normal victory, applies to computer
        r.take(match victory {
            8 | 9 | 11 => 0,
            0 | 5 => 1,
            4 | 6 | 7 => 3,
            10 | 12 => 4,
            1..=3 => 5,
            _ => return None,
        })?;
    }
    match r.u8()? {
        0xff => {}
        0 => drop(r.take(3)?),
        1 | 2 => drop(r.take(2)?),
        _ => return None,
    }
    let count = r.u8()?;
    if count > 8 {
        return None;
    }
    let mut teams = [0u8; 8];
    let mut assignments_at = None;
    if count > 0 {
        assignments_at = Some(r.at);
        teams.copy_from_slice(r.take(8)?);
    }
    for _ in 0..r.u8()? {
        r.take(2)?; // hero type, portrait
        let len = r.take(4)?;
        let len = u32::from_le_bytes([len[0], len[1], len[2], len[3]]) as usize;
        if len > MAX_HERO_NAME {
            return None;
        }
        r.take(len + 1)?; // name, hire mask
    }
    // Anchors proving the walk stayed aligned: the color identity row, the
    // starting factions and the original map file name.
    if r.take(8)? != [0, 1, 2, 3, 4, 5, 6, 7] {
        return None;
    }
    r.take(8)?;
    let mut participating = [false; 8];
    let factions = r.take(32)?.to_vec();
    let types = r.take(8)?.to_vec();
    for color in 0..8 {
        let at = color * 4;
        let faction = i32::from_le_bytes([
            factions[at],
            factions[at + 1],
            factions[at + 2],
            factions[at + 3],
        ]);
        if !(-1..=31).contains(&faction) {
            return None;
        }
        participating[color] = faction >= 0 && types[color] != 0xff && eligible[color];
    }
    r.take(1)?; // chosen difficulty
    let file = r.take(251)?;
    let name = &file[..file.iter().position(|&b| b == 0)?];
    if name.len() < 5
        || !name[name.len() - 4..].eq_ignore_ascii_case(b".h3m")
        || name.iter().any(|&b| b < 0x20)
    {
        return None;
    }
    if (0..8).any(|c| participating[c] && count > 0 && teams[c] >= count) {
        return None;
    }
    Some(TeamTable {
        count,
        teams,
        participating,
        assignments_at,
    })
}

fn invalid(reason: &str) -> EditError {
    EditError::InvalidValue {
        field: "player team",
        reason: reason.into(),
    }
}

impl SaveDocument {
    /// Current team table, or `None` when the header layout is not the
    /// verified HotA 44/11 one. Always re-derived from the immutable payload.
    pub fn team_table(&self) -> Option<TeamTable> {
        locate(self.payload())
    }

    /// Plan all team moves together: one checked patch per changed byte.
    /// No count change, renumbering, fog, visit or victory-state updates.
    pub fn plan_team_edits(&self, edits: &[TeamEdit]) -> Result<Vec<BytePatch>, EditError> {
        let table = self.team_table().ok_or_else(|| {
            invalid("Team layout is unverified; requires a regular HotA 44/11 save.")
        })?;
        let Some(at) = table.assignments_at.filter(|_| table.editable()) else {
            return Err(invalid(
                "This save has fewer than two teams; creating teams is not supported.",
            ));
        };
        let mut teams = table.teams;
        let mut seen = [false; 8];
        for edit in edits {
            let color = usize::from(edit.color);
            if color > 7 || !table.participating[color] {
                return Err(invalid("Player does not take part in this game."));
            }
            if edit.team >= table.count {
                return Err(invalid(
                    "Team does not exist; creating teams is not supported.",
                ));
            }
            if seen[color] {
                return Err(invalid("Duplicate team operation for one player."));
            }
            seen[color] = true;
            teams[color] = edit.team;
        }
        if (0..table.count).any(|team| !(0..8).any(|c| table.participating[c] && teams[c] == team))
        {
            return Err(invalid(
                "Every team must keep at least one player; removing teams is not supported.",
            ));
        }
        (0..8)
            .filter(|&c| teams[c] != table.teams[c])
            .map(|c| {
                BytePatch::new(
                    at + c,
                    vec![table.teams[c]],
                    vec![teams[c]],
                    format!("Player {c}: team"),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal synthetic header: two teams, Red/Blue versus Tan.
    fn header(victory: &[u8], teams: &[u8]) -> Vec<u8> {
        let mut p = b"H3SVG\0\0\0".to_vec();
        p.extend([44, 0, 0, 0, 11, 0, 0, 0]);
        p.resize(89, 0);
        p.extend([1, 36, 0, 0, 0, 0]);
        p.extend([3, 0, b'M', b'a', b'p', 0, 0, 1, 0]);
        for color in 0..8u8 {
            let on = u8::from(color < 3);
            p.extend([on, on, 0, 0xff, 0x0f, 1]);
            if color == 0 {
                p.extend([1, 5, 6, 0, 0, 7, 0xff, 2, 0, b'A', b'b']);
            } else {
                p.extend([0, 0, 0xff]);
            }
        }
        p.extend(victory);
        p.push(0xff);
        p.extend(teams);
        p.extend([1, 9, 0xff, 2, 0, 0, 0, b'X', b'y', 0xff]);
        p.extend(0..8u8);
        p.extend([0; 8]);
        for color in 0..8 {
            p.extend(if color < 3 { 1i32 } else { -1 }.to_le_bytes());
        }
        p.extend([0, 10, 10, 0xff, 0xff, 0xff, 0xff, 0xff, 1]);
        let mut file = b"Map.h3m".to_vec();
        file.resize(251, 0);
        p.extend(file);
        p.resize(p.len() + 4096, 0);
        p
    }

    #[test]
    fn walks_variable_header_and_rejects_misaligned_layouts() {
        let teams = [2, 0, 0, 1, 0, 0, 0, 0, 0];
        for victory in [&[0xff][..], &[7, 1, 1, 3, 4, 0], &[1, 0, 0, 5, 1, 0, 0, 0]] {
            let table = locate(&header(victory, &teams)).unwrap();
            assert_eq!((table.count, table.teams), (2, [0, 0, 1, 0, 0, 0, 0, 0]));
            assert_eq!(table.participating[..4], [true, true, true, false]);
            assert!(table.editable());
        }
        let none = locate(&header(&[0xff], &[0])).unwrap();
        assert!(!none.editable() && none.assignments_offset().is_none());
        assert!(locate(&header(&[99, 0, 0], &teams)).is_none());
        assert!(locate(&header(&[0xff], &[2, 0, 0, 5, 0, 0, 0, 0, 0])).is_none());
        let good = header(&[0xff], &teams);
        assert!(locate(&good[..good.len() - 4200]).is_none());
        let mut other = good;
        other[12] = 10;
        assert!(locate(&other).is_none());
    }
}
