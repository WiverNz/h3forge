# H3F-008 - Complete the sample spell-edit workflow with backups and safe saving

Status: implemented (2026-11-08); hero attributes/mana/known spells now verified in HotA by user report recorded 2026-11-14. Separate persistence/recovery scenarios remain automated-only. Milestone: M3. Priority: P0.

## Goal

Produce and reload a valid modified sample through the desktop editor.

## Scope

Connect hero attribute/spell controls to semantic operations and implement dirty summaries, reset/discard and save errors. Add native Save As destination grants, automatic verified backups, source/destination conflict checks, same-directory temporary output, flush/reparse checks, no-clobber new-file commit and reviewed Windows replacement for overwrite. Keep edits on failure and guard reference paths including aliases. Asset issues H3F-005/006 enhance the screen but do not block the text-based editing path.

## Relevant files or components

apps/h3forge-desktop/src-tauri/src/{persistence,session,commands,dto}.rs; web editor/save controls; focused persistence tests. (Persistence lives in `session.rs`: `save_save_as` and its helpers.)

## Dependencies

H3F-007 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [x] Automated sample checkpoint: a copy of SaAndVla opens, the owned hero at span 4,727,078 gains Air Shield, output/backup are saved and reopened, and the payload diff matches the five planned offsets (`tests/edit_flow.rs`).
- [x] Feature-level HotA acceptance: the user confirms known-spell, current mana and all four primary attribute edits worked in the actual game. See the [manual checklist](../MANUAL_COMPATIBILITY.md).
- Evidence limit: the user report identifies neither the historical SaAndVla/Air Shield case nor a Save/Save As mode; it must not be presented as that exact test.
- [x] Decompressed diff is limited to the reviewed patches; backup equals original compressed source; no-op Save writes nothing. - byte-diff equality with the patch union asserted on the real save; backup bytes compared to the original; a save with no pending edits fails with `nothing-to-save` and never touches disk.
- [x] Failed backup, locked file, changed source or failed output validation preserve a recoverable original and retain unsaved edits. - verified for changed-source (`source-changed`, detected before any write, no backup/temp residue), existing destination (`file-exists`, destination content untouched, edits retained) and post-write validation failure (temp removed, original and edits intact). Locked-file behavior maps to `write-failed` with the same recovery guarantees (same code path as the other write errors).
- [x] Overwrite is never implemented as delete-then-rename; Save As handles an existing target with explicit selection and its own backup. - Save As refuses existing targets (`file-exists`, no-clobber); the commit is a same-directory rename of a verified temp file, which on Windows atomically fails when the destination exists. An explicit overwrite flow (with target backup) remains future work and is not needed for the M3 checkpoint.
- [x] No test or application development save destination can resolve to repository games/ or Data/. - canonical-path guard rejects both (including via junctions), asserted in `edit_flow.rs`; the open source file itself is also rejected as a destination (`same-file`).

## Implementation evidence (2026-11-08)

- `AppState::save_save_as`: guard destination → source fingerprint (path/len/mtime) → plan all pending edits against the immutable source payload → apply all patches (global overlap + preimage checks) → recompress → refuse existing destination → write `<dest>.backup-<millis>` from the retained original bytes and verify it → write/sync a unique temp file in the destination directory → reopen through `SaveDocument::open` and compare the payload exactly → no-clobber rename → re-base the session onto the written file with a clean pending set. Failure at any step removes temp residue, keeps pending edits and leaves the original untouched.
- Frontend: editable attribute fields (validated ranges mirroring the backend), 69-spell learned checklist with per-spell dirty markers, spellbook warning banner, pending-changes summary with Save As / Discard all, per-hero Revert, success/error status line. Values displayed are effective values (base plus pending).
- New commands: `apply_hero_edit`, `get_hero_edit_state`, `get_pending_edits`, `discard_hero_edits`, `discard_all_edits`, `save_save_as` (Rust-owned picker; the frontend never names a path). Registered in `main.rs`, `build.rs` and `capabilities/main.json`.

## Minimal verification

Run focused persistence tests in temp for no-op, changed source, backup failure, locked target and success. Perform one UI open/edit/save/reopen smoke plus one matching-game load of the output copy. Record the game version/result; leave acceptance pending if the game is unavailable.

Covered by `tests/edit_flow.rs` (success round trip incl. backup + original hash; file-exists; source-changed; same-file; forbidden destinations; no-spellbook warning; discard) - 3 tests, private-copy opt-in. Frontend `npm run build` (typecheck + bundle) passes. The later user report confirms the listed hero edits in HotA. Detailed native UI/failure/recovery smoke and separate Save modes remain unrecorded; see the manual checklist.

Verification note: opt-in fixture tests resolve `H3FORGE_FIXTURE_ROOT` against the *crate* working directory, so the variable must be an absolute path (e.g. `D:\Projects\h3forge\games`); a relative value silently skips while reporting `ok`. The 2026-11-08 run executed every fixture test for real (107 passed / 0 failed workspace-wide with the saves root, 39 passed / 0 skips for the resource suite with the `Data` root) and fixed two test-side defects found on the way: a wrong relative fixture depth in the src-tauri tests, and two assertions that counted patched offsets instead of comparing sorted patch unions.

## Direct Save follow-up (2026-11-09)

Implemented direct Save beside Save As. Save targets only the open session source; Save As still requires a new destination and rebases the session onto it. The toolbar keeps both actions visible, disables unsupported/protected/unwritable direct targets with an explanatory tooltip, and retains dirty/discard/result indicators.

Both modes now share a writer: exact typed patches → source metadata **and byte comparison** → create-new backup, flush and verify against retained original bytes → create-new same-directory temporary output, flush and reopen/compare the entire payload → recheck the source and protected paths → commit → rebase/clear pending edits. A Windows read handle denies competing writers during preparation. Existing backup/temp names cannot be overwritten.

Direct Save uses same-directory `std::fs::rename`, whose [Rust 1.99 Windows implementation](https://doc.rust-lang.org/std/fs/fn.rename.html) uses MoveFileExW with SetFileInformationByHandle fallback. It never deletes or truncates the destination first. The baseline metadata is captured from the verified temporary file before commit, avoiding a post-commit metadata-read failure leaving stale edits. Replacement failure retains the original and verified backup; only owned temporary output is cleaned up.

Save As now atomically installs a new hard link to the verified temporary output, then removes the temporary name; this fixes the previous check-then-rename race (Rust rename can replace an existing destination on Windows). It requires a filesystem supporting hard links; unsupported filesystems return a write error with source/pending edits preserved. Direct Save does not require hard links. Repository games/ and Data/ protections use canonical paths, while ordinary installation save folders remain eligible. The capability check probes file/directory permissions without truncating the source.

### Permanent sample regressions

`tests/direct_save.rs` extends the opt-in `H3FORGE_FIXTURE_ROOT` approach using unique ignored copies. It locates the verified hero by span 4,727,078, not name/list order. Tests cover exact no-op payload, primary edits, learned spell add/remove, creature/count replacement, stack clear/add, two Saves without reopen, Save As followed by direct Save, backup equality to each previous version, and full-payload equality with both planned patches and an independent allowed-offset check. Original sample SHA-256 before/after: `f326b6ff04cb2b6d3fd7e44773dfa78eecca28f02277ff201b620c215b3856b5` (unchanged).

Two real-save regression tests passed with 215 heroes. Rejections cover invalid values, existing Save As targets, protected development sources/Data destinations, same-size same-mtime external byte changes, and a real Windows sharing lock that allows preparation but denies replacement (original, backup, and pending edit verified intact). All five existing edit-flow tests also passed. No sample bytes, extracted art or output saves are tracked.

Current status: **implemented, reported hero operations verified in HotA**. The 2026-11-14 recorded report supplements structural/binary checks; it does not certify every gameplay interaction, persistence path or the exact historical fixture. `cargo check --workspace` and the frontend TypeScript/Vite build both passed (one final run each); `git diff --check` is clean.
