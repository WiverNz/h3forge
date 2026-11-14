//! Read-only player enrichment. H3M facts never grant GM2 write capabilities.
use crate::{dto::TextDto, map_link::MapLink};
use h3forge_save::SaveDocument;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDetailsDto {
    pub map_linked: bool,
    pub eligibility: Option<&'static str>,
    pub controller: Option<&'static str>,
    pub controller_reason: &'static str,
    /// One-based presentation number, never a GM2 team ID.
    pub team: Option<u8>,
    pub allies: Option<Vec<u8>>,
    pub enemies: Option<Vec<u8>>,
    pub team_source: &'static str,
    pub starting_town: Option<StartingTownDto>,
    pub starting_hero: Option<StartingHeroDto>,
    pub starting_hero_reason: String,
    pub map_size: Option<u32>,
    pub levels: Option<u8>,
    pub obelisks: Option<Vec<ObeliskDto>>,
    pub grail_placements: Option<usize>,
    pub puzzle_progress: Option<usize>,
    pub visited_obelisks: Option<usize>,
    pub reveal_map_reason: &'static str,
    pub reveal_puzzle_reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartingTownDto {
    pub name: Option<TextDto>,
    pub faction: Option<u8>,
    pub faction_name: &'static str,
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartingHeroDto {
    pub hero_type: u8,
    pub name: Option<TextDto>,
    pub portrait_id: Option<u8>,
    pub portrait_source: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObeliskDto {
    pub map_object_id: usize,
    /// Stored H3M sprite anchor, not an inferred actionable tile.
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

fn text(bytes: &[u8]) -> TextDto {
    TextDto::new(bytes, &h3forge_save::decode_legacy_text(bytes))
}

pub fn describe(document: &SaveDocument, link: Option<&MapLink>, color: u8) -> PlayerDetailsDto {
    let linked = link.and_then(|l| l.linked.as_ref());
    let mut out = PlayerDetailsDto {
        map_linked: linked.is_some(), eligibility: None, controller: None,
        controller_reason: "Current GM2 controller fields are not verified. Map eligibility does not identify the current controller.",
        team: None, allies: None, enemies: None,
        team_source: "No linked H3M. The current save team is reported separately.",
        starting_town: None, starting_hero: None,
        starting_hero_reason: "No linked H3M starting-hero information.".into(),
        map_size: document.map.size,
        levels: document.map.has_underground.map(|u| if u { 2 } else { 1 }),
        obelisks: None, grail_placements: None, puzzle_progress: None, visited_obelisks: None,
        reveal_map_reason: "Disabled: HotA 44/11 visibility boundaries, tile order and team-sharing rules are not verified. No visibility bytes will be changed.",
        reveal_puzzle_reason: "Disabled: GM2 obelisk-to-object IDs, puzzle counters, team update rules and current Grail availability are not verified. No puzzle bytes will be changed.".into(),
    };
    let Some(linked) = linked else { return out };
    let map = &linked.map;
    let Some(player) = map.header.players.get(color as usize) else {
        return out;
    };
    out.eligibility = Some(match (player.can_human_play, player.can_computer_play) {
        (true, true) => "Human or Computer",
        (true, false) => "Human",
        (false, true) => "Computer",
        (false, false) => "Not playable",
    });
    out.team = player.team.map(|t| t + 1);
    out.team_source = "Authored H3M teams; the current save team is reported separately.";
    if player.is_playable() {
        let mut allies = Vec::new();
        let mut enemies = Vec::new();
        for (other_color, other) in map.header.players.iter().enumerate() {
            if other_color == color as usize || !other.is_playable() {
                continue;
            }
            if player.team.is_some() && player.team == other.team {
                allies.push(other_color as u8);
            } else {
                enemies.push(other_color as u8);
            }
        }
        out.allies = Some(allies);
        out.enemies = Some(enemies);
    }
    if let Some(main) = &player.main_hero {
        let saved: Vec<_> = document
            .heroes
            .iter()
            .filter_map(|h| h.portrait)
            .filter(|p| p.hero_type == main.hero_type)
            .collect();
        let (portrait_id, portrait_source) = if let Some(id) = main.portrait {
            (Some(id), "H3M starting portrait")
        } else if saved.len() == 1 {
            (
                Some(saved[0].portrait_id),
                "Current GM2 portrait of the H3M starting hero",
            )
        } else {
            (None, "Default starting portrait is not identified")
        };
        out.starting_hero = Some(StartingHeroDto {
            hero_type: main.hero_type,
            name: (!main.name_raw.is_empty()).then(|| text(&main.name_raw)),
            portrait_id,
            portrait_source,
        });
        out.starting_hero_reason =
            "H3M-defined starting hero; not the player's current selected hero.".into();
    } else {
        out.starting_hero_reason = if player.has_random_hero || player.generate_hero_at_main_town {
            "Random/generated starting hero in H3M; the rolled GM2 identity is unverified."
        } else {
            "No explicit starting hero in H3M."
        }
        .into();
    }
    if let Ok(body) = &map.body {
        if let Some(position) = player.main_town {
            let towns: Vec<_> = body
                .towns
                .iter()
                .filter(|t| t.owner == Some(color) && t.entrance() == position)
                .collect();
            if towns.len() == 1 {
                let town = towns[0];
                let faction = town.faction.and_then(|f| u8::try_from(f).ok());
                out.starting_town = Some(StartingTownDto {
                    name: town.name_raw.as_deref().map(text),
                    faction,
                    faction_name: faction
                        .and_then(h3forge_save::definitions::towns::town_type_name)
                        .unwrap_or(if town.faction.is_none() {
                            "Random faction (H3M)"
                        } else {
                            "Unknown faction"
                        }),
                    x: position.x,
                    y: position.y,
                    z: position.z,
                });
            }
        }
        out.obelisks = Some(
            body.obelisks
                .iter()
                .map(|&(id, p)| ObeliskDto {
                    map_object_id: id,
                    x: p.x,
                    y: p.y,
                    z: p.z,
                })
                .collect(),
        );
        out.grail_placements = Some(body.grails.len());
        if color < 8
            && let Some(masks) = document.obelisk_visit_masks()
            && masks.len() == body.obelisks.len()
        {
            out.visited_obelisks = Some(
                masks
                    .iter()
                    .filter(|&&mask| mask & (1 << color) != 0)
                    .count(),
            );
        }
        if body.obelisks.is_empty() {
            out.reveal_puzzle_reason = "Disabled: the linked H3M contains no obelisks. Current GM2 puzzle availability is unverified.".into();
        } else if body.grails.is_empty() {
            out.reveal_puzzle_reason = "Disabled: no explicit Grail placement in H3M; a generated Grail and current GM2 puzzle state are unverified.".into();
        }
    }
    out
}
