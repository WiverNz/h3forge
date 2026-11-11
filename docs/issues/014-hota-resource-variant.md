# H3F-014 - Resolve or explicitly classify the supplied HotA LOD variant

Status: completed. Local stored/zlib archive access and the six Interference/Runes skill icons are identified, verified, and integrated. LZMA stays unsupported (no local fixture). Milestone: M4. Priority: P2.

## Goal

Determine whether HotA-specific resources can be read safely without delaying the editor.

## Scope

Investigate the supplied HotA.lod header/directory anomaly using bounded metadata probes and primary source evidence. Check for an existing reusable Rust solution before adding code. If a transformation/layout is established, implement it behind an explicit detector with range checks and provenance; confirm individual entries and icon mappings. Do not label the archive encrypted/corrupt as a fact without evidence. Stop after one focused investigation session if no trustworthy approach emerges, recording a concrete unsupported reason and next evidence needed.

## Relevant files or components

resource lod/discovery/catalog; RESEARCH.md compatibility notes; notices for any new reuse.

## Dependencies

H3F-004, H3F-005 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [x] Standard LOD behavior remains unchanged and base assets remain available.
- [x] Either the variant has reproducible directory/entry validation and one decoded asset, or it is explicitly classified as unresolved/unsupported with a user-visible fallback.
- [x] No archive is rewritten, bulk-extracted, or accepted solely because its magic/count look normal.
- [x] The outcome clearly distinguishes research closure from actual HotA asset support; unsupported status is not advertised as decoder success.

## Minimal verification

Read one validated entry if supported, plus malformed variant bounds and standard archive regression. Otherwise verify the diagnostic/placeholder path and document the unresolved evidence. No repeated speculative decoding cycles.


## Findings and implementation (2026-11-09)

Verified both D:/Projects/h3forge/Data/HotA.lod and installed
G:/Games/Heroes3_HotA/Data/HotA.lod: key 0xB5A4D744 at 0x0C,
directory at 80, 5232 records of 32 bytes. First 16 bytes are opaque IDs;
XOR-decoded words at record +16/+20/+24 are offset/unpacked/packed size.
Method byte at +28 is not XORed. All ranges validate: 3869 zlib (3),
1363 stored (0), no LZMA (2). Header recognition is not a game-version claim;
prior local notes calling this a 1.7 installation do not describe the archive
layout reliably. No game file was changed or bulk-extracted.

- Implemented bounded Rust read-only access in the existing LodArchive.
  Standard-directory first-record validation takes precedence over key-based
  detection so nonzero reserved bytes do not misclassify standard LODs.
  Existing standard semantics remain; HotA zlib rejects output-size overflow.
  Opaque IDs are exposed as 32-digit lowercase hex, never invented filenames.
- Unknown compression (including raw LZMA1 method 2) gets a typed error before
  payload allocation. The reference describes LZMA1 at payload +1 with 256KiB
  dictionary/lc3/lp0/pb2, but no local fixture verifies it; no speculative
  decoder or dependency was introduced. This is **not full 1.8+ coverage**.
- Format reference: [homm3data/lodfile.py](https://github.com/Laserlicht/homm3data/blob/main/homm3data/lodfile.py),
  MIT copyright Johannes Schauer (2014), Laserlicht (2024). License checked
  through GitHub's repository license endpoint and preserved in
  licenses/homm3data-MIT.txt. Existing mmarch notices remain. No Python runtime.
- Minimal regression: 28 focused LOD/PCX tests pass (including synthetic XOR
  stored/zlib, wrong key, count/size/range limits, truncated/corrupt streams,
  unknown compression, and standard reserved-word behavior). Real archive
  test validates directory and decodes opaque resource
  7aa5a6b612d85711b32e3e287b850e66 (4-frame interface DEF, first frame
  iam000A000, 32x32) to PNG; this is not claimed to be a skill icon.

## Skill icon identification and integration (2026-11-09, second pass)

The opaque-name blocker above was resolved by content, not by hashing. A
bounded read-only scan of the local archive decoded candidate 32x32 image
containers and compared pixels with reference artwork published in the VCMI
HotA mod repositories (references used for identification only, never
bundled or committed):

- Directory entry `48043b9164e06a6f8e626a6b6ca7caa1` is a 93-frame container
  with magic `D32F` whose frames are named `secsk32._000`..`secsk32.A092`.
  Layout reverse-engineered from the local bytes; no third-party decoder
  reused. Documented as a bounded parser in `crates/h3forge-resources/src/hota32.rs`.
- Frames 87/88/89 decode **pixel-exactly** (RGBA SHA-256 equal) to the
  verified Interference Basic/Advanced/Expert references. Frames 90/91/92
  match the Runes references by identical alpha silhouettes and per-frame
  minimum RGB distance on the correct (frame, mastery) diagonal; the
  published Runes reference PNGs are re-brightened re-encodes of the same
  art, which is why they are not byte-identical.
- The frame numbering continues the base-game `secsk32.def` formula
  `2 + mastery + 3 * skill ID`: Interference (28) -> 87/88/89, Runes (29)
  -> 90/91/92. Frames are selected by exact embedded name
  (`secsk32.A087`..`secsk32.A092`), never by directory position.
- VERIFIED format fact: all six icons are fully opaque squares (alpha 255
  everywhere, drawn backgrounds) - unlike `secsk32.def`, whose palette
  index 0 is transparent.

Integration: `HotASkillIcons` in `crates/h3forge-resources/src/skill_icons.rs`
loads the container from `HotA.lod`; the desktop `skill_icon` command serves
ids 28/29 from it (identity-cached like the other artwork), so mastery
changes immediately select the correct image in the Skills tab. All six
icons resolve in the installed game; placeholders remain only for genuinely
unverified ids. Focused tests: `crates/h3forge-resources/tests/hota_skill_icons.rs`
pins the exact decoded PNG bytes (CRC-32) of all six icons, asserts the
opaque-alpha fact and pairwise distinctness, and exercises malformed D32F
bounds in the `hota32.rs` unit tests. No archive was modified; no
copyrighted artwork was bundled; no new dependency was added.

## Exact skill mapping blocker (first pass - superseded by content identification)

[Interference config](https://github.com/vcmi-mods/horn-of-the-abyss/blob/vcmi-1.7/mods/interference/content/config/hotaInterference/interference.json)
identifies small/basic, small/advanced, small/expert under
hota/skills/interference/icons. [Runes config](https://github.com/vcmi-mods/horn-of-the-abyss/blob/vcmi-1.7/mods/bulwark/content/config/hota/bulwark/runes/runesSkill.json)
and repository tree identify the corresponding paths under
hota/bulwark/skills/runes/icons. Those are VCMI asset paths, not local opaque
LOD identifiers. [Issue 305](https://github.com/vcmi-mods/horn-of-the-abyss/issues/305)
discusses extraction/renaming and manual image identification but supplies no
verified six-entry mapping in the inspected discussion/configs.

A bounded Rust metadata probe reads at most 4096 stored prefix bytes for
entries with declared output 800..1,000,000 bytes, and fully reads only
matching 32x32/44x44/82x93 interface DEF candidates. Eight such DEFs exist;
none has a secondary-skill atlas frame directory. It also counts 249
PCX-sized candidates at those dimensions, with no filenames; no PNG-signature
candidates at those dimensions. No candidate identity is inferred from
position, dimensions or adjacent IDs. Simple MD5(name) for SECSK filenames
did not match opaque IDs. No copyrighted images were downloaded.

Superseded approach note: filename-hash reverse-engineering was abandoned in
favor of direct pixel comparison; the D32F resource was found by scanning
decoded image containers, not by hashing names.
