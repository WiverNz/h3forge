import assert from 'node:assert/strict';
import { test } from 'node:test';
import { equipmentLabel } from './slotPresentation.ts';

test('equipment blanks do not depend on the second word or guess zero is empty', () => {
  assert.equal(equipmentLabel([255, 255, 255, 255, 0, 0, 0, 0]), 'Empty');
  assert.equal(equipmentLabel([42, 0, 0, 0, 255, 255, 255, 255]), 'Artifact not identified');
  assert.equal(equipmentLabel(Array(8).fill(0)), 'Artifact not identified');
  assert.equal(equipmentLabel([]), 'Artifact not identified');
});
