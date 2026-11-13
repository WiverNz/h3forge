import assert from 'node:assert/strict';
import { test } from 'node:test';
import { sortList, matchesFacet, toggleFacet, ownerKey, ownerValue, creatureOwner, teamKey,
  creatureName, creatureQuantity, matchesCreatureType, selectedRecord, readListSort, writeListSort, preferenceKey } from './listView.ts';
import type { NeutralArmyDto, PlayerDto } from '../types/api.ts';

test('locale collation, ties and missing values in both directions preserve source arrays', () => {
  const rows = ['Яна', 'Adam', 'Борис', 'Änne', 'adam', 'Елена', 'Ёлка', null].map((name, id) => ({ id, name }));
  const snapshot = JSON.stringify(rows);
  for (const locale of ['en', 'ru', 'de', 'es']) {
    const collator = new Intl.Collator(locale, { numeric: true, sensitivity: 'base' });
    for (const direction of ['asc', 'desc'] as const) {
      const result = sortList(rows, { key: 'name', direction }, locale, row => row.name);
      assert.equal(result.at(-1)?.name, null);
      assert.ok(result.findIndex(row => row.id === 1) < result.findIndex(row => row.id === 4));
      const known = result.filter(row => row.name !== null);
      for (let index = 1; index < known.length; index++) {
        assert.ok(collator.compare(known[index - 1]!.name!, known[index]!.name!) * (direction === 'asc' ? 1 : -1) <= 0);
      }
    }
  }
  assert.equal(JSON.stringify(rows), snapshot);
  assert.deepEqual(sortList(rows, { key: 'original', direction: 'desc' }, 'en', () => null), [...rows].reverse());
});

test('numeric sorting, coordinates and neutral/unknown ownership are deterministic', () => {
  const values = [10, null, 2, 2, 0];
  assert.deepEqual(sortList(values, { key: 'quantity', direction: 'desc' }, 'en', value => value), [10, 2, 2, 0, null]);
  const coordinates = [[0, 10, 1], [1, 0, 0], [0, 2, 10], [0, 2, 2]];
  assert.deepEqual(sortList(coordinates, { key: 'coordinates', direction: 'asc' }, 'en', value => value),
    [[0, 2, 2], [0, 2, 10], [0, 10, 1], [1, 0, 0]]);
  assert.equal(ownerKey({ kind: 'neutral', player: null }), 'neutral');
  for (const player of [null, -1, 8, 255]) assert.equal(ownerKey({ kind: 'player', player }), 'unknown');
  assert.equal(ownerKey({ kind: 'player', player: 7 }), '7');
  assert.deepEqual(sortList(['unknown', 'neutral', '7', '0'], { key: 'owner', direction: 'desc' }, 'en', ownerValue), ['neutral', '7', '0', 'unknown']);
});

const group = (patch: Partial<NeutralArmyDto> = {}): NeutralArmyDto => ({
  id: 10, kind: 'garrison', position: [10, 2, 0], creatureId: null, name: null,
  owner: 255, allowedCreatures: [], dirtySlots: [0], readOnlyReason: null, initialArmy: [{ creatureId: 1, count: 9999, name: 'Initial' }],
  army: [{ creatureId: 1, name: 'Архангел', count: 10, empty: false, editable: true },
    { creatureId: 2, name: 'Dragon', count: 2, empty: false, editable: true }], ...patch,
});
test('current creature stacks drive type and quantity; empty and unavailable remain distinct', () => {
  assert.equal(creatureName(group()), 'Архангел');
  assert.equal(creatureQuantity(group()), 12);
  assert.ok(matchesCreatureType(group(), ['2', '3']));
  assert.equal(matchesCreatureType(group(), ['3']), false);
  assert.equal(creatureQuantity(group({ army: [] })), 0);
  assert.equal(creatureQuantity(group({ army: null })), null);
  assert.equal(creatureName(group({ army: [] })), null);
  assert.ok(matchesCreatureType(group({ army: [] }), ['empty']));
  assert.ok(matchesCreatureType(group({ army: null }), ['unknown']));
  assert.equal(creatureOwner(group()), 'neutral');
  assert.equal(creatureOwner(group({ owner: null })), 'unknown');
  assert.equal(creatureOwner(group({ owner: 8 })), 'unknown');
});

test('multi-select OR combines with search/other facets using AND without changing selection', () => {
  const rows = [group(), group({ id: 11, owner: 0 }), group({ id: 12, owner: 1 })];
  let owners = toggleFacet([], 'neutral');
  owners = toggleFacet(owners, '0');
  const visible = rows.filter(row => matchesFacet(owners, creatureOwner(row)) && matchesCreatureType(row, ['2']) && creatureName(row)?.includes('Арх'));
  assert.deepEqual(visible.map(row => row.id), [10, 11]);
  assert.equal(selectedRecord(rows, 12), rows[2]);
  assert.equal(selectedRecord(rows, 999), rows[0]);
  assert.equal(selectedRecord([], 12), undefined);
  assert.deepEqual(toggleFacet(owners, '0'), ['neutral']);
  assert.ok(matchesFacet([], 'unknown'));
});

test('team facets distinguish map-authored teams, absent teams and unverified metadata', () => {
  const player = (mapLinked: boolean, team: number | null) => ({ details: { mapLinked, team } }) as PlayerDto;
  assert.equal(teamKey(player(true, 2)), '2');
  assert.equal(teamKey(player(true, null)), 'none');
  assert.equal(teamKey(player(false, null)), 'unknown');
  assert.equal(teamKey(player(false, 2)), 'unknown');
});

test('only valid sort preferences persist; unavailable or corrupt storage is harmless', () => {
  const previous = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const values = new Map<string, string>();
  try {
    Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: {
      getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => values.set(key, value),
    } });
    writeListSort('heroes', { key: 'name', direction: 'desc' });
    assert.deepEqual(readListSort('heroes', ['original', 'name'], 'original'), { key: 'name', direction: 'desc' });
    assert.deepEqual(readListSort('towns', ['original', 'name'], 'original'), { key: 'original', direction: 'asc' });
    for (const invalid of ['oops', '{}', '{"key":"bad","direction":"asc"}', '{"key":"name","direction":"bad"}']) {
      values.set(preferenceKey('heroes'), invalid);
      assert.deepEqual(readListSort('heroes', ['original', 'name'], 'original'), { key: 'original', direction: 'asc' });
    }
    Object.defineProperty(globalThis, 'localStorage', { configurable: true, get() { throw new Error('unavailable'); } });
    assert.doesNotThrow(() => writeListSort('heroes', { key: 'name', direction: 'asc' }));
    assert.deepEqual(readListSort('heroes', ['original'], 'original'), { key: 'original', direction: 'asc' });
  } finally {
    if (previous) Object.defineProperty(globalThis, 'localStorage', previous);
    else Reflect.deleteProperty(globalThis, 'localStorage');
  }
});
