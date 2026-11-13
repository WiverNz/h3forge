# H3F-029 - List Sorting & Filtering

Status: implemented with automated verification, 2026-11-13; native WebView2
smoke remains manual. Milestone: M5. Priority: P1.
Dependencies: 003, 013, 018, 023 (delivered player details only), 024, 027.

## Goal and baseline

Consistent, compact client-side sorting and filtering for Heroes, Towns,
Players and Creatures. Preserve search, edits and selection without changing
save data, parsing, write gates or application command contracts.

Inspected `HeroList.tsx`, `TownList.tsx`, `PlayersPanel.tsx`, `CreaturesPanel.tsx`,
`SaveOverviewPage.tsx` and `types/api.ts` under `web/src/`. Heroes has search/Owned
only; Towns has search/Hero inside (garrisoned hero); Creatures has search,
category and map-level filters. Before this issue, Creatures fell back to the first visible record when
selection was hidden; it now resolves the selected ID against all loaded groups.
[H3F-023](023-player-details-map-reveal.md) establishes color-based player
identity, Active/Unknown status and map-authored teams, not personal names or
defeat history. No new binary research is required.

Requested screenshots were unavailable in the planning conversation. This
proposal follows the existing compact UI; screenshot matching remains pending
their availability and is not claimed as verified.

## Delivered UI

- Above each list: existing search/quick filters, compact Sort selector,
  ascending/descending toggle and Filter button with an active-filter badge.
  Allow a short second row at narrow widths, not a permanent large panel.
- A small anchored Filter popover contains labeled multi-select color swatches
  with checkmarks, distinct Neutral/Unowned and Unknown options, compact facet
  checkboxes and a searchable creature-type picker. Never rely on color alone.
- Keep Owned only, Hero inside and Creatures category controls visible. Category
  uses one shared state, not duplicate controls; add All categories. Map level
  may move into the popover while retaining its functionality.
- Always show `filtered / total`, counting all loaded records in that section
  before search/category/other filters as total. Keep category subtotals if useful.
  Provide an empty-results message and Clear filters action.
- Clear filters clears search, quick filters and facets, including category to
  All, but retains sorting. Reset list restores defaults: original order for
  Heroes/Towns/Creatures, color ascending for Players, no search/facets, and
  the existing Wandering category default for Creatures.
- Retain selected ID and detail/editor when filtered out. Show a small
  "Selection hidden by filters" notice with Clear filters; never auto-select
  another record or inject hidden rows into results. Initial selection and
  actual record removal follow existing rules.
- Support keyboard operation, labeled direction state, Escape/outside dismissal
  and focus return. Reuse theme tokens and English/Russian/German/Spanish
  catalogs; verify long labels and both themes at 1280x720.

## List capabilities

| List | Sort keys | Filters retained or added |
|---|---|---|
| Heroes | Name, owner color, level, original order | Search; Owned only; one or multiple owner colors, Neutral/Unowned and Unknown |
| Towns | Name, owner color, faction, original order | Search by name/type; Hero inside; owner colors; factions including unknown |
| Players | Color, name, active status | Active/Unknown; verified map-authored team; distinguish no authored team from unknown/unverified team |
| Creatures | Creature name, quantity, coordinates, original order | Existing search/category; creature type; map level; owner where applicable |

## Ordering and data rules

- Derive views from current DTOs without mutating source arrays. Prefer small
  pure helpers and a shared toolbar/color picker; no generic table framework
  or new state-management dependency.
- Use `Intl.Collator` with the active UI locale for displayed names/factions,
  after existing legacy-text decoding. Recompute on locale/encoding changes.
  Player name means displayed localized identity, currently color; do not invent
  personal names. Levels, quantities and coordinates compare numerically.
- Color ascending follows player IDs: Red, Blue, Tan, Green, Orange, Purple,
  Teal, Pink, then explicit Neutral/Unowned. Unknown/invalid owners stay separate
  and last; the visual neutral fallback is not evidence of neutral ownership.
- Missing/unknown values stay last in both directions. Equal keys retain original
  source order in both directions, using source ordinal as the final tie-breaker
  independently of direction. Original order is unfiltered source order;
  descending for this key explicitly reverses that ordinal.
- Status uses evidenced Active/Unknown, with Unknown last. Empty armies, zero
  heroes or resources do not imply defeat. Team facets retain map provenance
  and never imply verified current alliances.
- Creature-type filters match any nonempty current army slot. For multi-stack
  garrisons, creature-name sort uses the first nonempty slot's displayed name;
  quantity sums current nonempty slots and is labeled total quantity. Verified
  empty armies have zero quantity and no name; unavailable current armies have
  unknown quantity/type. Never substitute initial H3M quantities. Coordinates
  compare `(z, y, x)` numerically, explained by a tooltip.
- Creature ownership uses evidenced DTO semantics. Distinguish known neutral,
  unknown and not applicable where supported; null alone is not proof of neutral.
  Unsupported object categories remain out of scope.
- Selected values within one facet combine with OR; facets, search and quick
  filters combine with AND. An empty facet selection means All. Owned only plus
  Neutral-only legitimately yields zero matches; do not silently alter filters.
- Controls must not remount editors, discard local input/pending edits or trigger
  edit commands. Refresh presentation from current edits while retaining IDs.
- Persistence: versioned per-list sort key/direction only. Search,
  filters and selected IDs remain session-only and reset for a newly opened save.
  Validate stored enums; invalid/unavailable storage falls back safely. Do not
  persist save data or paths. This avoids silently hiding records on next launch.

## Implementation phases

Small local epic tasks, not separate issue IDs. A-D are implemented; E passed
automated acceptance, with native checks below still manual:

1. **029-A: Shared behavior** - pure stable comparators/predicates, compact
   toolbar/popover/color selection, localization and sort preference storage.
2. **029-B: Heroes and Towns** (after A) - keys/facets, retained quick filters,
   counts, hidden-selection notice and edit-state preservation.
3. **029-C: Players** (after A) - color/display-name/status sorting, evidenced
   status/team filters, counts and retained resource/tent edits.
4. **029-D: Creatures** (after A) - category/type/level/owner composition,
   multi-stack semantics and ID-based detail selection independent of results.
5. **029-E: Acceptance** (after B-D) - focused frontend behavior tests, existing
   localization rendering checks, one frontend production build and native
   1280x720 smoke in both themes; record results and outstanding manual gates.

## Acceptance criteria

- [x] All specified keys support both directions; ties remain stable. English,
      Cyrillic, mixed-script and localized labels follow the active locale;
      locale/encoding changes preserve record identity.
- [x] Numeric values, duplicate names, missing fields, Neutral/Unowned and
      unknown owners follow the documented ordering and filter rules.
- [x] Search, sorting, multi-select facets and existing quick/category filters
      compose correctly. Counts, empty states, Clear filters and Reset list agree.
- [x] Players expose only evidenced status/team facts, with no invented defeat.
      Multi-stack garrisons, empty armies and unavailable quantities behave as specified.
- [x] Sorting/filtering retains selected IDs and editor nodes. Browser fixtures
      verify hidden player/creature draft retention and pending markers, with zero
      native writes. Existing edit/Revert/Discard/Save handlers remain unchanged.
- [ ] Native save-session smoke for Revert/Discard/Save and asynchronous edit refresh
      with real pending edits; synthetic fixtures do not establish native outcomes.
- [x] Browser layout at 1280x720 in all four languages/both themes: popovers stay
      in the viewport, lists retain space, no horizontal overflow; checked colors,
      focus entry/return and an exported screenshot were inspected.
- [ ] Native WebView2 keyboard/Escape/outside-click and desktop visual smoke.
- [x] Sort persistence round trips and remount restoration pass; corrupt/unavailable
      storage falls back safely. Filters are component-local and reset on unmount
      when opening another save (also when switching sections).
- [ ] Native application relaunch smoke for sort preference restoration.

## Verification and implementation notes

- `node --test web/src/lib/listView.test.ts web/src/lib/i18n.test.ts`: 11 passed.
  Covers four-locale collation, stable ties, missing values, numeric coordinates,
  owner distinctions, current/empty/unavailable armies, facet composition,
  ID selection, team provenance and valid/corrupt/unavailable preference storage.
- `npm.cmd --prefix web --cache .local/npm-cache run test:i18n-ui`: 2 passed,
  including four-language player controls and existing editor/dirty rendering.
- `npm.cmd --prefix web --cache .local/npm-cache run test:lists-ui`: 224 browser
  assertions passed, zero native write calls. The reusable Windows runner bundles
  synthetic DTO fixtures, uses installed Chrome (or Edge via `-Browser` on
  `web/scripts/test-list-controls.ps1`), and keeps profile/DOM/logs/screenshot in
  ignored `.local/`. No game data is opened. It tests combined controls, hidden
  selection/drafts, pending markers, counts/reset, scroll retention, focus and
  1280x720 layout across four languages/two themes. Fixture initialization was
  corrected to supply Tauri 3's `isTauri` flag before the successful run.
- Browser testing found native scroll anchoring moved the viewport on row reorder;
  `overflow-anchor: none` now keeps the same list scroll position while sorting.
  Filtering naturally clamps scroll when the remaining list is shorter.
- `npm.cmd --prefix web --cache .local/npm-cache run build`: TypeScript and Vite
  passed once after application changes. `git diff --check`: clean.
- Reusable code is limited to `ListControls.tsx` and small `listView.ts` helpers.
  Native HTML popovers escape list clipping, with focus management and labeled
  checkbox swatches. Only sort key/direction persists. No dependencies installed.
- Creature owner byte 255 maps to explicit neutral; null/invalid ownership stays
  Unknown. No not-applicable ownership state is invented from the current DTOs.
  Creatures retains loaded details during epoch refresh, preventing an unnecessary
  editor unmount; selected IDs reconcile only when the loaded record disappears.

Native checks above remain manual, including actual application relaunch and
Revert/Discard/Save with real pending edits. Browser fixtures are not native
WebView2 or in-game verification. Screenshots supplied by the user remain
unavailable; the generated browser screenshot was visually reviewed instead.
No Rust, command contracts, savegame bytes, format specifications, source game
assets or other issue implementations changed. No commit, push or release.
