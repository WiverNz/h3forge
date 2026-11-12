import { dialogLabels } from '../i18n/dialogs';
import { invoke, isTauri } from '@tauri-apps/api/core';
import type { InstallationDto, PortraitDto } from '../types/api';

function requireTauri(): void {
  if (!isTauri()) {
    throw new Error('Open the H3Forge desktop application to connect. This is a browser preview.');
  }
}

export async function getInstallationStatus(): Promise<InstallationDto> {
  requireTauri();
  return invoke<InstallationDto>('get_installation_status');
}

/** Runs the native folder picker; resolves to null when the user cancels. */
export async function chooseInstallation(): Promise<InstallationDto | null> {
  requireTauri();
  const result = await invoke<InstallationDto | null>('choose_installation', dialogLabels('folder'));
  if (result) {
    skillIconCache.clear();
    skillIconVersion += 1;
    for (const listener of skillIconListeners) listener();
    creaturePortraitCache.clear();
    creaturePortraitVersion += 1;
    for (const listener of creaturePortraitListeners) listener();
    artifactIconCache.clear();
    artifactIconVersion += 1;
    for (const listener of artifactIconListeners) listener();
    townIconCache.clear();
    buildingIconCache.clear();
    townIconVersion += 1;
    for (const listener of townIconListeners) listener();
  }
  return result;
}

const buildingIconCache = new Map<string, Promise<PortraitDto>>();

/**
 * Town Hall picture of one building. Shares the town icon version, so a new
 * installation refreshes both.
 */
export async function getBuildingIcon(townType: number, building: number): Promise<PortraitDto> {
  requireTauri();
  const key = `${townType}:${building}`;
  let request = buildingIconCache.get(key);
  if (!request) {
    request = invoke<PortraitDto>('get_building_icon', { townType, building });
    buildingIconCache.set(key, request);
    void request.catch(() => {
      if (buildingIconCache.get(key) === request) buildingIconCache.delete(key);
    });
  }
  return request;
}

const townIconCache = new Map<string, Promise<PortraitDto>>();
const townIconListeners = new Set<() => void>();
let townIconVersion = 0;
export const getTownIconVersion = () => townIconVersion;
export function subscribeTownIcons(listener: () => void): () => void {
  townIconListeners.add(listener);
  return () => { townIconListeners.delete(listener); };
}

/** Town type (0-11) to its fort or village icon; `large` picks 58x64 over 48x32. */
export async function getTownIcon(townType: number, fort: boolean, large: boolean): Promise<PortraitDto> {
  requireTauri();
  const key = `${townType}:${fort}:${large}`;
  let request = townIconCache.get(key);
  if (!request) {
    request = invoke<PortraitDto>('get_town_icon', { townType, fort, large });
    townIconCache.set(key, request);
    // Retry rejected IPC calls on remount instead of retaining a failed promise.
    void request.catch(() => {
      if (townIconCache.get(key) === request) townIconCache.delete(key);
    });
  }
  return request;
}

export async function getHeroPortrait(id: number, size: 'large' | 'small'): Promise<PortraitDto> {
  requireTauri();
  return invoke<PortraitDto>('get_hero_portrait', { id, size });
}

export async function getHeroPortraits(
  ids: number[],
  size: 'large' | 'small',
): Promise<PortraitDto[]> {
  requireTauri();
  return invoke<PortraitDto[]>('get_hero_portraits', { ids, size });
}

const creaturePortraitCache = new Map<number, Promise<PortraitDto>>();
const creaturePortraitListeners = new Set<() => void>();
let creaturePortraitVersion = 0;
export const getCreaturePortraitVersion = () => creaturePortraitVersion;
export function subscribeCreaturePortraits(listener: () => void): () => void {
  creaturePortraitListeners.add(listener);
  return () => { creaturePortraitListeners.delete(listener); };
}

/** Share in-flight and completed requests, including unavailable-art responses. */
export async function getCreaturePortrait(id: number): Promise<PortraitDto> {
  requireTauri();
  let request = creaturePortraitCache.get(id);
  if (!request) {
    request = invoke<PortraitDto>('get_creature_portrait', { id });
    creaturePortraitCache.set(id, request);
  }
  return request;
}

const spellIconCache = new Map<number, Promise<PortraitDto>>();
const spellIconListeners = new Set<() => void>();
let spellIconVersion = 0;
export const getSpellIconVersion = () => spellIconVersion;
export function subscribeSpellIcons(listener: () => void): () => void {
  spellIconListeners.add(listener);
  return () => { spellIconListeners.delete(listener); };
}

/** Game spell ID (0-69) to spellbook icon; failures stay cached as placeholders. */
export async function getSpellIcon(id: number): Promise<PortraitDto> {
  requireTauri();
  let request = spellIconCache.get(id);
  if (!request) {
    request = invoke<PortraitDto>('get_spell_icon', { id });
    spellIconCache.set(id, request);
  }
  return request;
}

const skillIconCache = new Map<string, Promise<PortraitDto>>();
const skillIconListeners = new Set<() => void>();
let skillIconVersion = 0;
export const getSkillIconVersion = () => skillIconVersion;
export function subscribeSkillIcons(listener: () => void): () => void {
  skillIconListeners.add(listener);
  return () => { skillIconListeners.delete(listener); };
}

/** Base skill ID (0-27) to secondary-skill icon; failures stay cached as placeholders. */
export async function getSkillIcon(id: number, mastery: number): Promise<PortraitDto> {
  requireTauri();
  const key = `${id}:${mastery}`;
  let request = skillIconCache.get(key);
  if (!request) {
    request = invoke<PortraitDto>('get_skill_icon', { id, mastery });
    skillIconCache.set(key, request);
    // Retry rejected IPC calls on remount instead of retaining a failed promise.
    void request.catch(() => {
      if (skillIconCache.get(key) === request) skillIconCache.delete(key);
    });
  }
  return request;
}

const artifactIconCache = new Map<number, Promise<PortraitDto>>();
const artifactIconListeners = new Set<() => void>();
let artifactIconVersion = 0;
export const getArtifactIconVersion = () => artifactIconVersion;
export function subscribeArtifactIcons(listener: () => void): () => void {
  artifactIconListeners.add(listener);
  return () => { artifactIconListeners.delete(listener); };
}

/** Verified artifact ID to 44x44 base DEF or HotA D32F artwork. */
export async function getArtifactIcon(id: number): Promise<PortraitDto> {
  requireTauri();
  let request = artifactIconCache.get(id);
  if (!request) {
    request = invoke<PortraitDto>('get_artifact_icon', { id });
    artifactIconCache.set(id, request);
    void request.catch(() => {
      if (artifactIconCache.get(id) === request) artifactIconCache.delete(id);
    });
  }
  return request;
}

export async function getResourceIcon(id: number): Promise<PortraitDto> {
  requireTauri();
  return invoke('get_resource_icon', { id });
}

export async function getPlayerFlag(color: number): Promise<PortraitDto> {
  requireTauri();
  return invoke('get_player_flag', { color });
}
export async function getPlayerStartingPortrait(id: number): Promise<PortraitDto> {
  requireTauri();
  return invoke('get_player_starting_portrait', { id });
}

export async function getKeymasterIcon(color: number): Promise<import("../types/api").PortraitDto> { return invoke("get_keymaster_icon", { color }); }
