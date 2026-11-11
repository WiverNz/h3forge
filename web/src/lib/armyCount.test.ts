import assert from 'node:assert/strict';
import { test } from 'node:test';
import { parseArmyCount, MAX_ARMY_COUNT } from './armyCount.ts';

test('parseArmyCount accepts positive integers within range', () => {
  assert.equal(parseArmyCount('1'), 1);
  assert.equal(parseArmyCount('250'), 250);
  assert.equal(parseArmyCount(String(MAX_ARMY_COUNT)), MAX_ARMY_COUNT);
  assert.equal(parseArmyCount(' 12 '), 12);
});

test('parseArmyCount rejects empty, partial, and out-of-range input', () => {
  assert.equal(parseArmyCount(''), null);
  assert.equal(parseArmyCount('   '), null);
  assert.equal(parseArmyCount('0'), null);
  assert.equal(parseArmyCount('-5'), null);
  assert.equal(parseArmyCount('2.5'), null);
  assert.equal(parseArmyCount('12x'), null);
  assert.equal(parseArmyCount(String(MAX_ARMY_COUNT + 1)), null);
});

test('parseArmyCount keeps the previous Number-based semantics for exotic numerals', () => {
  // Scientific notation is valid in number inputs and was accepted before.
  assert.equal(parseArmyCount('1e3'), 1000);
});
