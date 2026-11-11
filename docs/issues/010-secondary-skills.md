# H3F-010 - Edit secondary skills and mastery with version-aware ordering

Status: completed (2026-11-09). Milestone: M4. Priority: P1.

## Goal

Add, remove and reorder supported secondary skills without damaging legacy bytes.

## Scope

Use explicit ID/mastery/order/count definitions, including HotA order offset 1092. Resolve usable-skill-count versus 30-definition/29-limit ambiguity for the supported minor before enabling edge counts. Add mastery controls and ordered slots. Reject duplicates and retain unrecognized or out-of-profile data. Do not overwrite HotA's legacy slot region.

## Relevant files or components

save definitions/edit/parse; web skills panel; app hero DTO.

## Dependencies

H3F-008 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [x] Supported skills and mastery values survive save/reopen in chosen order.
- [x] Count/order/mastery fields stay consistent; duplicate skills and unsupported extremes are rejected.
- [x] Changes do not touch obsolete order bytes or unrelated attributes.
- [x] Unsupported layouts display skills read-only with an explanation.

## Minimal verification

One synthetic add/remove/reorder test plus a private sample-copy diff. Include invalid mastery/duplicate and legacy-region preservation assertions.


## Implementation evidence (2026-11-09)

- **ID table verified (H3F-006 lesson applied):** the 30-entry `SKILL_NAMES` table was checked against h3sed `metadata.IDS` skill entries (Pathfinding 0x00 .. First Aid 0x1B, plus `version/hota.py`: Interference 0x1C, Runes 0x1D) - the existing table is already true ID order; h3sed's alphabetical display list was NOT used. Count/mastery/order semantics ported from `hero/skills.py` parse/serialize and `version/hota.py` (`skills_slot: 1092`).
- **29/30 ambiguity resolved:** editing is gated to the exact 44/11 1122-byte record (30 order bytes at 1092); the 1121-byte 29-order-byte layout stays read-only. Capacity is 28 (h3sed DATA_RANGES "skills", locator cap 0x1C) even though 30 skill IDs exist.
- **Operation:** `HeroEdit::SetSkills { skills: Vec<(id, mastery)> }` - one absolute ordered list per hero (session `FieldKey::Skills`), so add/remove/mastery/reorder all replay deterministically against the immutable base payload. `plan_skills` writes only: count low byte (43), mastery bytes (182+id), HotA order bytes (1092+id, dense 1-based); removed skills are zeroed; unrecognized entries (zero slot/mastery or slot beyond base count) and the legacy region 212..238 are byte-preserved. Rejects duplicates, unknown IDs, mastery outside 1-3, lists over 28.
- **Frontend:** new `SkillsEditor` on the Skills tab - mastery dropdown per skill, up/down reorder, remove, and an alphabetical add-select that carries binary IDs; immediate pending edits (no Apply), dirty dot via "Skills", Revert/Discard unchanged. Unsupported layouts render the read-only chip list with the reason.
- **Verification:** 6 synthetic edit.rs tests (add/remove/mastery/reorder offsets, duplicate/invalid/capacity rejection, legacy+unrecognized preservation) and one private sample-copy round trip (reorder + Wisdom→Expert + add Logistics = 7 patches, reopen shows `[(8,1,1),(24,2,2),(7,3,3),(2,1,4)]`, diff confined to count/mastery/order bytes, source untouched). Full workspace suites pass; clippy/fmt/build clean. In-game load remains the separate manual gate.


## Skills artwork follow-up (2026-11-09)

- Added compact 24px original game icons in current skill rows (including read-only views) and Add options. Source: H3sprite.lod/secsk32.def; all 28 base skill IDs resolve, HotA-only IDs retain placeholders. Mapping evidence and limits are recorded in GAME_RESOURCES_FORMAT.md.
- Reused bounded DEF decoding, PNG encoding, archive-identity caching, typed resource command and frontend shared-request/image patterns. No save/edit pipeline changes.
- Native Add select replaced by a compact searchable image listbox, with arrow/Enter/Escape/Tab support, outside-click dismissal and creature-picker popup placement. Mastery, add/remove, reorder and pending edit/save/revert/discard behavior remain unchanged.
- Verification: focused mapping/fallback unit test and real-archive 28-icon decode/cache test passed; targeted desktop cargo check, formatting/diff checks and one frontend build passed.
- H3F-012 and other issues were not started.

### Skill icon IPC correction
The first UI implementation omitted get_skill_icon from build.rs AppManifest and the main-window capability, so desktop IPC requests were blocked despite successful resource tests. Added both registrations and a focused wiring regression test. Placeholder tooltips now expose request errors, and rejected requests are evicted from the frontend cache. Targeted permission test and frontend build passed; desktop executable rebuilt with custom-protocol assets. Live desktop visual verification remains manual.


### Owner accents and direct skill replacement (2026-11-09)

Completed requested UI follow-up: owner IDs 0–7 map to Red, Blue, Tan,
Green, Orange, Purple, Teal and Pink; neutral/invalid ownership uses gray.
Hero rows use inset left stripes and selected rows use a subtle matching
border/tint; detail headers have the same stripe. Existing gold dirty
indicators and compact row geometry are preserved.

Skill names now open the existing searchable image selector. Replacement
preserves mastery and position and sends one absolute SetSkills list, never
an intermediate remove/add. Options exclude other learned IDs; the helper
rejects duplicates/unsupported IDs and Rust's existing limits/compatibility
validation remains authoritative. Add remains independent; Revert/Discard
and save semantics are unchanged.

Verification: three focused frontend checks passed (owner mapping,
replacement mastery/order preservation, invalid/duplicate rejection); the
existing Rust skill icon mapping/fallback test passed for IDs 28/29. One
successful frontend production build and diff check passed (initial
TypeScript check required adding the missing assert.throws ambient test
declaration). No Rust production code changed, so no extra Rust check.
Live visual verification remains manual. H3F-012/014 were not started.


### Mastery-specific artwork follow-up (2026-11-09)

Completed: existing/read-only skill rows and Add/Replace options now request
artwork by skill ID and current mastery. Changing mastery updates the icon
from pending state without Apply/Save. Reused the Rust DEF loader, PNG encoder
and installation-aware caches, with mastery added to both cache keys.
Verified all 84 local base-skill variants and actual directory labels;
unknown skills/invalid mastery retain placeholders. Interference/Runes remain
blocked as documented above. Focused mapping/fallback unit test, real-archive
variant/cache test, targeted desktop check and one frontend build passed.
Editing, dirty state and persistence are unchanged; no other issue started.
