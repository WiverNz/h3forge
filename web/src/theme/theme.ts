// Adapted from FlightTrace's theme runtime. See licenses/FlightTrace-MIT.txt.
export type ThemePreference = 'light' | 'dark' | 'system';
// Keep the pre-paint script in public/theme-init.js in sync with this key.
export const themeKey = 'h3forge.theme.v1';

export function isTheme(value: string | null): value is ThemePreference {
  return value === 'light' || value === 'dark' || value === 'system';
}

export function readTheme(): ThemePreference {
  try {
    const value = localStorage.getItem(themeKey);
    if (isTheme(value)) return value;
  } catch { /* Storage can be unavailable; follow the OS. */ }
  return 'system';
}

export function selectTheme(preference: ThemePreference): boolean {
  if (preference === 'system') delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = preference;
  try {
    if (preference === 'system') localStorage.removeItem(themeKey);
    else localStorage.setItem(themeKey, preference);
    return true;
  } catch { return false; }
}
