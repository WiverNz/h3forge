export const languages = ['en', 'ru', 'de', 'es'] as const;
export type Language = typeof languages[number];
export const languageKey = 'h3forge.uiLanguage.v1';
export function isLanguage(value: unknown): value is Language {
  return languages.some(language => language === value);
}
export function detectLanguage(tags: readonly string[]): Language {
  for (const tag of tags) {
    const primary = tag.toLowerCase().split(/[-_]/)[0];
    if (isLanguage(primary)) return primary;
  }
  return 'en';
}
export function readLanguage(storage: Pick<Storage, 'getItem'>): Language | null {
  try { const value = storage.getItem(languageKey); return isLanguage(value) ? value : null; }
  catch { return null; }
}
export function persistLanguage(storage: Pick<Storage, 'setItem' | 'removeItem'>, language: Language | null): boolean {
  try {
    if (language) storage.setItem(languageKey, language);
    else storage.removeItem(languageKey);
    return true;
  } catch { return false; }
}
