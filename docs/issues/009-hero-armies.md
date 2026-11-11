# H3F-009 - Edit hero armies and complete the MVP

Status: completed (2026-11-09; automated verification complete, actual in-game compatibility unverified). Milestone: M4. Priority: P0.

## Goal

Edit the seven hero army slots safely and clearly.

## Scope

Add version-aware creature choices and counts, retain unrecognized stacks read-only, preserve untouched empty encodings, and patch only changed type/count words. Add slot UI with generic fallback images and reuse supported creature icon mappings from H3F-006 when available. Validate count bounds and any proven last-stack rule explicitly; do not assume a storage maximum is a sensible default count.

## Relevant files or components

save edit/definitions/army parsing; resource catalog; app DTOs; web army panel.

## Dependencies

H3F-008 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [x] Known sample hero stacks can change creature/count and clear a supported slot, then save/reopen correctly.
- [x] Counts outside validated range and unsupported creature IDs are rejected in Rust.
- [x] Untouched empty/unknown slots are byte-identical; creature pictures never substitute an unrelated ID.
- [x] M3 attributes/spells and this army workflow together satisfy the MVP scope.

## Minimal verification

Focused tests for one changed slot, both observed empty encodings, invalid counts and an unknown creature. Save/reopen a temporary sample copy and inspect the changed army in game when validating new army semantics.


## Implementation and evidence

- `HeroEdit::SetArmySlot` accepts zero-based slot 0–6, an optional creature ID and a u32 quantity. It patches only changed 4-byte ID/count words, with expected original bytes. All army writes are gated to high-confidence hero records in exact HotA **44/11**; other versions and unknown stacks stay read-only.
- `creatures.rs` ports 189 choices from h3sed 4.1 `metadata.py` + `version/hota.py` (existing MIT notice retained). This includes Cove, Factory and the minor-10 Bulwark additions present in 44/11; creature definitions do not imply support for other save layouts. War-machine/reserved IDs absent from the creature choices are rejected.
- Reference `common/army.py:parse/serialize` treats zero count or FF ID as empty, preserves original empty words when still empty, and clears an occupied stack to FF ID / zero count. These rules are now Rust-owned. Unknown type words remain read-only, even when their count is zero. Reverting an initially empty slot drops the pending operation and restores its exact original encoding.
- Occupied count range is 1–4,294,967,295, matching the reference's `DATA_RANGES["army.count"]`; typed deserialization rejects negatives, fractions and overflow. A newly added stack defaults to **1**, never the storage maximum. The reference permits 0–7 stacks, so no unverified last-stack restriction is invented. Storage-boundary tests do not establish game behavior at extreme counts.
- The verified garrison restriction in `common/army.py:is_readonly` blocks a **town** army when a garrison hero owns it, not that hero's own army. H3F-009 edits only the hero record; no town record, town relationship or garrison linkage is parsed or written.
- The compact Army tab has seven creature/quantity editors. Apply stages each complete stack atomically; choosing Empty removes it. Known names come from Rust. Applied changes show dirty markers and the existing top-toolbar indicator; hero revert and global Discard Changes use the same session operations. Unsupported slots show read-only values. No DEF graphics dependency.
- The native dialog, backup, source-change check, temporary output verification and no-clobber Save As code are unchanged. All pending army operations are revalidated when saving.

## Verification results

- `cargo test -p h3forge-save --lib army_`: **2 passed**, covering all seven slots, Pikeman ID 0, both original empty forms including FF count, no-op/clear/replace, boundary counts, unknown stacks, invalid IDs/slot/count, exact-version gating, low-confidence rejection and independent changed-byte bounds.
- `H3FORGE_FIXTURE_ROOT=D:\Projects\h3forge\games cargo test -p h3forge-desktop --test edit_flow army_ -- --nocapture`: **2 passed**, including request overflow/negative/fraction rejection and one real SaAndVla round trip through the safe session pipeline. The test explicitly ran on an ignored copy (215 heroes), replaced slots 1/7, cleared slot 2, combined army edits with attributes/spells, verified exact full-payload equality to planned patches, confirmed source/backup identity and clean state, and exercised invalid-request retention plus revert/hero-discard/global-discard.
- Initial desktop test compilation exposed a missing direct test dependency; the already locked `serde_json` version is now a pinned dev dependency. The corrected focused run passed; no dependency upgrades.
- `cargo check --workspace`: passed (one final workspace check).
- `npm.cmd --prefix web --cache .local/npm-cache run build`: TypeScript and Vite passed (one final frontend build).
- `cargo fmt --all` and final `git diff --check`: clean.
- **Not verified:** actual matching-game load, gameplay at extreme quantities, native UI interaction. Parser/session round trips are not in-game compatibility evidence. No other issue started.
