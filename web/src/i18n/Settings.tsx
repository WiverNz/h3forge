import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { i18n } from './index';
import { detectLanguage, isLanguage, languages, persistLanguage, readLanguage, type Language } from './locale';
import { isTheme, readTheme, selectTheme } from '../theme/theme';

export async function systemLanguage(): Promise<Language> {
  try {
    return detectLanguage(isTauri() ? await invoke<string[]>('get_ui_languages') : navigator.languages);
  } catch { return 'en'; }
}
export async function initializeLanguage() {
  let saved: Language | null = null;
  try { saved = readLanguage(localStorage); } catch { /* unavailable storage */ }
  await i18n.changeLanguage(saved ?? await systemLanguage());
  document.documentElement.lang = i18n.resolvedLanguage ?? 'en';
}
export function Settings() {
  const { t } = useTranslation();
  const panelRef = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    const closeOutside = (event: PointerEvent) => {
      const panel = panelRef.current;
      if (panel?.open && event.target instanceof Node && !panel.contains(event.target)) {
        panel.open = false;
      }
    };
    document.addEventListener('pointerdown', closeOutside, true);
    return () => document.removeEventListener('pointerdown', closeOutside, true);
  }, []);
  const [warning, setWarning] = useState(false);
  const [busy, setBusy] = useState(false);
  const [theme, setTheme] = useState(readTheme);
  const [themeWarning, setThemeWarning] = useState(false);
  const select = async (choice: Language | null) => {
    setBusy(true);
    const language = choice ?? await systemLanguage();
    let persisted = false;
    try { persisted = persistLanguage(localStorage, choice); } catch { /* unavailable storage */ }
    await i18n.changeLanguage(language);
    document.documentElement.lang = language;
    setWarning(!persisted); setBusy(false);
  };
  return <details ref={panelRef} className="language-settings">
    <summary>{t('common.settings')}</summary>
    <label>{t('common.theme')} <select value={theme} onChange={event => {
      const value = event.target.value;
      if (!isTheme(value)) return;
      setTheme(value);
      setThemeWarning(!selectTheme(value));
    }}>
      <option value="light">{t('common.themeLight')}</option>
      <option value="dark">{t('common.themeDark')}</option>
      <option value="system">{t('common.themeSystem')}</option>
    </select></label>
    {themeWarning && <p role="status">{t('common.themePersistence')}</p>}
    <label>{t('common.language')} <select disabled={busy} value={i18n.resolvedLanguage ?? 'en'} onChange={event => {
      if (isLanguage(event.target.value)) void select(event.target.value);
    }}>{languages.map(language => <option key={language} value={language} lang={language}>
      {{ en: 'English', ru: 'Русский', de: 'Deutsch', es: 'Español' }[language]}
    </option>)}</select></label>
    <button type="button" disabled={busy} onClick={() => void select(null)}>{t('common.system')}</button>
    <p>{t('common.integerHint')}</p>
    {warning && <p role="status">{t('common.persistence')}</p>}
  </details>;
}
