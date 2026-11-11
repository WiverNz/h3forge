# H3F-004 - Adapt mmarch Rust into a bounded read-only LOD and PCX library

Status: completed (2026-11-08). Milestone: M2. Priority: P0.

## Goal

Reuse proven archive/PCX code through a small internal Rust API.

## Scope

Extract only required H3 paths from mmarch archive.rs/lod.rs/image.rs into h3forge-resources with MIT notice, commit provenance and adaptation notes. Preserve decompression logic where suitable. Replace whole-file load with header/table reads and selected-entry seeks. Return raw decompressed entries; decode PCX explicitly to RGBA/PNG. Remove writer/CLI/MM6–8 dependencies and add checked ranges, allocation limits and index validation. Reject the supplied nonstandard HotA directory instead of trusting its header.

## Relevant files or components

crates/h3forge-resources/src/{lod,pcx,error,lib}.rs; focused resource tests; licenses/mmarch-MIT.txt; THIRD_PARTY_NOTICES.md.

## Dependencies

H3F-001 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [x] A standard LOD can enumerate names and read one raw/compressed entry without extracting or reading all payloads at startup.
- [x] Indexed and 24-bit Heroes PCX are decoded in Rust; palette handling is explicit.
- [x] Short tables, impossible ranges, huge counts and bad indices return errors; zero-byte archives are not accepted.
- [x] No archive writer or external mmarch executable is used; sibling repository remains unchanged.

## Implementation notes (2026-11-08)

- **API** (`crates/h3forge-resources`): `LodArchive::open/open_with_limits` (metadata-only: 92-byte header + directory table, payloads never read at open), `entries() -> &[EntryInfo]`, `find_entry(name)` (case-insensitive, duplicate names rejected as ambiguous), `read_entry(index) -> Vec<u8>` (raw decompressed bytes, packed entries decompressed to exactly their declared unpacked size). `decode_pcx(bytes, ResourceLimits) -> PcxImage` keeps indexed pixels + 768-byte R,G,B palette explicit before optional `to_rgba()` (straight RGBA, alpha 255 - no transparency convention applied without fixture evidence). `ResourceLimits` defaults: 100,000 entries, 32 MiB per entry, 16M pixels.
- **mmarch reuse**: H3 header/entry layout, per-entry bounded zlib semantics (stop at declared unpacked size, tolerate trailing bytes, mmarch's 1032:1 impossible-size ceiling) and the 12-byte-header PCX model, transcribed from `mmarch-rust/src/lod.rs` and `image.rs` (commit `fc104f2a7d8b170ed67bc05b9bfac5d0bb9a60b8`) with MIT notice in `licenses/mmarch-MIT.txt` and provenance/adaptation headers in each module. Bounded zlib reimplemented over the workspace's existing `miniz_oxide 0.9.1` core API (zlib-header parse flag, non-wrapping output) instead of mmarch's 0.7 stream API.
- **Hardening over mmarch**: whole-file `fs::read` replaced by `Read + Seek` on the open file; short directories, non-printable names, data ranges outside the file or inside the directory table, and zero-stored-with-unpacked entries are rejected up front; the first record failing validation yields `UnsupportedDirectory` (the supplied `HotA.lod` lands there and is rejected, not half-enumerated); allocations are limited before reserve/decompress; no PCX-to-BMP auto-conversion.
- **HotA.lod**: header is plausible (version 200, count 5232) but the first directory record's name is non-printable garbage, so `open` returns `ResourceError::UnsupportedDirectory`; the header count is never reported as enumerated resources. Layout investigation stays with H3F-014.

## Verification (2026-11-08)

- 25 synthetic tests (`tests/lod_pcx.rs`): raw + zlib-compressed round trips, metadata-only open (payload untouched), zero-byte/short header/bad magic/huge count/short table/garbage first record/bad later record/entry-into-table/ambiguous name/bad index/over-limit size/impossible unpacked size/truncated and corrupt zlib; indexed and 24-bit PCX pixel/palette checks plus truncated/bad-dimension/bad-stride/missing-palette/pixel-limit failures.
- 4 opt-in real-archive tests (`tests/local_archives.rs`, read-only against `Data/` or `H3FORGE_FIXTURE_ROOT`): `H3bitmap.lod` enumerates 4,912 entries and `HPL000KN.PCX` reads (4,303 packed → 4,492 bytes) and decodes as 58x64 indexed with cyan palette entry 0 and pinned pixel values; `HotA_lng.lod` (225 entries) `CAMPBACK.pcx` decodes as 800x600 24-bit with pinned B,G,R→R,G,B pixel values; `HotA.lod` rejected `UnsupportedDirectory`; `HotA_ext.lod` (zero entries) opens; `h3ab_bmp.lod` (zero bytes) rejected `EmptyArchive`.
- `cargo test -p h3forge-resources` 29 passed; `cargo clippy --all-targets` clean; `cargo fmt --check` clean; `cargo check --workspace --locked` passes.

## Limitations

- PCX decodes to indexed data or RGBA; PNG encoding is deferred to the desktop command layer (H3F-005/006) where transfer format is wired.
- No transparency applied to PCX output (no fixture evidence for a convention); indexed palette is preserved for callers.
- MM6/7/8-format LODs (version word > 0xFFFF) are out of scope; `HotA.lod`'s directory layout remains unsupported pending H3F-014.

## Minimal verification

Run a small synthetic archive/PCX fixture test, including raw/compressed and invalid-range entries. Read one local H3bitmap portrait entry read-only. Inspect I/O boundaries to confirm metadata-only opening.
