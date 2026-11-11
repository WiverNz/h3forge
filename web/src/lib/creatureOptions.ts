/**
 * Pure helpers for the creature combobox: option construction, name
 * filtering, and fixed-popup placement. Kept free of DOM dependencies so the
 * focused node tests exercise the real logic.
 */

export const CREATURE_OPTION_HEIGHT = 30;
export const CREATURE_POPUP_MAX_HEIGHT = 320;
export const CREATURE_POPUP_MIN_WIDTH = 240;
/** Search input row plus popup padding; reserved above the option list. */
export const CREATURE_SEARCH_HEIGHT = 44;
export const POPUP_GAP = 4;
export const VIEWPORT_MARGIN = 8;
/** Prefer opening upward when the space below the trigger is this small. */
export const OPEN_UP_THRESHOLD = 150;

export interface CreatureOption {
  /** null marks the Empty (clear stack) option. */
  id: number | null;
  name: string;
}

export function buildCreatureOptions(choices: { id: number; name: string }[]): CreatureOption[] {
  return [{ id: null, name: 'Empty' }, ...choices.map((choice) => ({ id: choice.id, name: choice.name }))];
}

export function filterCreatureOptions(options: CreatureOption[], query: string): CreatureOption[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return options;
  return options.filter((option) => option.name.toLowerCase().includes(needle));
}

export interface PopupPosition {
  top: number;
  left: number;
  width: number;
  /** Budgeted height for the scrollable option list (search row excluded). */
  listHeight: number;
  openUp: boolean;
}

/**
 * Place the popup relative to the viewport (position: fixed), preferring the
 * space below the trigger and flipping upward when that space is too small.
 * Never overlaps the trigger when there is enough room on either side.
 */
export function computePopupPosition(
  trigger: { top: number; bottom: number; left: number; width: number },
  viewport: { width: number; height: number },
  optionCount: number,
): PopupPosition {
  const spaceBelow = viewport.height - trigger.bottom - VIEWPORT_MARGIN;
  const spaceAbove = trigger.top - VIEWPORT_MARGIN;
  const openUp = spaceBelow < OPEN_UP_THRESHOLD && spaceAbove > spaceBelow;
  const space = openUp ? spaceAbove : spaceBelow;
  const listHeight = Math.max(
    Math.min(
      optionCount * CREATURE_OPTION_HEIGHT,
      CREATURE_POPUP_MAX_HEIGHT,
      Math.max(space - CREATURE_SEARCH_HEIGHT, CREATURE_OPTION_HEIGHT * 2),
    ),
    0,
  );
  const width = Math.max(trigger.width, CREATURE_POPUP_MIN_WIDTH);
  const maxLeft = viewport.width - VIEWPORT_MARGIN - width;
  const left = Math.min(Math.max(VIEWPORT_MARGIN, trigger.left), Math.max(VIEWPORT_MARGIN, maxLeft));
  const top = openUp
    ? Math.max(VIEWPORT_MARGIN, trigger.top - POPUP_GAP - listHeight)
    : trigger.bottom + POPUP_GAP;
  return { top, left, width, listHeight, openUp };
}
