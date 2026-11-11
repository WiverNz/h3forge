# H3F-018 - Edit player resources and gold

Status: completed (sample-verified; native UI and in-game loading remain manual acceptance gates). Milestone: M4. Priority: P1.

## Goal

Let the user change the seven per-player resources (gold, wood, ore, mercury, sulfur, crystal, gems) in supported saves.

## Scope

Research and verify the player-state block layout in the HotA 44/11 payload (h3svg specification `Player` struct as the primary reference; the vlucas homm3-savegame-editor project documents a writable player-resources layout for classic saves - check its license before reusing offsets, and re-verify every offset against the sample before enabling writes). Implement read-only player resource display first, then writes through the existing `BytePatch` pipeline with per-field gates. Distinguish player index from owner color; handle neutral/unused player slots explicitly.

## Relevant files or components

crates/h3forge-save/src (player definitions + locate/parse); session/dto commands; frontend player panel.

## Dependencies

H3F-008 (the write pipeline this issue plugs into). See the [issue index](README.md) and [architecture](../ARCHITECTURE.md).

## Acceptance criteria

- [x] Resource offsets verified against the sample (multiple players, values that unambiguously identify the field, e.g. distinctive gold amounts).
- [x] Read-only player resource view lands before any write gate opens.
- [x] Writes change only the targeted u32 fields; the existing no-op/overlap/preimage rules apply unchanged.
- [x] Unknown or ambiguous player blocks expose no edit capability.

## Minimal verification

Locate the resource block in the sample, cross-check against at least one independent reference implementation, then focused copy-based round-trip tests per field. No new abstractions beyond the existing edit pipeline.

## Implementation and verification (2026-11-10)

- Read-only `PlayerView` parsing identifies eight 149-byte records immediately
  before the complete town table. The sample's active colors are Red (5
  adventuring heroes), Orange (5 heroes, 19 towns), Teal (4 heroes, 14 towns),
  Pink (5 heroes). Blue, Tan, Green and Purple have no listed heroes/towns.
  They stay visible as inactive/unused and cannot be edited. Eliminated vs
  never-used, and human vs AI, are not inferred.
- Resource words start at record +98, ordered Wood, Mercury, Ore, Sulfur,
  Crystal, Gems, Gold. Sample table base: 4,634,994; stride: 149. Each word
  is four bytes, signed little-endian. Edits accept 0..2147483647. All
  addresses and starting values are in [format section E](../formats/SAVEGAME_FORMAT.md#e-player-data-h3f-018).
- Typed `PlayerEdit::SetResource` operations use checked BytePatch preimages,
  no-op suppression and duplicate/overlap rejection. Rust resolves offsets.
  Exact 44/11 plus matching hero/town ownership links is required. Negative
  stored resources and unsupported/ambiguous records remain read-only.
- Players navigation, color-coded list, active/inactive labels, compact
  resource fields, validation, dirty markers and per-player Revert are
  connected to pending session state. Valid input queues immediately with
  no Apply button. Player/tab switches retain pending changes. Save controls
  wait for queued player commands; Save/Save As and global Discard use the
  existing verified persistence path.
- Seven original resource icons resolve from H3sprite.lod/RESOURCE.def;
  embedded frame names and visual inspection verify all mappings. Existing
  Rust DEF/PNG support and installation-aware caching are reused. Missing
  or unrecognized artwork uses placeholders. No copyrighted assets bundled.
- Independent evidence: h3svg Player specification, Player.h implementation
  and ResourceType enum, cross-checked with sample owner relationships and
  distinctive gold values (Red 8,721,404; Orange 552,235). Correction: current
  vlucas source/tree does not contain the previously claimed player-resource
  implementation; no license file found and no code reused from it.

Focused checks run with absolute fixture roots (no fixture skips):

- `cargo test -p h3forge-save --test sample_players -- --nocapture`: passes.
  28 independent field/player round trips; two-player Save/reopen; exact
  offsets/values; no-op, negative/overflow, unknown ID, inactive, ambiguous
  ownership, unsupported version, duplicate patches and stale preimages.
- Two-player regression: Orange Gold = 2,147,483,647 and Teal Wood = 0.
  Changed payload bytes exactly `4635712, 4635713, 4635714, 4635715,
  4635986, 4635987`. All other 5,271,288 bytes, including inactive records,
  remain unchanged. All test output is under ignored `.local/fixtures/`.
- `cargo test -p h3forge-desktop --test player_flow -- --nocapture`: passes.
  Pending edits survive reads; validation failure preserves pending values;
  Revert and Discard restore the base; changing back removes pending edits;
  Save As/direct Save reopen and rebase; backups match exact prior bytes;
  changed source is rejected with pending edits retained.
- `cargo test -p h3forge-resources --test resource_icons -- --nocapture`:
  passes, 7/7 PNGs, cache equality and unknown-ID placeholders.
- `cargo check --workspace`: passes.
- `npm.cmd --prefix web --cache .local/npm-cache run build`: passes
  (TypeScript check and Vite production build).

Limitations: this locator requires eight ordered color slots and a complete
verified town table of 1-72 towns. It deliberately does not scan guessed
resource patterns in unsupported saves. Extreme supported values are storage
bounds, not proven game arithmetic limits. Actual in-game loading and native
window interaction remain separate manual acceptance gates.

Next issue: **H3F-019**. Not started.

### Desktop permission correction

The first native open exposed missing Tauri command permissions, which the
plain session tests and frontend build did not exercise. Registered all four
new commands in `build.rs` and granted their individual permissions to the
local main window: list players, edit resources, discard player edits and
load resource icons. No wildcard permission is granted. Rebuild/restart the
desktop process to load the corrected capability manifest.

The native follow-up also exposed the navigation button still carrying its
old disabled/planned markup despite the panel being wired. Replaced it with
an enabled button selecting the Players section. Rebuilt frontend and desktop.
