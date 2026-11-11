# H3F-002 - Read and identify save containers safely

Status: completed (read-only parser; edits intentionally unavailable). Milestone: M1. Priority: P0.

## Goal

Open private save copies as bounded, integrity-checked data without offering writes.

## Scope

Implement gzip integrity validation, bounded decompression, H3SVG/H3SVC/Chronicles recognition and raw version identifiers. Parse map text using observed little-endian lengths with bounded reads. Retain original compressed/payload bytes. Identify the sample as HotA family 44/11 but mark unvalidated edits unavailable. Reject unsupported containers/trailing-member cases explicitly. Add opt-in local fixture-root handling; synthetic CI fixtures require no game data.

## Relevant files or components

Implemented in `crates/h3forge-save/src/{container,detect,error,domain,parse,locate,definitions}.rs`; tests in `crates/h3forge-save/tests/{e2e,sample_save}.rs` plus module unit tests. No app commands/DTOs yet (H3F-003); the desktop command surface is unchanged and registers no write command. h3sed MIT notice adopted under `licenses/h3sed-MIT.txt`.

## Dependencies

H3F-001 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Implementation notes (2026-11-08)

- `container.rs`: single-member gzip with manual header parse (incl. FHCRC validation), bounded DEFLATE via miniz_oxide 0.9.1 (`=`-pinned, cached), CRC-32 + ISIZE trailer validation, explicit `MultiMemberGzip`/`TrailingData` rejection. Unlike h3sed's patched gzip reader, integrity failures block the open. Default limits: 64 MiB compressed / 256 MiB payload (`Limits` is injectable for tests).
- `detect.rs`: magic `H3SVG`/`H3SVC`/`HCHRONSVG`; version u32 LE at payload offsets 8/12; release-order family matching (RoE 16-41/0, AB 42/1, SoD 42-43/0-4, HC by magic, HotA >=44/>=5). `Compatibility` distinguishes `Readable` (recognized, read-only) from `Verified` (none yet; `is_editable()` is false everywhere). Known magic with unknown version bytes yields `Unsupported` plus a diagnostic, not an error.
- `parse.rs`: map name/description via h3sed's greedy 0..=100-byte post-magic scan with three-NUL anchor; lengths are little-endian u16 fields with bounded reads (cap 1024 per text). No metadata is a diagnostic, not an error.
- `locate.rs`: h3sed HotA hero locator replayed natively: scan from offset 30 000, 1121/1122-byte record structure checks (skill-count word, name anchor, 70+70 spell flags, equipment word pattern, 64 inventory entries, 30 mastery bytes, 29/30 trailing slot run), then secondary checks - name strip, equipment grammar, and an added owner-range (0-7/255) check. After the first acceptance the search window is 5 000 bytes, matching h3sed. Records failing secondary checks are kept as low-confidence candidates with `RejectReason`s; nothing is silently dropped. Only the HotA layout has fixture evidence; other recognized families return `HeroScanNotImplemented` diagnostics instead of pretending an unproven layout.
- `domain.rs`: `HeroView` per record - owner, raw name bytes + lossy display text, level, experience, experience-derived level, spell points, movement, four raw primary attributes, decoded secondary skills (ID/mastery/slot, h3sed semantics), learned/available spell flag arrays, army (id,count) pairs, raw equipment/inventory words, and notes (level/experience mismatch, duplicate names, rejections). Spans give byte access to the untouched record for future milestones.
- Hero identification: 215 sample candidates are **not** presented as 215 confirmed heroes. All 215 pass every implemented structural check (High confidence), which reproduces the research baseline (first offset 4 657 602, last 4 921 020, all 1 122 bytes, owner mix 179/14/12/5/5) - but confidence is labeled detection confidence, not game-verified, and low-confidence records would list separately with reasons.

## Acceptance criteria

- [x] Copied SaAndVla reports 5,271,294 payload bytes, H3SVG 44/11 and the researched map name. (opt-in `sample_save` test: SHA-256 `f326b6ff…3856b5` verified first, works on a copy under `.local/fixtures/`, map name `- The Mysterious Island - COOP`, description 295 bytes.)
- [x] Truncated, invalid-CRC, unknown-header and over-limit input return typed errors without panics or partial editable sessions. (`SaveError` variants; `is_editable()` false for every format; synthetic gzip fixtures cover each case.)
- [x] Loading is read-only and preserves all source bytes; test destinations cannot resolve into repository games/ or Data/. (`original_bytes`/`payload` are immutable; the sample test canonicalizes its copy destination and asserts it stays under `.local/fixtures/`.)
- [x] Detection distinguishes family recognition from validated layout/operation support. (`Compatibility::Readable` vs `Verified`; unknown versions `Unsupported`; non-HotA families recognized but not hero-scanned.)

## Minimal verification

Run focused container tests on synthetic cases plus one opt-in sample-copy test using the recorded SHA-256. Confirm no write command is registered.

Result: 30 lib unit tests + 5 e2e integration tests pass offline (container failures, detection boundaries, metadata edge cases, locator accept/reject/window behavior). Opt-in sample test passes with `H3FORGE_FIXTURE_ROOT=games` against a `.local/fixtures` copy: 215 heroes, all High confidence, research candidate at 4 727 078 (owner 4, level 2, 13 learned spells `[3,15,17,20,25,33,35,37,42,43,44,47,64]`) matches the recorded baseline exactly, including the 8 level/experience mismatches. `cargo clippy -p h3forge-save --all-targets` clean. Desktop app untouched; its only registered command remains `get_app_info` - no write command exists.

## Follow-ups (not this issue)

- H3F-003 wires commands/DTOs and the read-only overview UI; selectable legacy name decoding (163/215 sample names are CP1251 bytes; raw bytes are preserved, lossy display may mojibake).
- H3F-007+ must treat 44/11 as `Readable`, not `Verified`; edit gating needs per-version validation evidence.
