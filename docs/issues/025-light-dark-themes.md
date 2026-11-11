# H3F-025 - FlightTrace light and dark themes in Settings

Status: implemented with automated verification, 2026-11-11. Native visual/relaunch smoke pending. Priority: P1. Dependencies: H3F-024 (Settings and localization).

## Requested outcome

Adapt FlightTrace's warm parchment and warm charcoal themes and expose the choice in H3Forge Settings.

## Implementation

- Settings has a labeled Theme selector with Light, Dark and System options in all four interface languages. Changes apply immediately through the root `data-theme` attribute without remounting editors or touching save data.
- Explicit choices persist under `h3forge.theme.v1`. System clears the override and follows live `prefers-color-scheme` changes through CSS. Missing/invalid preferences and unavailable storage default to System. Failed writes retain the session choice and display a localized notice.
- External synchronous `public/theme-init.js` applies saved preferences before rendering, compatible with desktop `script-src 'self'`. No inline-script permission or dependency change is needed.
- Palette values and preference behavior are adapted from FlightTrace's `web/src/styles/tokens.css` and `web/src/theme/theme.ts`, inspected read-only on 2026-11-11. Retained its MIT notice in `licenses/FlightTrace-MIT.txt` and updated attribution.
- Kept the compact layout and Segoe UI typography. Converted fixed dark editor surfaces, equipment picker, status labels, hover and selected states to semantic colors. Light-mode accent/status/owner text uses deeper colors for readability. Original artwork is unchanged.

## Dark button preference follow-up

Restored H3Forge's original dark-mode gold buttons (`#e2bc72`), hover (`#f0cf93`) and button text (`#211c12`) at user request. Matching accent text, selection fills and borders use gold again. Both explicit Dark and System-dark receive the same values; the light palette and theme selector remain unchanged. Production build and diff whitespace check passed for this CSS-only follow-up.

## Verification

Settings dismissal follow-up: clicking or tapping outside the open Settings panel now closes it. Pointer events inside the panel keep its controls usable; the outside interaction is not prevented. The document listener is removed on unmount. Native interaction smoke remains pending.

- `npm.cmd --prefix web --cache .local/npm-cache run build`: TypeScript and Vite production build passed.
- `npm.cmd --prefix web --cache .local/npm-cache test`: all 25 tests passed, including preference changes, storage reset/failure, invalid preferences and startup-script restoration.
- `npm.cmd --prefix web --cache .local/npm-cache run test:i18n-ui`: four-language shell/editor render passed, including all theme options.
- Node test workers initially hit sandbox `EPERM`; the authorized outside-sandbox rerun passed. No dependency install was needed.
- Native WebView2 visual checks, real application relaunch and live Windows theme changes remain manual checks. Automated DOM/storage tests and static rendering do not establish those results.
- No Rust, save parsing, binary format, reference repository or original game data changes. No other issue started.
