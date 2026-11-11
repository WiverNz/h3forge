import assert from 'node:assert/strict';
import { test } from 'node:test';
import { replaceSkill } from './skillReplacement.ts';
const skills = [7, 24, 8].map((skillId, index) => ({skillId, mastery: 3 - index, slot: index + 1, name: null, masteryName: null}));
test('replacement preserves mastery, count and ordering in one absolute edit', () => {
  assert.deepEqual(replaceSkill(skills, 0, 2, [2,7,8,24]), [{id:2,mastery:3}, {id:24,mastery:2}, {id:8,mastery:1}]);
  assert.equal(skills[0]?.skillId, 7);
  assert.deepEqual(replaceSkill(skills, 0, 7, [7]), skills.map(s => ({id:s.skillId,mastery:s.mastery})));
});
test('duplicate, unsupported and invalid-slot replacements are rejected', () => {
  assert.throws(() => replaceSkill(skills, 0, 24, [24]), /Duplicate/);
  assert.throws(() => replaceSkill(skills, 0, 99, [2]), /Unsupported/);
  assert.throws(() => replaceSkill(skills, 3, 2, [2]), /slot/);
});
