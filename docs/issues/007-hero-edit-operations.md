# H3F-007 - Plan validated attribute and spell patches for HotA 44/11

Status: completed (implementation evidence 2026-11-08). Milestone: M3. Priority: P0.

## Goal

Make controlled in-memory edits whose binary impact is explicit and testable.

## Scope

Add exact-layout operation gates, immutable source payload, revisions and equal-length preimage-checked BytePatch planning. Support a learned-spell toggle plus conservative primary values and mana. Add level/experience with explicit synchronization choice and version limits. Port spellbook/equipment-granted availability rules; preserve unrelated map-suppressed flags. Do not normalize complete hero sections, create spellbooks implicitly, or enable writes on unresolved artifact/stat coupling. Document the evidence for minor-11 offsets and any narrower per-record restrictions.

## Relevant files or components

crates/h3forge-save/src/{edit,patch,definitions}.rs; core domain capability model; app session/dto.

## Dependencies

H3F-003 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [x] A known owned spellbook-bearing sample record supports a deliberate spell change, with only required learned/available bytes patched. - The research candidate at span 4,727,078 (owner 4, spellbook carried, learned `[3,15,17,20,25,33,35,37,42,43,44,47,64]`) learns Air Shield and unlearns Visions: exactly 5 patches (attack + learned/available for both spells), verified byte-for-byte on a private copy (`tests/sample_edit_roundtrip.rs`).
- [x] Invalid values, stale revisions, overlapping patches and preimage mismatches are rejected in Rust. - `EditError::InvalidValue` (attributes outside HotA game ranges 0–99/1–99, level outside 1–74, spell ID ≥ 70), `OverlappingPatches`, `PreimageMismatch`, `PatchOutOfBounds` covered in `patch.rs`/`edit.rs` unit tests; apply is all-or-nothing before the first write.
- [x] No-op operations produce no patches; unrelated payload bytes and unknown IDs remain identical. - `no_op_edits_plan_no_patches`; the sample round trip diffs exactly the planned offsets and nothing else.
- [x] Recognized-but-unsupported formats and ambiguous records expose no edit capability; conservative primary edits account for proven equipment bonuses. - edits gate on `hero_layout_supported()` (HotA) plus High confidence; primary edits accept only the h3sed HotA in-game ranges (`version/hota.py` `PRIMARY_ATTRIBUTE_GAME_RANGES`), so raw overflow/cap semantics (values ≥ 100, handicap from 231) are preserved read-only.

## Implementation evidence (2026-11-08)

- New core modules: `patch.rs` (`BytePatch` with expected-old-bytes preimage, sorted-overlap detection, all-or-nothing `apply_patches`, `compress_gzip` producing a standards-compliant member: MTIME 0 header, DEFLATE level 6, correct CRC-32/ISIZE) and `edit.rs` (`HeroEdit` semantic operations, `SaveDocument::plan_hero_edits`, `serialize_with_patches`). Typed `EditError`; no panics on file-derived data.
- Spell coupling ported from h3sed `hero/spells.py:serialize`: learn sets the learned flag and, when the hero carries a spellbook, the available flag (`available = has_spellbook && in_book`); unlearn clears both under a spellbook. Without a spellbook the available array is left untouched (artifact grants not decoded yet - documented limitation; a scroll-granted spell may need re-equipping after unlearn). Map-banned spell preservation is intentionally narrower than h3sed's bannable-spell heuristic: unchanged flags are never normalized.
- Level/experience stay independent bytes (offsets 49 / 39); `EXPERIENCE_LEVEL_MIN` backs UI explanation only. Level validated 1–74 per h3sed `version/hota.py` `DATA_RANGES` (base game allows 75).
- Session layer stores absolute desired values per hero/field, replans against the immutable source payload on every change, drops no-op overrides automatically.

## Minimal verification

Focused tests for one spell addition/removal, artifact-granted retention, no spellbook, primary/mana bounds and stale preimages. On a private copy, compare the payload against the expected patch union. This issue does not write a save.

All covered: `cargo test -p h3forge-save` (46 lib + `sample_edit_roundtrip` with `H3FORGE_FIXTURE_ROOT=games`). No save file is written by this issue; writing is H3F-008.
