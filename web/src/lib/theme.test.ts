import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { readTheme, selectTheme, themeKey } from '../theme/theme.ts';

test('theme changes persist, system resets, and startup restores the same preference', () => {
  const bootstrap = readFileSync(new URL('../../public/theme-init.js', import.meta.url), 'utf8');
  const values = new Map<string, string>();
  const storage = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => { values.set(key, value); },
    removeItem: (key: string) => { values.delete(key); },
  };
  const document = { documentElement: { dataset: {} as Record<string, string> } };
  const oldStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const oldDocument = Object.getOwnPropertyDescriptor(globalThis, 'document');
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: storage });
  Object.defineProperty(globalThis, 'document', { configurable: true, value: document });
  try {
    assert.equal(readTheme(), 'system');
    for (const theme of ['light', 'dark', 'system'] as const) {
      assert.equal(selectTheme(theme), true);
      assert.equal(readTheme(), theme);
      assert.equal(document.documentElement.dataset.theme, theme === 'system' ? undefined : theme);
      const freshDocument = { documentElement: { dataset: {} as Record<string, string> } };
      runInNewContext(bootstrap, { localStorage: storage, document: freshDocument });
      assert.deepEqual(freshDocument.documentElement.dataset, document.documentElement.dataset);
    }
    values.set(themeKey, 'invalid');
    assert.equal(readTheme(), 'system');
    const freshDocument = { documentElement: { dataset: {} } };
    runInNewContext(bootstrap, { localStorage: storage, document: freshDocument });
    assert.deepEqual(freshDocument.documentElement.dataset, {});
    const failingStorage = { getItem() { throw new Error('blocked'); }, setItem() { throw new Error('quota'); } };
    Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: failingStorage });
    assert.equal(readTheme(), 'system');
    assert.equal(selectTheme('light'), false);
    assert.equal(document.documentElement.dataset.theme, 'light');
    assert.doesNotThrow(() => runInNewContext(bootstrap, { localStorage: failingStorage, document: freshDocument }));
  } finally {
    if (oldStorage) Object.defineProperty(globalThis, 'localStorage', oldStorage);
    else Reflect.deleteProperty(globalThis, 'localStorage');
    if (oldDocument) Object.defineProperty(globalThis, 'document', oldDocument);
    else Reflect.deleteProperty(globalThis, 'document');
  }
});
