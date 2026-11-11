# H3F-024 - Multi-Language Support (i18n)

Status: implemented with automated verification, 2026-11-11; native visual/relaunch smoke and human translation review remain pending. Milestone: M5, after core editing and before final Windows release. Priority: P1. Dependencies: H3F-009 (completed core editing baseline, including 001/003/007/008 transitively). Independent of H3F-019 and every other unfinished feature; neither H3F-016 nor optional M4 work is a prerequisite.

## Implementation and verification (2026-11-11)

- Pinned `i18next` 25.6.3 and `react-i18next` 16.3.5; retained React/TypeScript/Vite and every Tauri alpha pin. Added app-only `sys-locale` 0.3.2. Its inspected Windows implementation calls `GetUserPreferredUILanguages`; H3Forge introduces no unsafe code or new Tauri plugin. Both lockfiles updated.
- Added `web/src/i18n/locales/{en,ru,de,es}.ts`, English-derived resource typing, interpolation, English fallback, a generic production missing-key fallback, and Russian count plurals. Resources ship in the frontend bundle; no translation service, network loader or generated runtime catalog is used.
- Migrated the existing component/page labels, actions, accessible names, placeholders, tooltips, statuses and local validation. Language changes rerender the existing tree without changing record IDs, tab IDs, CSS tokens, dirty-field identifiers, numeric inputs or edit operations. Settings offers four autonyms, immediate switching, guarded persistence, a system-language reset and a persistence-failure notice. `document.lang` follows selection.
- The new narrowly permitted `get_ui_languages` command supplies native preferences. Stored `h3forge.uiLanguage.v1` wins; absent/invalid preference uses the first supported system language, otherwise English. Browser preview uses navigator preferences. Existing `h3forge.encoding` and decoder behavior are independent and unchanged.
- Rust errors retain diagnostic details and now carry numeric parameters and typed cause classifications. Frontend maps codes/causes to translated summaries, retains recovery details, and treats malformed/unknown descriptors safely. Read-only DTO fields serialize as `{code: "read-only", details}` without changing Rust capability decisions. Native picker commands accept bounded display-only labels; fixed filters, grants, destinations and save protections remain Rust-owned.
- Detailed parser/resource diagnostics, low-level capability explanations, artifact-effect/catalog text and source-map provenance can still appear in their original English as technical/game-data details. They are not machine-translated or used to choose encoding. This is not a claim that original game catalogs or every technical explanation have four-language translations. Broad read-only descriptors use a localized read-only label plus the exact technical explanation, rather than inventing a more specific localized cause from prose.
- Verification passed: 24 frontend tests (including five i18n groups); four-language static rendering of shell, army/player/town views and feedback; TypeScript and Vite production build; Rust check/build; two localization unit tests; two command-permission tests; five synthetic overview tests. A private-copy `edit_flow::save_as_roundtrip_backup_and_original_intact` run passed with an absolute fixture root and no skip, exercising edits, backup, Save As and reload. Original reference directories were not modified. Parsing/format specifications did not change.
- Pending manual gates: WebView2 four-language visual sweep at minimum window size, actual native dialog labels, OS first-launch behavior, persistence across real application relaunch, live draft/focus preservation and native open/edit/save/reopen. Computer-use launch was rejected with `Computer Use was not approved to use h3forge`; static rendering and session tests do not replace these checks. Translation text is authored but not claimed as native-speaker reviewed. Final release remains gated on this review; no other issue was started.

Maintenance: edit the locale TypeScript files directly, keep protocol/data identifiers out of translated strings, and run `npm --prefix web test`, `npm --prefix web run test:i18n-ui` and `npm --prefix web run build`. The rendering check uses the esbuild executable already supplied by the pinned Vite dependency graph and writes its bundle only to ignored `.local/`. No extraction script is required for builds. The design below records the original plan; the implementation notes above describe verified current behavior and its limits.

## Outcome and boundaries

Provide English (default/fallback), Russian, German and Spanish interface languages. Localize the currently delivered editor incrementally, including accessibility text and actionable safety explanations. Add a small Settings surface within this issue if none exists. Do not expand editing capabilities, change binary formats, translate save contents, or require complete localized game-name catalogs. The original planning session changed documentation only; implementation was authorized separately.

## Inspected baseline and string inventory

Verified against the current source, rather than the architecture document's original proposed layout: React 19.2.0, TypeScript 5.9.3, Vite 7.2.2 and `@tauri-apps/api` 3.0.0-alpha.3. `App.tsx` renders `SaveOverviewPage` directly; there is no Settings screen, router or i18n dependency. The following is the migration checklist for all current user-facing string surfaces, not a duplicated literal catalog. Paths below are relative to `web/src/` unless specified.

| Source | Text to migrate |
|---|---|
| `pages/SaveOverviewPage.tsx` | Open/Save/Save As/install selection, Heroes/Towns/Players navigation, busy/empty/error/saved states, backup notices, diagnostics, pending-edit field labels, footer and any confirmation copy |
| `components/SaveBanner.tsx`, `MapLinkRow.tsx` | Summary facts, compatibility and encoding labels, diagnostic headings/messages, map match strengths, search/link actions, source-map facts and warnings |
| `HeroList.tsx`, `TownList.tsx` | Search placeholders, filters, counts, selection/empty states, owner/status labels, duplicate-name and confidence warnings, army/garrison summaries |
| `HeroDetail.tsx`, `TownDetail.tsx` | Tabs, field/group labels, attributes/progression, revert actions, editable/read-only/pending badges, validation, learned/available spells, garrison/visitor links, raw-data and map-context captions |
| `ArmyEditor.tsx`, `SkillsEditor.tsx` | Slot/quantity labels, mastery, add/remove/reorder actions, bounds/invalid-input feedback, pending-change and preservation explanations |
| `BackpackEditor.tsx`, `EquipmentEditor.tsx`, `ArtifactSlot.tsx`, `ArtifactPicker.tsx` | Equipment positions, empty/unknown/scroll descriptions, search, add/replace/remove/move/equip actions, picker headings, locked-item and coupled-effect explanations |
| `CreatureCombobox.tsx`, `SkillCombobox.tsx`, `SpellCombobox.tsx` | Search/replace/add/empty/no-match labels and keyboard-accessible control names |
| `TownBuildings.tsx`, `TownGuild.tsx` | Building groups and states, locked/editable explanations, guild levels and spell actions; actual building/spell names follow the separate catalog policy below |
| `PlayersPanel.tsx` | Resource/general/exploration tabs, resources and colors, active/uncertain statuses, controller/alliance/obelisk details, disabled revelation explanations, original-value tooltips, validation and revert feedback |
| `Portrait.tsx`, `CreaturePortrait.tsx`, `ArtifactIcon.tsx`, `SkillIcon.tsx`, `SpellIcon.tsx`, `TownIcon.tsx` | Missing/failed/loading image explanations, alternative text and tooltips |
| `lib/legacyText.ts`, `slotPresentation.ts`, `creatureOptions.ts`, `artifactIconError.ts`; `api/`, `types/api.ts` | Encoding option labels, Empty/unknown-item labels, error normalization and DTO-carried text. Keep CSS tokens from `playerColor.ts`, IDs and protocol enum values untranslated |
| Desktop Rust `src/{dto,session,installation,map_link,player_details}.rs` | Error messages, read-only reasons, edit warnings, parser/resource/map diagnostics, status reasons, generated player labels, pending field/value descriptions and catalog names forwarded to React |
| Desktop Rust `src/commands.rs` | Open/save/map/folder dialog titles and file-filter descriptions. System-owned dialog buttons remain Windows-localized |

For every component include visible text, `title`, `placeholder`, `alt`, `aria-label`, option labels and generated sentences. Preserve paths, filenames, versions, raw hex, diagnostic codes, user-authored map descriptions and decoded custom names as data. Keep stable tab/DOM IDs and selection keys independent of translated labels (notably Players tabs currently use labels as IDs).

## Recommended implementation design

Use **i18next + react-i18next**, with bundled resources and the standard React hook. Two direct runtime dependencies provide fallback, interpolation, plural rules (including Russian) and reactive language switching without maintaining a custom translation engine. Do not add a detector, HTTP backend, ICU plugin, translation service, extraction framework or Tauri plugin. Exact package versions and peer compatibility must be selected/pinned once during implementation against the existing React/TS graph; no install or compatibility build was performed in this planning session. Browser-only translation code requires no Tauri runtime changes, so compatibility with the pinned alpha is an architectural assessment, to be confirmed by the Windows smoke test.

Planned structure:

```text
web/src/i18n/
  index.ts                 # one configured instance, bundled resources
  locale.ts                # supported languages, selection/persistence
  i18next.d.ts             # CustomTypeOptions derived from English
  locales/{en,ru,de,es}.ts  # UTF-8 TypeScript resource objects
```

Use one namespace with nested semantic groups such as `common`, `settings`, `save`, `heroes`, `towns`, `players`, `validation`, `errors` and `diagnostics`. English is the canonical `as const` schema; other files satisfy its key structure with string-valued leaves, allowing each locale's required plural forms. Derive typed keys through i18next's resource typing, enable strict key checks and use typed key maps for runtime enums/codes. Verify interpolation token parity separately; literal English values must not become required translation values. Prefer the simplest supported typed API in the chosen pinned version; no code generator is needed.

Use named `{{parameter}}` interpolation, whole sentences and native i18next count plurals instead of concatenating translated fragments. Render as React text, never HTML. Configure `supportedLngs`, English `fallbackLng`, and treat empty/null translations as missing. Missing localized keys resolve to English; a key missing even in English is a development/test failure and uses a localized generic message in production. Unknown backend codes must not become unchecked translation keys.

Official references checked for this recommendation: [React setup](https://react.i18next.com/latest), [TypeScript resource typing](https://www.i18next.com/overview/typescript), [fallback](https://www.i18next.com/principles/fallback), [interpolation](https://www.i18next.com/translation-function/interpolation). These establish library capabilities, not a tested dependency resolution for H3Forge.

## Settings, detection and number presentation

- Add an accessible Language selector with recognizable autonyms: English, Русский, Deutsch, Español. Switch immediately, update `document.documentElement.lang`, and preserve selection, drafts, pending edits and errors. Store message descriptors rather than already-translated strings so existing feedback updates too.
- Persist explicit choices under a versioned, UI-only key such as `h3forge.uiLanguage.v1`, following the existing guarded localStorage pattern. A valid stored choice wins across restarts; malformed/unsupported values act as absent. If storage fails, keep the session choice and show a localized persistence warning.
- On first launch without a saved choice, obtain the Windows user-preferred UI language through a narrow Rust application command (Windows API, no general settings/filesystem access). Normalize language tags and select the first supported primary language, e.g. `de-DE` -> `de`; otherwise English. `navigator.languages` is only a development/browser fallback, not proof of Windows UI-language detection. Implementation must verify the native API binding against existing Windows dependencies. Offer a small 'Use system language' reset action that clears the override and detects again.
- Use `Intl.NumberFormat(selectedLocale)` for read-only quantities, counts, ranges and original-value tooltips; replace ambient `toLocaleString()` calls. Keep numeric command payloads, comparisons, validation limits and save integers unchanged. Editable integer fields retain ungrouped ASCII digits with a translated hint; do not feed localized separators back through `Number()`. Changing language must not reparse a draft or send an edit command. Hex, IDs and binary version fields keep invariant formatting.

## UI language, encoding and game names are separate

Current `lib/legacyText.ts` decodes raw text using strict UTF-8 with CP1251 fallback or forced CP1251; `SaveOverviewPage` persists that choice as `h3forge.encoding`. This is an existing presentation-layer behavior, despite the broader architecture preference for Rust-owned decoding. H3F-024 does not relocate or change it: only its labels are translated. Changing interface language must never select an encoding, rewrite a decoded name, or translate/modify raw save bytes.

Original hero/town identities and artifact/creature/skill/spell names (also buildings/factions) need a separate ID- and game-version-based catalog strategy. Keep saved custom hero/town/map names verbatim. Prefer verified translations from the user's read-only installation when ID mapping, version and text encoding are established, or independently licensed catalogs with provenance and redistribution permission. Installation presence is not permission to bundle copyrighted text/artwork. No unverified archive names, guessed ID mappings or automatic machine translation of game data.

Safe fallback: verified localized catalog name -> existing verified English definition -> localized 'Unknown <kind> <id>'. Where only a decoded custom name exists, retain it. Catalog completeness/extraction is outside this issue and cannot block interface localization; search uses the actually displayed name and stable ID. Existing Rust parsing/archive access stays in Rust. No format specification change is needed because this plan changes no verified binary behavior.

## Structured Rust feedback and native dialogs

`AppErrorDto` currently supplies `{ code, message }`; several components stringify errors or display `reason`/`message` directly. Extend the app boundary with stable codes and typed parameters, e.g. `file-too-large` plus numeric `limit`, and an explicit code-to-key map in TypeScript. Keep Rust errors language-neutral and retain optional technical details for diagnostics. Distinguish actionable causes currently flattened into broad `open-failed`/edit errors; do not parse or translate English error strings with regexes. Apply the same descriptor pattern to capability reasons, status reasons, warnings and pending-edit labels. Core crates remain independent of Tauri and i18next.

Unknown/malformed codes or missing parameters produce a localized generic failure plus a diagnostic code/details view; never suppress the error or invent a recovery action. Legacy `{code,message}` remains readable during migration, with raw text confined to technical details. OS error text may remain in its original language there.

Translate app-owned native picker titles/filter labels in the frontend and pass a bounded, typed display-label DTO to the existing picker commands. Rust retains fixed filters, grant rules and path validation; labels confer no capabilities. Windows-owned buttons and OS error details may follow Windows language rather than the manual UI choice. Do not duplicate full translation catalogs in Rust.

## Incremental delivery within this issue

1. Add the resources/types, locale selection, Settings and focused resolver tests. Extract the shell and shared buttons/statuses first; untouched screens can remain English temporarily.
2. Migrate feature groups using the inventory, retaining component layout/state and edit handlers. Move text-producing helpers to descriptors or inject translated labels. Add structured Rust feedback and native dialog labels through existing typed wrappers. Deliver all four interface catalogs; do not create separate feature issues.
3. Audit remaining literals/DTO prose, check accessibility and expanded text, and complete the verification below. Mixed-language UI is acceptable only during migration, not for the inventoried app-owned interface at completion. Future features add keys when implemented; no dependency on H3F-019, 022/023 completion or hypothetical screens.

## Acceptance and minimal verification

- Focused table-driven tests cover all four locales, regional tag normalization, unsupported/empty OS results -> English, stored override precedence, reset, corrupt storage and storage failures. Relaunch proves manual selection persists.
- Catalog checks enforce expected keys, locale-specific plural forms and matching named parameters; test Russian counts 1/2/5/21 and representative 0/1/2 cases elsewhere. A removed/empty non-English key falls back to English; a missing English key fails validation. Unknown keys fail TypeScript checks, and malformed error descriptors yield the generic message without unresolved tokens.
- Check interpolation with numbers, Cyrillic, accented text and HTML-like names rendered as text. Exercise a structured Rust error, a read-only reason and unknown/legacy errors. Switching language refreshes existing feedback and accessible names.
- One compact four-language UI sweep covers Settings, navigation, a hero, town and player view, picker/filter, validation and safety status. Check German expansion, Russian glyphs, Spanish accents, focus order, clipped controls and tooltips at the supported minimum window size. Record review quality; unreviewed translations are not claimed as native-speaker verified.
- Assert locale changes preserve decoded CP1251/UTF-8 names, raw bytes, numeric values, current drafts and pending commands. Test one localized displayed quantity per language while its edit input/payload remains invariant. Existing save preservation tests remain authoritative; language changes themselves issue no save/edit calls.
- Run focused i18n tests plus frontend typecheck/build once; run targeted Rust DTO/locale/dialog tests for the boundary changes. One native Windows launch/open/edit/save/reopen smoke on a private copy confirms the pinned Tauri stack, first-launch detection and dialog labels. Do not repeat full game-load suites solely for UI text.

Historical planning verification (2026-11-10): documentation links, issue sequencing and diff reviewed; that planning session installed no packages and changed no application code/translations. Current verification and remaining manual gates are recorded above.
