//! Typed projections of current GM2 armies and separately labeled H3M facts.
use crate::dto::{ArmySlotDto, CreatureDto};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeutralDto {
    pub id: usize,
    pub kind: &'static str,
    pub position: [u8; 3],
    pub creature_id: Option<u32>,
    pub allowed_creatures: Vec<u32>,
    pub name: Option<&'static str>,
    pub army: Option<Vec<ArmySlotDto>>,
    pub owner: Option<u8>,
    pub read_only_reason: Option<String>,
    pub dirty_slots: Vec<u8>,
    pub initial_army: Option<Vec<InitialStackDto>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialStackDto {
    pub creature_id: Option<u32>,
    pub name: Option<&'static str>,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeutralListDto {
    pub groups: Vec<NeutralDto>,
    pub unavailable_reason: Option<String>,
    pub choices: Vec<CreatureDto>,
    pub initial_groups: Option<usize>,
}
