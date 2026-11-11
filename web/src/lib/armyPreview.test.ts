import assert from 'node:assert/strict';
import { test } from 'node:test';
import { armyPreview } from './armyPreview.ts';

test('army preview preserves stack order and duplicates, counting only overflow', () => {
  const preview = armyPreview([44, 0, 44, 153, 199]);
  assert.equal(preview.icons.join(','), '44,0,44');
  assert.equal(preview.remaining, 2);
  assert.equal(armyPreview([0]).remaining, 0);
  assert.equal(armyPreview([]).icons.length, 0);
  assert.equal(armyPreview([1, 2, 3]).remaining, 0);
});
