# H3F-028 - Keymaster Tents and Player Visit Status

Status: implemented and sample-verified; native HotA loading/access validation
remains manual. Milestone: M4. Priority: P1. Dependencies: 006, 008, 018, 021.

## Delivered

- Compact Players / Exploration / Keymaster Tents section, with original icons,
  eight Visited / Not visited controls and selected-player identity. Clear visits,
  immediate pending edits, dirty markers, player Revert, global Discard, Save and
  Save As are integrated. Labels are present in English, Russian, German, Spanish.
- All eight players and colors: Light Blue, Green, Red, Dark Blue, Brown, Purple,
  White, Black. Tent order differs from player color order. Unknown layouts show
  an unavailable explanation and have no writable backend operation.
- Typed `TentEdit` operations, no frontend offsets. The core composes all player
  operations before generating one expected-old-byte patch per changed color.
  Other players, colors and all unrelated bytes survive unchanged.
- Original `H3sprite.lod/AVXkey{0..7}0.def` artwork verified against all eight
  current-save templates, H3M subtype mappings, frame names and visual inspection.
  Rust bounded format-3 DEF decoding and source-identity PNG caching are reused.
  Special map shadow/selection indices are omitted in the compact icon; original
  sprite colors remain. No copyrighted assets were added to source or packages.

## Binary evidence

Full offsets, hashes and all replacement/inserted bytes are in
[SAVEGAME_FORMAT.md](../formats/SAVEGAME_FORMAT.md#h3f-028---keymaster-tent-visits-hota-4411).
Artwork decoding is in
[GAME_RESOURCES_FORMAT.md](../formats/GAME_RESOURCES_FORMAT.md#h3f-028---original-keymaster-tent-icons).

The mask table starts at last verified hero end +12+430+109. Exact 44/11,
215 complete ordered verified heroes, owner-table agreement and bounded globals
are required. The supplied pair's eight masks are at 4552402..4552409.
Pink visiting Red changes only the access mask at **4552404: 01 -> 81**.
The current map confirms class 10/subtype 2 at (142,131,1), sprite AVXkey20.def.
No separate tent count or related access flag changes in the pair.

Native decompressed sizes are 5,012,796 / 5,012,847. Aligned differences:
24 filename bytes, 11 source/destination tile bytes, 9 hero position/object-under/
facing/movement bytes, 1 tent-mask byte, 2 replay-counter bytes, plus 51 inserted
replay bytes. Total **47 changed bytes + 51 inserted**, with an identical
90,987-byte aligned suffix. A naive unaligned comparison counts 61,300 changed
bytes in 15,234 runs. Fog and all remaining payload state are unchanged.

An editor Pink/Red visit changes **exactly one byte**, leaves payload length
5,012,796, and preserves the other 5,012,795 bytes, including the replay history.
All 64 player/color cases independently toggle exactly their mask byte, preserve
all other bits, save/reopen and reverse to the original payload.

## Verification

- `H3FORGE_FIXTURE_ROOT=D:\Projects\h3forge\games cargo test -p h3forge-save --test sample_keymaster -- --nocapture`:
  1 passed, no skip. Native before/after masks, 64 independent toggle/reopen/reverse
  cases, combined same-byte player edits, no-op, duplicate/range/version/corrupt
  owner-table rejection and untouched originals.
- `cargo test -p h3forge-save --lib keymaster -- --nocapture` with the same root:
  1 passed, no skip. Exact tent coordinates and all eight current template names.
- `cargo test -p h3forge-desktop --test keymaster_flow -- --nocapture` with the same
  root: 1 passed, no skip. Pending state, change-back, Revert, Discard, multiple
  players sharing one byte, simultaneous resources, Save As, direct Save,
  source-change rejection and all eight cached original icons.
- `H3FORGE_FIXTURE_ROOT=D:\Projects\h3forge\Data cargo test -p h3forge-resources --test def_portraits -- --nocapture`:
  5 passed, no skip. New block offsets/literal/run/truncation checks and existing
  interface/creature regression. Initial asset-root setup used games instead of
  Data; corrected for this asset test before the successful verification.
- `cargo test -p h3forge-map --test synthetic -- --nocapture`: 8 passed, including
  bounded tent metadata and existing parser regressions.
- `npm.cmd --prefix web --cache .local/npm-cache run test:i18n-ui`: 2 passed,
  including eight controls and translated labels in four-language rendering.
- `cargo check --workspace`: passed once.
- `npm.cmd --prefix web --cache .local/npm-cache run build`: TypeScript and Vite
  production build passed once. `git diff --check`: clean.
- Original pair SHA-256 values rechecked after tests and unchanged. All generated
  save copies/images are under ignored `.local/fixtures/`. No originals modified.

## Limitations and manual acceptance

Only Pink/Red has a native before/after action pair. Remaining player/color
support is based on the shared bitmask layout, enum, current-save templates and
independent-bit tests. Other versions/roster shapes are read-only. No additional
pair is needed for this verified structure.

The editor changes player-wide key access. It does not remove Border Guards,
restore removed guards when clearing access, simulate movement/fog/replay, or
alter allies' bits. The exact semantics of some incidental replay subevents
remain unresolved and are deliberately not written. The existing local H3M
reference has different tent positions from the new pair and corroborates only
subtype/artwork mappings, never current save offsets.

Still manual: load an edited copy in matching native HotA, check Pink's Red
Tent visit text and matching Border Guard/Gate access, and verify native window
layout at 1280x720. Automated tests do not establish these in-game outcomes.
No commit, push, release, or other issue work was performed.
