//! HotA 44/11 player state. See SAVEGAME_FORMAT.md section E for evidence.
//! No guessed absolute save offsets: the eight records must precede a
//! complete, verified town table and cross-reference its ownership lists.
use crate::{BytePatch, EditError, HeroConfidence, HeroView, TownConfidence, TownView};

pub const RECORD_LEN: usize = 149;
pub const RESOURCES: usize = 98;
pub const RESOURCE_NAMES: [&str; 7] = [
    "Wood", "Mercury", "Ore", "Sulfur", "Crystal", "Gems", "Gold",
];
pub const MAX_RESOURCE: i64 = i32::MAX as i64;

/// Current participation, independent of resource edit capability.
/// Defeated/Unused are reserved for verified historical evidence; the
/// researched 44/11 record cannot establish either from empty asset lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatus {
    Active,
    Defeated,
    Unused,
    Unknown,
}

impl PlayerStatus {
    pub fn code(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Defeated => "defeated",
            Self::Unused => "unused",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerView {
    /// Record index, distinct from the stored owner color.
    pub id: usize,
    pub color: u8,
    pub hero_count: u8,
    pub town_count: u8,
    pub resources: [i32; 7],
    pub active: bool,
    pub status: PlayerStatus,
    pub status_reason: &'static str,
    pub read_only_reason: Option<String>,
    pub(crate) start: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerEdit {
    /// Resource ID in binary order 0-6. Signed wide input rejects negative
    /// values and overflow explicitly before creating any four-byte patch.
    SetResource { resource: u8, value: i64 },
}

pub(crate) fn parse(payload: &[u8], towns: &[TownView], heroes: &[HeroView]) -> Vec<PlayerView> {
    let Some(first) = towns.first() else {
        return Vec::new();
    };
    let Some(end) = first.span.start.checked_sub(2) else {
        return Vec::new();
    };
    let Some(start) = end.checked_sub(8 * RECORD_LEN) else {
        return Vec::new();
    };
    let Some(table) = payload.get(start..end) else {
        return Vec::new();
    };
    if payload.get(end).copied().map(usize::from) != Some(towns.len())
        || towns.len() > 72
        || towns.iter().any(|t| t.confidence != TownConfidence::High)
        || towns.windows(2).any(|w| w[0].span.end != w[1].span.start)
        || (0..8).any(|i| table[i * RECORD_LEN] != i as u8)
    {
        return Vec::new();
    }

    let mut result = Vec::new();
    for id in 0..8 {
        let r = &table[id * RECORD_LEN..(id + 1) * RECORD_LEN];
        let color = r[0];
        let hero_count = r[1];
        let town_count = r[24];
        let active = hero_count > 0 || town_count > 0;
        let mut valid = hero_count <= 8 && town_count <= 72 && r[142] <= 1;
        let mut listed_heroes: Vec<u8> = r[3..11].iter().copied().filter(|&h| h != 255).collect();
        listed_heroes.sort_unstable();
        valid &= listed_heroes.len() == hero_count as usize
            && !listed_heroes.windows(2).any(|w| w[0] == w[1])
            && (r[2] == 255 || listed_heroes.contains(&r[2]));
        // Only roster references are used here; owner bytes alone are not
        // evidence that every owned hero is an active adventuring hero.
        for hero_type in listed_heroes {
            valid &= heroes
                .iter()
                .filter(|h| {
                    h.confidence == HeroConfidence::High
                        && h.owner.byte() == color
                        && payload.get(h.span.start + 10) == Some(&hero_type)
                })
                .count()
                == 1;
        }
        if town_count <= 72 {
            let mut listed = r[26..26 + town_count as usize].to_vec();
            listed.sort_unstable();
            let mut owned: Vec<u8> = towns
                .iter()
                .filter(|t| t.owner.byte() == color)
                .filter_map(|t| {
                    t.span
                        .start
                        .checked_sub(1)
                        .and_then(|i| payload.get(i))
                        .copied()
                })
                .collect();
            owned.sort_unstable();
            valid &= listed == owned && !listed.windows(2).any(|w| w[0] == w[1]);
        }
        let resources = std::array::from_fn(|i| {
            let at = RESOURCES + i * 4;
            i32::from_le_bytes([r[at], r[at + 1], r[at + 2], r[at + 3]])
        });
        let read_only_reason = if !valid {
            Some("Player structure or hero/town ownership links are ambiguous.".into())
        } else if resources.iter().any(|&v| v < 0) {
            Some("Negative stored resources have no verified edit semantics.".into())
        } else {
            None
        };
        result.push(PlayerView {
            id,
            color,
            hero_count,
            town_count,
            resources,
            active: valid && active,
            status: if valid && active {
                PlayerStatus::Active
            } else {
                PlayerStatus::Unknown
            },
            status_reason: if !valid {
                "Player structure or ownership links are ambiguous."
            } else if active {
                "Current GM2 hero or town ownership is verified."
            } else {
                "No listed heroes or towns; defeat versus never participating is unverified."
            },
            read_only_reason,
            start: start + id * RECORD_LEN,
        });
    }
    result
}

impl crate::SaveDocument {
    /// Read-only HotA 44/11 obelisk masks immediately before Player[8].
    /// The h3svg table is count:u8 + 48 visit masks; unused masks must be
    /// zero for this verified layout. Callers must cross-check the count
    /// against the linked map. Slot order is NOT a proven H3M object ID map.
    pub fn obelisk_visit_masks(&self) -> Option<Vec<u8>> {
        if !self.format.army_edit_supported() || self.players.len() != 8 {
            return None;
        }
        let end = self.players.first()?.start;
        let start = end.checked_sub(49)?;
        let table = self.payload().get(start..end)?;
        let count = usize::from(table[0]);
        if count > 48 || table[1 + count..].iter().any(|&byte| byte != 0) {
            return None;
        }
        Some(table[1..1 + count].to_vec())
    }

    pub fn plan_player_edits(
        &self,
        id: usize,
        edits: &[PlayerEdit],
    ) -> Result<Vec<BytePatch>, EditError> {
        if !self.format.army_edit_supported() {
            return Err(EditError::UnsupportedFormat {
                family: self.format.family_title(),
                major: self.format.major,
                minor: self.format.minor,
            });
        }
        // Reparse from immutable bytes so even a caller-mutated public view
        // cannot grant itself a writable resource offset or capability.
        let players = parse(self.payload(), &self.towns, &self.heroes);
        let player = players
            .get(id)
            .ok_or_else(|| invalid("Player table is unsupported or the record does not exist."))?;
        if let Some(reason) = &player.read_only_reason {
            return Err(invalid(reason));
        }
        let mut seen = [false; 7];
        let mut patches = Vec::new();
        for edit in edits {
            let PlayerEdit::SetResource { resource, value } = *edit;
            if resource > 6 || !(0..=MAX_RESOURCE).contains(&value) {
                return Err(invalid(
                    "Resource must be 0-6 and value must be 0-2147483647.",
                ));
            }
            let index = resource as usize;
            let offset = player.start + RESOURCES + index * 4;
            if seen[index] {
                return Err(EditError::OverlappingPatches { offset });
            }
            seen[index] = true;
            if player.resources[index] as i64 != value {
                patches.push(BytePatch::new(
                    offset,
                    self.payload()[offset..offset + 4].to_vec(),
                    (value as i32).to_le_bytes().to_vec(),
                    format!("Player {id}: {}", RESOURCE_NAMES[index]),
                )?);
            }
        }
        Ok(patches)
    }
}

fn invalid(reason: impl Into<String>) -> EditError {
    EditError::InvalidValue {
        field: "player resource",
        reason: reason.into(),
    }
}
