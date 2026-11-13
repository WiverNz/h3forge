import type { NeutralArmyDto, OwnerDto, PlayerDto } from '../types/api.ts';

export type Direction = 'asc' | 'desc';
export type SortValue = string | number | readonly number[] | null;
export interface ListSort { key: string; direction: Direction }

// Missing values always trail known values. Source ordinal breaks ties even in descending order.
export function sortList<T>(rows: readonly T[], sort: ListSort, locale: string,
  value: (row: T) => SortValue): T[] {
  const collator = new Intl.Collator(locale, { numeric: true, sensitivity: 'base' });
  const sign = sort.direction === 'asc' ? 1 : -1;
  return rows.map((row, ordinal) => ({ row, ordinal, value: value(row) })).sort((a, b) => {
    if (sort.key === 'original') return sign * (a.ordinal - b.ordinal);
    if (a.value === null || b.value === null) return (a.value === null ? 1 : 0) - (b.value === null ? 1 : 0) || a.ordinal - b.ordinal;
    let comparison = 0;
    if (typeof a.value === 'string' && typeof b.value === 'string') comparison = collator.compare(a.value, b.value);
    else if (typeof a.value === 'number' && typeof b.value === 'number') comparison = a.value - b.value;
    else if (Array.isArray(a.value) && Array.isArray(b.value)) {
      for (let i = 0; i < a.value.length; i++) {
        comparison = a.value[i] - b.value[i];
        if (comparison) break;
      }
    }
    return sign * comparison || a.ordinal - b.ordinal;
  }).map(item => item.row);
}

export function matchesFacet(selected: readonly string[], value: string): boolean {
  return selected.length === 0 || selected.includes(value);
}
export function toggleFacet(selected: readonly string[], value: string): string[] {
  return selected.includes(value) ? selected.filter(item => item !== value) : [...selected, value];
}
export function ownerKey(owner: Pick<OwnerDto, 'kind' | 'player'>): string {
  if (owner.kind === 'neutral') return 'neutral';
  return owner.kind === 'player' && owner.player !== null && Number.isInteger(owner.player)
    && owner.player >= 0 && owner.player < 8 ? String(owner.player) : 'unknown';
}
export function ownerValue(key: string): number | null {
  return key === 'neutral' ? 8 : /^[0-7]$/.test(key) ? Number(key) : null;
}
export function creatureOwner(group: NeutralArmyDto): string {
  return group.owner === 255 ? 'neutral' : ownerKey({ kind: 'player', player: group.owner });
}
export function teamKey(player: PlayerDto): string {
  if (!player.details.mapLinked) return 'unknown';
  return player.details.team === null ? 'none' : String(player.details.team);
}
export function creatureName(group: NeutralArmyDto): string | null {
  const slot = group.army?.find(item => !item.empty);
  return slot ? slot.name ?? (slot.creatureId === null ? null : `#${slot.creatureId}`) : null;
}
export function creatureQuantity(group: NeutralArmyDto): number | null {
  return group.army === null ? null : group.army.reduce((sum, slot) => sum + (slot.empty ? 0 : slot.count), 0);
}
export function matchesCreatureType(group: NeutralArmyDto, types: readonly string[]): boolean {
  if (!types.length) return true;
  if (group.army === null) return types.includes('unknown');
  const slots = group.army.filter(slot => !slot.empty);
  return slots.length === 0 ? types.includes('empty')
    : slots.some(slot => types.includes(slot.creatureId === null ? 'unknown' : String(slot.creatureId)));
}
export function selectedRecord<T extends { id: number }>(rows: readonly T[], id: number | null): T | undefined {
  return rows.find(row => row.id === id) ?? rows[0];
}
export const preferenceKey = (list: string) => `h3forge.list.${list}.v1`;
export function readListSort(list: string, keys: readonly string[], defaultKey: string): ListSort {
  try {
    const value = JSON.parse(localStorage.getItem(preferenceKey(list)) ?? 'null') as ListSort | null;
    if (value && keys.includes(value.key) && (value.direction === 'asc' || value.direction === 'desc')) return value;
  } catch { /* Storage is optional. */ }
  return { key: defaultKey, direction: 'asc' };
}
export function writeListSort(list: string, sort: ListSort): void {
  try { localStorage.setItem(preferenceKey(list), JSON.stringify({ key: sort.key, direction: sort.direction })); }
  catch { /* Retain the session preference when storage is unavailable. */ }
}
