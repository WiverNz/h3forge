/**
 * Quantity parsing for army stack editing. Invalid or in-progress input
 * (empty, non-numeric, out of range) maps to null so callers can leave the
 * input untouched instead of committing a bad value.
 */
export const MAX_ARMY_COUNT = 4294967295;

export function parseArmyCount(text: string): number | null {
  if (text.trim() === '') return null;
  const parsed = Number(text);
  if (!Number.isInteger(parsed) || parsed < 1 || parsed > MAX_ARMY_COUNT) return null;
  return parsed;
}
