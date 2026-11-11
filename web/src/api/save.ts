import { dialogLabels } from '../i18n/dialogs';
import { invoke, isTauri } from '@tauri-apps/api/core';
import type {
  AppInfo,
  HeroDetailDto,
  HeroEditRequest,
  HeroEditResponseDto,
  HeroListItemDto,
  MapLinkDto,
  PendingEditDto,
  SaveGrantDto,
  SaveSummaryDto,
  TownDetailDto,
  TownEditRequest,
  TownEditResponseDto,
  TownListItemDto,
} from '../types/api';

function requireTauri(): void {
  if (!isTauri()) {
    throw new Error('Open the H3Forge desktop application to connect. This is a browser preview.');
  }
}

export async function getAppInfo(): Promise<AppInfo> {
  requireTauri();
  return invoke<AppInfo>('get_app_info');
}

/** Runs the native picker; resolves to null when the user cancels. */
export async function chooseSave(): Promise<SaveGrantDto | null> {
  requireTauri();
  return invoke<SaveGrantDto | null>('choose_save', dialogLabels('open'));
}

export async function openGrantedSave(grantId: string): Promise<SaveSummaryDto> {
  requireTauri();
  return invoke<SaveSummaryDto>('open_granted_save', { grantId });
}

export async function listHeroes(): Promise<HeroListItemDto[]> {
  requireTauri();
  return invoke<HeroListItemDto[]>('list_heroes');
}

export async function getHero(id: number): Promise<HeroDetailDto> {
  requireTauri();
  return invoke<HeroDetailDto>('get_hero', { id });
}

/** Applies one batch of semantic edits; returns the hero's effective editable state. */
export async function applyHeroEdit(
  id: number,
  request: HeroEditRequest,
): Promise<HeroEditResponseDto> {
  requireTauri();
  return invoke<HeroEditResponseDto>('apply_hero_edit', { id, request });
}

/** Effective editable state for one hero (base values plus pending edits). */
export async function getHeroEditState(id: number): Promise<HeroEditResponseDto> {
  requireTauri();
  return invoke<HeroEditResponseDto>('get_hero_edit_state', { id });
}

/** Marks or unmarks one hero for dismissal; nothing is written until Save. */
export async function setHeroDismissal(id: number, dismissed: boolean): Promise<HeroEditResponseDto> {
  requireTauri();
  return invoke<HeroEditResponseDto>('set_hero_dismissal', { id, dismissed });
}

export async function getPendingEdits(): Promise<PendingEditDto[]> {
  requireTauri();
  return invoke<PendingEditDto[]>('get_pending_edits');
}

export async function discardHeroEdits(id: number): Promise<PendingEditDto[]> {
  requireTauri();
  return invoke<PendingEditDto[]>('discard_hero_edits', { id });
}

export async function discardAllEdits(): Promise<PendingEditDto[]> {
  requireTauri();
  return invoke<PendingEditDto[]>('discard_all_edits');
}

/**
 * Native Save As picker plus the safe write. Resolves to null when the user
 * cancels; on success the returned summary describes the written file.
 */
export async function saveSaveAs(): Promise<SaveSummaryDto | null> {
  requireTauri();
  return invoke<SaveSummaryDto | null>('save_save_as', dialogLabels('save'));
}

/** Safely replaces the currently open file, with a verified backup. */
export async function save(): Promise<SaveSummaryDto> {
  requireTauri();
  return invoke<SaveSummaryDto>('save');
}

/** Link between the open save and its source map (.h3m). */
export async function getMapLink(): Promise<MapLinkDto> {
  requireTauri();
  return invoke<MapLinkDto>('get_map_link');
}

/** Re-runs bounded map discovery for the open save. */
export async function rescanMaps(): Promise<MapLinkDto> {
  requireTauri();
  return invoke<MapLinkDto>('rescan_maps');
}

/** Links one candidate of an ambiguous discovery result. */
export async function selectMapCandidate(index: number): Promise<MapLinkDto> {
  requireTauri();
  return invoke<MapLinkDto>('select_map_candidate', { index });
}

/** Native map picker; resolves to null when the user cancels. */
export async function chooseMapFile(): Promise<MapLinkDto | null> {
  requireTauri();
  return invoke<MapLinkDto | null>('choose_map_file', dialogLabels('map'));
}

// ------------------------------------------------------ towns (H3F-013) --

export async function listTowns(): Promise<TownListItemDto[]> {
  requireTauri();
  return invoke<TownListItemDto[]>('list_towns');
}

export async function getTown(id: number): Promise<TownDetailDto> {
  requireTauri();
  return invoke<TownDetailDto>('get_town', { id });
}

/** Applies one batch of semantic garrison edits; returns the town's effective editable state. */
export async function applyTownEdit(
  id: number,
  request: TownEditRequest,
): Promise<TownEditResponseDto> {
  requireTauri();
  return invoke<TownEditResponseDto>('apply_town_edit', { id, request });
}

/** Effective editable state for one town (base values plus pending edits). */
export async function getTownEditState(id: number): Promise<TownEditResponseDto> {
  requireTauri();
  return invoke<TownEditResponseDto>('get_town_edit_state', { id });
}

export async function discardTownEdits(id: number): Promise<PendingEditDto[]> {
  requireTauri();
  return invoke<PendingEditDto[]>('discard_town_edits', { id });
}

export async function listPlayers(): Promise<import('../types/api').PlayerDto[]> {
  requireTauri();
  return invoke('list_players');
}
export async function applyPlayerEdit(id: number, resource: number, value: number): Promise<import('../types/api').PlayerDto[]> {
  requireTauri();
  return invoke('apply_player_edit', { id, resource, value });
}
export async function discardPlayerEdits(id: number): Promise<import('../types/api').PlayerDto[]> {
  requireTauri();
  return invoke('discard_player_edits', { id });
}
export async function listNeutralArmies(): Promise<import('../types/api').NeutralListDto> {
  requireTauri();
  return invoke('list_neutral_armies');
}
export async function applyNeutralEdit(id: number, army: {slot: number; creatureId: number | null; count: number}): Promise<import('../types/api').NeutralListDto> {
  requireTauri();
  return invoke('apply_neutral_edit', { id, army });
}
export async function discardNeutralEdits(id: number): Promise<import('../types/api').NeutralListDto> {
  requireTauri();
  return invoke('discard_neutral_edits', { id });
}
