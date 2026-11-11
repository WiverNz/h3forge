// Apply before paint; external script respects the desktop's script-src 'self'.
// Keep the key and accepted values aligned with src/theme/theme.ts.
try {
  var savedTheme = localStorage.getItem('h3forge.theme.v1');
  if (savedTheme === 'light' || savedTheme === 'dark') {
    document.documentElement.dataset.theme = savedTheme;
  }
} catch { /* CSS follows the OS when storage is unavailable. */ }
