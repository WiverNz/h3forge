import assert from 'node:assert/strict';
import { test } from 'node:test';
import { decodeRaw, decodeText } from './legacyText.ts';

// "Оррин" as stored by the game: CP1251 bytes for the supplied HotA save,
// and the same string as strict UTF-8 for modern tooling.
const ORRIN_CP1251 = [0xce, 0xf0, 0xf0, 0xe8, 0xed];
const ORRIN_UTF8 = new TextEncoder().encode('Оррин');

test('ASCII decodes identically in every mode', () => {
  const ascii = Array.from(new TextEncoder().encode('Captain Nemo'));
  assert.equal(decodeRaw(ascii, 'auto'), 'Captain Nemo');
  assert.equal(decodeRaw(ascii, 'cp1251'), 'Captain Nemo');
});

test('valid UTF-8 decodes strictly in auto mode', () => {
  assert.equal(decodeRaw(ORRIN_UTF8, 'auto'), 'Оррин');
});

test('CP1251 bytes fall back automatically and match forced decoding', () => {
  assert.equal(decodeRaw(ORRIN_CP1251, 'auto'), 'Оррин');
  assert.equal(decodeRaw(ORRIN_CP1251, 'cp1251'), 'Оррин');
});

test('town garrison hero reference decodes from its raw CP1251 bytes', () => {
  // Garrisoned hero of Tadorn Fens in the supplied save, as sent in
  // HeroRefDto.name; the lossy field is what used to be rendered.
  const sandro = { raw: [0xd1, 0xe0, 0xed, 0xe4, 0xf0, 0xee], lossy: 'Ñàíäðî' };
  assert.equal(decodeText(sandro, 'auto'), 'Сандро');
  assert.equal(decodeText(sandro, 'cp1251'), 'Сандро');
});

test('decodeText uses the preserved raw bytes', () => {
  const dto = { raw: ORRIN_CP1251, lossy: 'wrong' };
  assert.equal(decodeText(dto, 'auto'), 'Оррин');
});
