import assert from 'node:assert/strict';
import { test } from 'node:test';
import { playerColor } from './playerColor.ts';
test('binary owner IDs map to the eight player colors; neutral and invalid stay gray', () => {
  assert.deepEqual(Array.from({length: 8}, (_, player) => playerColor({kind: 'player', player})),
    ['red', 'blue', 'tan', 'green', 'orange', 'purple', 'teal', 'pink']);
  for (const player of [null, -1, 8, 255]) assert.equal(playerColor({kind: 'player', player}), 'neutral');
  assert.equal(playerColor({kind: 'neutral', player: 0}), 'neutral');
});
