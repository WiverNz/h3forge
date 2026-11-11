import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createInstance } from 'i18next';
import { detectLanguage, languageKey, languages, persistLanguage, readLanguage } from '../i18n/locale.ts';
import { i18n, resources, t, formatNumber } from '../i18n/index.ts';
import { describeError, errorSummary } from '../i18n/errors.ts';
import { decodeRaw } from './legacyText.ts';

function leaves(object: object, prefix = ''): Record<string, string> {
  return Object.fromEntries(Object.entries(object).flatMap(([key, value]) => {
    const full = prefix ? `${prefix}.${key}` : key;
    return typeof value === 'string' ? [[full, value]] : Object.entries(leaves(value as object, full));
  }));
}
function tokens(value: string): string[] {
  return [...value.matchAll(/{{\s*([^}]+)\s*}}/g)].map(match => match[1]!).sort();
}
test('all four catalogs have the English keys and matching interpolation parameters', () => {
  const english = leaves(resources.en.translation);
  for (const language of languages) {
    const catalog = leaves(resources[language].translation);
    assert.deepEqual(Object.keys(catalog).sort(), Object.keys(english).sort());
    for (const [key, value] of Object.entries(english)) {
      assert.ok(catalog[key]?.trim(), `${language}:${key} is empty`);
      assert.deepEqual(tokens(catalog[key]!), tokens(value), `${language}:${key}`);
    }
  }
});
test('language detection honors preferred order, supported regional tags and English fallback', () => {
  for (const [tag, expected] of [['ru-RU', 'ru'], ['de-DE', 'de'], ['es-MX', 'es'], ['en-GB', 'en'], ['DE_at', 'de']]) {
    assert.equal(detectLanguage([tag!]), expected);
  }
  assert.equal(detectLanguage(['fr-FR', 'es-ES', 'de-DE']), 'es');
  assert.equal(detectLanguage([]), 'en');
  assert.equal(detectLanguage(['ja-JP', '', 'unknown']), 'en');
});
test('manual preference survives new reads, reset removes it, and storage failures are safe', () => {
  const values = new Map<string, string>();
  const storage = { getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => { values.set(key, value); }, removeItem: (key: string) => { values.delete(key); } };
  assert.equal(readLanguage(storage), null);
  for (const language of languages) {
    assert.ok(persistLanguage(storage, language));
    assert.equal(readLanguage(storage) ?? detectLanguage(['fr-FR']), language);
  }
  assert.ok(persistLanguage(storage, null));
  assert.equal(readLanguage(storage), null);
  values.set(languageKey, 'unsupported');
  assert.equal(readLanguage(storage), null);
  const fail = () => { throw new Error('storage denied'); };
  assert.equal(readLanguage({ getItem: fail }), null);
  assert.equal(persistLanguage({ setItem: fail, removeItem: fail }, 'ru'), false);
});
test('Russian plurals and all locale interpolation, numbers and error summaries update', async () => {
  const cyrillic = [0xc8, 0xe2, 0xe0, 0xed];
  const raw = [...cyrillic];
  const utf8 = [...new TextEncoder().encode('Иван Álvarez')];
  const storedValue = 1234567;
  for (const language of languages) {
    await i18n.changeLanguage(language);
    assert.equal(decodeRaw(cyrillic, 'cp1251'), 'Иван');
    assert.equal(decodeRaw(utf8, 'auto'), 'Иван Álvarez');
    assert.deepEqual(cyrillic, raw);
    assert.equal(storedValue, 1234567);
    assert.equal(formatNumber(storedValue), new Intl.NumberFormat(language).format(storedValue));
    for (const count of [0, 1, 2, 5, 21]) assert.ok(!t('common.unsaved', { count }).includes('{{'));
    assert.ok(t('common.saved', { file: '<b>Иван & Álvarez</b>' }).includes('<b>Иван & Álvarez</b>'));
    assert.ok(errorSummary(describeError({ code: 'file-too-large', params: { limit: 1234 } })).includes(formatNumber(1234)));
    assert.equal(errorSummary(describeError({ code: 'file-too-large' })), t('common.genericError'));
    assert.equal(errorSummary(describeError({ code: 'file-too-large', params: { limit: '1234' } })), t('common.genericError'));
    assert.equal(errorSummary(describeError({ code: 'unknown' })), t('common.genericError'));
    assert.equal(errorSummary(describeError({ code: 'edit-failed', cause: 'invalid-value' })), t('messages.invalidValue'));
    assert.equal(errorSummary(describeError({ code: 'open-failed', cause: 'corrupt-input' })), t('messages.corrupt'));
    assert.equal((t as (key: string) => string)('missing.runtime.key'), t('common.genericError'));
    assert.equal(describeError('legacy diagnostic').message, 'legacy diagnostic');
  }
  await i18n.changeLanguage('ru');
  assert.equal(t('common.unsaved', { count: 1 }), '1 несохранённое изменение');
  assert.equal(t('common.unsaved', { count: 2 }), '2 несохранённых изменения');
  assert.equal(t('common.unsaved', { count: 5 }), '5 несохранённых изменений');
  assert.equal(t('common.unsaved', { count: 21 }), '21 несохранённое изменение');
  await i18n.changeLanguage('en');
});
test('missing and empty translations fall back to English without loading network resources', async () => {
  const instance = createInstance();
  await instance.init({ lng: 'de', fallbackLng: 'en', returnEmptyString: false, returnNull: false,
    resources: { en: { translation: { common: { settings: 'Settings', language: 'Language' } } }, de: { translation: { common: { settings: '' } } } } });
  assert.equal(instance.t('common.settings'), 'Settings');
  assert.equal(instance.t('common.language'), 'Language');
});
