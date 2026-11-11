# H3F-011 - Edit backpack artifacts and scrolls

Status: completed (in-game loading remains a separate manual acceptance gate). Milestone: M4. Priority: P1.

## Goal

Provide a useful backpack editor before the more coupled equipment editor.

## Scope

Port 64-slot backpack IDs and eight-byte scroll encoding. Add add/remove/replace actions with version-aware choices and original icons when mapped. Preserve unchanged sentinel words and unknown artifacts; do not automatically compact or sort the binary inventory. Equipment remains unchanged.

## Relevant files or components

save artifact definitions/edit/parse; resource artifact catalog; web backpack panel.

## Dependencies

H3F-008 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [x] Supported artifact and scroll entries round-trip within the backpack's fixed capacity.
- [x] Unknown entries and untouched blank encodings survive exactly.
- [x] Invalid/version-incompatible items are rejected; no equipment or reservation bytes change.
- [x] The UI explains scroll identity and full-backpack errors.

## Minimal verification

One artifact and one scroll replacement test, full-capacity rejection, and unchanged-unknown/blank preservation. Save/reopen one temporary sample copy.


## Implementation and verification (2026-11-09)

- Continued the existing uncommitted core/DTO draft; completed request conversion, pending state, compact occupied-slot UI, searchable artifact/scroll selection and per-slot dirty indicators. No Apply step; Save/Save As retain the existing safety pipeline. Revert/Discard restore original bytes; hero/tab changes retain session edits.
- All 158 artifact ID/name literals were compared explicitly with H3SED `metadata.py` and `version/hota.py` binary IDS (not display indices). Writable conservative set: The Grail (2), ordinary base artifacts 7–126; scroll ID 1 plus corrected spell IDs 0–69. IDs 0, 3–6, expansion/combined items and unknown IDs cannot be written. Expansion names are displayed but availability is not inferred from H3SED's broad HotA support.
- Fixed capacity: 64 slots, eight bytes each at record offset 534. Only explicitly changed slots are patched. No sorting, compaction, count normalization, equipment, reservation, attribute or spell-array edits.
- First word FF means empty; preserve its trailing word when untouched. Zero-ID entries remain ambiguous/read-only. Ordinary known entries accept observed zero/FF trailing words; other representations and malformed scrolls remain unknown/read-only. Removal writes FF FF FF FF 00 00 00 00 (upstream executable code, correcting its reversed comment). Scrolls write little-endian u32(1) + u32(spell ID).
- No verified artifact resource mapping exists; neutral placeholders are used. No resource milestone expansion.
- Focused core tests: add/replace/remove, scroll add/replace, all eight scroll bytes, unknown/sentinel preservation, exact patch offsets, full-capacity boundary rejection and unsupported values. Full UI disables Add and explains removal is required.
- Actual temporary SaAndVla.GM2 round trip via application Save As: slots 2 and 3 (zero-based), patches at hero-relative 550 and 558, eight bytes each. Actual differing relative bytes: 550–562 (13 bytes). Reopened correctly; every byte outside those two slots and equipment unchanged; input copy unchanged. Pending state, DTO conversion, hero navigation, Revert, Discard and clean state after save verified in the same test.
- Final verification: targeted Rust tests, `cargo check --workspace`, frontend `npm run build`, formatting and diff checks. In-game loading remains unperformed/manual.

Next issue: H3F-012; not started. Equipment and combined-artifact behavior remain out of scope.
