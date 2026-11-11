# H3F-003 - Display actual heroes in a read-only save overview

Status: completed (2026-11-08). Milestone: M1. Priority: P0.

## Goal

Select and inspect real heroes from the sample in the desktop UI.

## What was implemented

- **Core display tables** (`crates/h3forge-save/src/definitions.rs`, `names` module): h3sed skill names (28 base + HotA Interference/Runes), mastery names, 69 spell names (flag slot 69 documented as unknown), player faction labels and the 19 equipment slot labels, all with MIT provenance. Read-only labels; unknown IDs stay unknown.
- **Spellbook presence** (`domain.rs`): `HeroView.has_spellbook`, ported from h3sed `hero/stats.py` `parse_special` semantics (word at the spellbook equipment slot, offset 518 / slot 17, blank iff four 0xFF bytes). Detection only, not castability.
- **App session layer** (`apps/h3forge-desktop/src-tauri/src/session.rs`): one open save behind `AppState`; the Rust-owned rfd picker issues a single-use opaque grant (`choose_save`) so the frontend can never name an arbitrary path; `open_granted_save` does a metadata size check, bounded read and `SaveDocument::open_with_limits`. No write path exists anywhere.
- **Commands/DTOs** (`commands.rs`, `dto.rs`): `get_app_info`, `choose_save`, `open_granted_save`, `list_heroes`, `get_hero` with structured `{code, message}` errors (`grant-not-found`, `open-failed`, `no-open-save`, `hero-not-found`, …). DTOs are read-only projections: raw name bytes travel with the lossy decode so the frontend can re-decode; notes carry human explanations for mismatch, duplicate names and rejections; equipment/inventory stay raw with labeled slots.
- **Frontend** (`web/src`): `SaveOverviewPage` with map/version banner, diagnostics, searchable hero list (search runs against the user-selected decoding; "owned only" filter; duplicate/candidate flags), read-only detail (attributes, spells with names, skills, army, raw equipment with explanations, record span), and a selectable legacy decoding control (Auto / Windows-1251 via `TextDecoder` - the frontend concern deferred by H3F-002). The M0 placeholder page was superseded; the Tauri foundation itself is unchanged except for the registered commands, `build.rs` command manifest and capability permissions.
- h3sed MIT attribution extended in `THIRD_PARTY_NOTICES.md`.

## Acceptance criteria

- [x] Locator results explain/reproduce the 215-candidate research baseline; discrepancies are investigated, not hidden by hardcoding the count. - Reproduced by H3F-002's port and re-asserted end-to-end through the session layer: sample opens with 215 records, 215 confirmed, first span 4,657,602; the count is a locator outcome, not an asserted constant in the app.
- [x] Renamed, duplicate and Cyrillic names select the correct span; owned heroes are distinguishable from neutral/uncertain records. - Synthetic duplicate-name fixture selects distinct spans by opaque ID; owner badges distinguish player/neutral; CP1251 names re-decode from the preserved raw bytes.
- [x] An owned spellbook-bearing record is inspectable with its actual values; unsupported fields have explanations. - The owned research candidate (span 4,727,078, Orange, spellbook carried, 13 learned spells) shows all parsed values; equipment/inventory render raw with explicit milestone explanations.
- [x] Malformed candidates cannot become editable records; no disk-writing API exists yet. - Low-confidence candidates are listed but flagged with rejection notes; there is no edit/write command or UI control.

## Verification (2026-11-08)

- `cargo check --workspace --locked --offline` - pass (lockfile changed only by the new workspace `h3forge-save` path edge).
- `cargo clippy -p h3forge-save -p h3forge-desktop --all-targets` - 0 warnings.
- `cargo test --workspace --offline` - green, stable across repeated runs: 33 core unit + 5 core e2e + 5 app overview integration + opt-in sample tests on both layers.
- `H3FORGE_FIXTURE_ROOT=<private sample dir>`: app-layer sample test opens the sample through grant → list → detail and re-asserts the research baseline (215/215, owner mix reproduced indirectly via the Orange candidate, learned spells `[3,15,17,20,25,33,35,37,42,43,44,47,64]`, skills Wisdom/Intelligence/Mysticism, ≥100 non-UTF-8 CP1251 names).
- `npm run check` (tsc) and `npm run build` (vite) - pass.
- Not performed: an interactive GUI click-through (consistent with H3F-001: the user performs the visual launch check). No in-game load check is claimed.

## Remaining limitations

- Hero detection confidence is structural, not game-verified; "owned" means the owner byte names a player, not that the hero is active or recruitable.
- Spell ID 69 has no researched name; equipment/inventory artifact IDs and creature IDs remain raw by scope.
- Level/experience mismatches (8 on the sample) are explained, not reconciled.
- Savegame writing, portraits, spells UI editing: later milestones.

## Dependencies

H3F-002 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.
