import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';
import { en } from './locales/en.ts';
import { ru } from './locales/ru.ts';
import { de } from './locales/de.ts';
import { es } from './locales/es.ts';
export const resources = { en: { translation: en }, ru: { translation: ru }, de: { translation: de }, es: { translation: es } };
export const i18n = i18next.createInstance();
void i18n.use(initReactI18next).init({
  resources, lng: 'en', fallbackLng: 'en', supportedLngs: ['en', 'ru', 'de', 'es'],
  initImmediate: false, returnNull: false, returnEmptyString: false,
  interpolation: { escapeValue: false },
  parseMissingKeyHandler: () => {
    const language = i18n.resolvedLanguage;
    return resources[language === 'ru' || language === 'de' || language === 'es' ? language : 'en'].translation.common.genericError;
  },
});
export const t = i18n.t.bind(i18n);
export function formatNumber(value: number): string { return new Intl.NumberFormat(i18n.resolvedLanguage ?? 'en').format(value); }
