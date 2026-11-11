import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  buildCreatureOptions,
  computePopupPosition,
  filterCreatureOptions,
  CREATURE_POPUP_MAX_HEIGHT,
} from './creatureOptions.ts';

const CHOICES = [
  { id: 0, name: 'Pikeman' },
  { id: 12, name: 'Arch Mage' },
  { id: 172, name: 'Pixie' },
  { id: 173, name: 'Sea Serpent' },
];

test('buildCreatureOptions prepends Empty and preserves ids and order', () => {
  const options = buildCreatureOptions(CHOICES);
  assert.equal(options.length, CHOICES.length + 1);
  assert.equal(options[0]!.id, null);
  assert.equal(options[0]!.name, 'Empty');
  assert.equal(options[1]!.id, 0);
  assert.equal(options[4]!.id, 173);
});

test('filterCreatureOptions keeps everything for a blank query and matches case-insensitively', () => {
  const options = buildCreatureOptions(CHOICES);
  assert.equal(filterCreatureOptions(options, '').length, options.length);
  assert.equal(filterCreatureOptions(options, '  ').length, options.length);
  const mages = filterCreatureOptions(options, 'arch');
  assert.equal(mages.length, 1);
  assert.equal(mages[0]!.name, 'Arch Mage');
  assert.deepEqual(filterCreatureOptions(options, 'zzz'), []);
  // The Empty option only matches by its own name.
  assert.deepEqual(filterCreatureOptions(options, 'empt').map((o) => o.id), [null]);
});

test('computePopupPosition opens below with a capped list height for full catalogs', () => {
  const pos = computePopupPosition(
    { top: 100, bottom: 128, left: 40, width: 220 },
    { width: 1280, height: 800 },
    189,
  );
  assert.equal(pos.openUp, false);
  assert.equal(pos.top, 132);
  assert.equal(pos.left, 40);
  assert.equal(pos.width, 240); // widened to the minimum
  assert.equal(pos.listHeight, CREATURE_POPUP_MAX_HEIGHT);
});

test('computePopupPosition flips upward when the space below is too small', () => {
  const pos = computePopupPosition(
    { top: 700, bottom: 728, left: 40, width: 260 },
    { width: 1280, height: 760 },
    60,
  );
  assert.equal(pos.openUp, true);
  // The list hangs above the trigger without overlapping it.
  assert.ok(pos.top + pos.listHeight <= 700 - 4);
  assert.ok(pos.top >= 8);
  // 60 options * 30px exceed the space, so the list is space-bounded.
  assert.ok(pos.listHeight < 60 * 30);
});

test('computePopupPosition clamps the width and keeps small lists compact', () => {
  const cramped = computePopupPosition(
    { top: 300, bottom: 328, left: 1240, width: 200 },
    { width: 1280, height: 800 },
    3,
  );
  assert.ok(cramped.left + cramped.width <= 1280 - 8);
  assert.equal(cramped.listHeight, 3 * 30);

  const tight = computePopupPosition(
    { top: 390, bottom: 418, left: 100, width: 220 },
    { width: 1280, height: 800 }, // tiny band in the middle of the screen
    189,
  );
  assert.ok(tight.listHeight >= 2 * 30);
  assert.ok(tight.top >= 8);
});
