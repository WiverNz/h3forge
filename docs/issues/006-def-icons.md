# H3F-006 - Decode required DEF frames and show original spell icons

Status: completed (2026-11-09: all four acceptance criteria satisfied). Milestone: M2. Priority: P0.

## Goal

Render the spell grid using verified local icon frames.

## Scope

Add a native Rust DEF reader for the frame encodings encountered in the required icon resources. Inspect bounded headers/frame tables from SpellInt.def first; validate palette transparency, shadow behavior, crop offsets and dimensions. Establish spell-ID/frame mapping with evidence. Extend the same decoder/catalog to Artifact.def, CPRSMALL.def and TwCrPort.def only as supported. Unknown encodings/IDs return placeholders. Any third-party decoder reuse requires its license/provenance to be recorded; mmarch's MM sprite decoder is not a DEF implementation.

## Relevant files or components

crates/h3forge-resources/src/{def,image,catalog}.rs; web spell grid; resource tests and notices if applicable.

## Dependencies

H3F-004, H3F-005 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [x] A verified spell icon appears with the correct spell label, and at least one transparent frame renders correctly.
- [x] Invalid frame offsets, truncated RLE and excessive dimensions return errors without panics or excessive allocation.
- [x] Unsupported frames cannot display a different spell/creature's graphic by accidental index fallback.
- [x] Decoding/PNG encoding stay in Rust; only requested frames are decoded.

## Minimal verification

Use a minimal synthetic DEF for bounds/RLE tests and visually inspect one real spell frame on a checkerboard. Check one missing-frame fallback. Do not commit extracted art or screenshots containing private data.


## Creature portrait progress (2026-11-09)

- Added an independently implemented bounded Rust interface DEF reader: **type 0x47**, compression **0 (raw)** and **1 (per-row RLE)** only. Formats 2/3, legacy short headers, animation playback, and battle/map shadow/selection semantics are explicitly unsupported. Palette indices and crop offsets are retained; interface index 0 is transparent, not every cyan-colored pixel.
- Format facts cross-checked with [VCMI's DEF reader](https://github.com/vcmi/vcmi/blob/develop/client/render/CDefFile.cpp); no GPL implementation code was copied. Frame-name mapping checked against the [creature portrait table](https://heroes.thelazy.net/index.php/Creature_portraits) and the local `H3sprite.lod/TwCrPort.def` directory.
- Local archive: 151 frame entries, all compression 1, 58×64 canvas. **141 standard creature IDs resolve**; blank/unknown/war-machine/reserved entries are excluded. Selection is by exact embedded filename, never frame index. In particular, Gog/Hell Hound frame positions are reordered in the real directory.
- Only the requested frame is decoded, converted through the existing PNG encoder and cached in memory (bounded to the 141 verified 58×64 portraits). The app invalidates this cache on sprite-archive path/size/mtime changes. Army slots show 44×48 portraits and update when the dropdown changes, including before Apply.
- HotA-exclusive creatures remain placeholders: no attempt to decode HotA.lod or reuse its differently mapped filenames. No copyrighted artwork is tracked or bundled.
- Verification: 4 focused resource tests passed, including the real archive (141 PNGs), truncation/offset/size/RLE rejection, unsupported variants, name lookup, palette/transparency and crop tests. App portrait tests passed, including cached command responses and HotA placeholders; existing hero portraits still resolve. Visually inspected Pikeman, Archer, Griffin, Gog, Hell Hound and Ice Elemental from an ignored contact sheet.
- H3F-006 remains **partial**: the existing spell-ID mapping, real spell icon and transparent spell-frame visual acceptance criterion have not been implemented. H3F-014 remains untouched.

## Spell icon completion (2026-11-09)

- **Mapping established with evidence, not guessed:** `H3sprite.lod/spells.def` = 70 frames, canvas 78×65, **array index = game spell ID** (0 Summon Boat .. 69 Summon Air Elemental). Positional loading proven by VCMI (`client/windows/CSpellWindow.cpp:862`, `setFrame(spell->id)`; `Mods/vcmi/Content/Sprites/spells.json` documents the original View Earth/View Air art swap, reproduced as-is). IDs additionally verified equal to h3sed `metadata.IDS` == h3mtxt `SpellType` == the save flag-array indexes. Frame labels follow the ID order with documented rename trivia (`Sp03Iden` = Visions, eye artwork inspected; `Sp37Back` = Magic Mirror; `Sp52Wind`/`Sp53Muck` = Haste/Slow).
- **Label bug fixed:** `SPELL_NAMES` had been ported from h3sed's alphabetical `metadata.SPELLS` as if it were ID-ordered (every spell mislabeled). It is now the game-ID order (70 names); sample candidate learned spells now read correctly.
- **Implemented:** `h3forge-resources/src/spell_icons.rs` (positional `DefFile::decode_index`, 78×65 assertion, per-ID PNG cache, no neighbouring-frame fallback), desktop `get_spell_icon` command (SourceIdentity-invalidated cache), frontend `SpellIcon` beside every spell name in the Spells tab. All 70 IDs resolve from the real archive; every real frame carries transparent index-0 padding (transparency acceptance met); IDs outside 0-69 return placeholders.
- Verification: 3 new focused resource tests pass against the real archive; full `h3forge-resources` and `h3forge-save` suites pass (table fix + sample round trip updated); no copyrighted artwork committed or bundled.
