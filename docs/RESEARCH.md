# Repository and format research

## Manual compatibility update (recorded 2026-11-14)

The user confirms that hero army composition, visited tent states, artifact
addition, secondary skill replacement/addition, current mana, known spells and
all four primary attributes worked in actual HotA. This is independent manual
evidence, not a parser-test inference. Exact version, save filename, build and
artifact placement were not supplied. See [MANUAL_COMPATIBILITY.md](MANUAL_COMPATIBILITY.md)
for individual results and remaining gates. Historical sample IDs, offsets and
hashes below must not be assigned to this report.

## Xyron army ordering correction (2026-11-14)

Verified in the current `games/SaAndVla.GM2` via an ignored copy: Xyron's
hero record starts at payload offset 4,307,390. The seven `(creature ID,
quantity)` pairs are `(43,1418), (45,5080), (47,6880), (49,230), (51,116),
(53,5008), (55,2251)`. The supplied game screenshot agrees with this
left-to-right quantity order, rather than the request's slot-2/3 description.
Existing ID/count offsets (record +113/+141, four-byte stride) are correct.
Each of the seven count edits round-trips with no other payload changes.

Root cause of the apparent creature swap: standard portrait numbers 44/45
contain hounds and 46/47 contain gogs, unlike creature IDs. Local decoded
frames establish the same exception for HotA's portrait-number-plus-two
mapping. The earlier cross-container comparison was not independent proof
of creature identity for these four frames. Corrected shared artwork lookup;
see [resource specification](formats/GAME_RESOURCES_FORMAT.md#e-creature-portraits).
The shared editor now presents a single horizontal slot row, scrolling on
narrow panels, without permuting DTOs or edit indices. Army composition now has a user-reported HotA pass; the exact Xyron
fixture, slot-by-slot edits and new native layout were not identified in that report.

Investigated 2026-11-08. This document distinguishes **observations**, **inferences**, and **unresolved work**. No builds, dependency installations, upstream test suites, application implementation, save writes, or archive writes were performed. Small standard-library Python probes were research tools only; H3Forge will not embed or require Python. They evaluated selected source constants without importing the wxPython application, decompressed the sample in memory, and read archive directories/selected entry headers. No game data was extracted to disk.

## Source provenance

| Reference | Local source | Snapshot |
|---|---|---|
| H3SED | `D:/Projects/h3sed_4.1/src/h3sed/` | Supplied 4.1 tree; no `.git` metadata available; inspected files include November 2026 modifications |
| mmarch | `D:/Projects/mmarch/mmarch-rust/` | Cargo package 7.0.1, commit `fc104f2a7d8b170ed67bc05b9bfac5d0bb9a60b8` |
| FlightTrace | `D:/Projects/FlightTrace/` | Commit `267b19dec2e38f7311d0c923441370eee20689d4`, package 0.1.6 |
| H3Forge | `D:/Projects/h3forge/` | Initially only `games/` and `Data/`; no Git metadata or existing project files observed |

Upstreams: [H3SED](https://github.com/suurjaak/h3sed), [mmarch](https://github.com/might-and-magic/mmarch), [FlightTrace](https://github.com/WiverNz/FlightTrace). Findings below concern the supplied local snapshots, not an assumed identical latest upstream revision.

## H3SED architecture and byte handling - verified

`metadata.py` contains definitions, version-aware `Store`, search patterns, and `Savefile`. `common/__init__.py` implements typed/validated data containers and `NamedEntity` state. `hero/__init__.py` contains hero domain objects and coupling such as primary attributes/equipment. Sections in `hero/{stats,skills,spells,inventory,equipment,profile}.py` combine GUI controllers with parsing/serialization functions. `common/army.py` serves both heroes and towns. `town/profile.py` exposes read-only profile data. `version/{roe,ab,sod,hc,hota}.py` override definitions, classes, regexes, offsets, and bounds through `version.adapt()`.

### Read, locate, modify, write

- `metadata.py:1371`, `patch_gzip_for_partial()`: monkey-patches Python gzip EOF validation to tolerate CRC problems. H3Forge must **not** inherit this permissive write behavior; integrity failures block editing.
- `Savefile.read()` at 1445 decompresses the file, retains original `raw0` and working `raw`, detects the version, parses metadata, then heroes/towns. Standard magic is `H3SV[GC]`; Chronicles adds `HCHRONSVG`.
- `populate_heroes()` at 1554 uses a version-adapted byte regex, separate name/equipment checks, and stores each match's raw bytes and span. It starts at offset 30,000 to skip possible campaign carry-over heroes. After the first match, subsequent searches use 5,000-byte windows. This is heuristic record discovery, not a complete file schema; it can miss records and needs explicit confidence checks in Rust.
- `parse_towns()` at 1592 searches before the hero area, checks name lengths, creature IDs/counts, adjacency and candidate lengths. It can discard an isolated candidate and infer HotA town size from adjacent records. Town discovery is especially uncertain. Hero and town coordinates associate visiting/garrison heroes.
- `NamedEntity.serialize()` invokes section serializers over the retained entity bytes. Most serializers clone the old byte array and replace known ranges. `Savefile.patch()` splices these into the decompressed save. `write()` recompresses the payload through gzip. `write_ranges()` exists, but its body opens `self.filename` despite accepting a destination argument: do not port that file-writing implementation.
- `conf.py` defaults `Backup=True`; this is not a transactional Windows persistence guarantee. Reuse field semantics, not H3SED's direct `wb` output strategy.

### Fields and offsets

Offsets below are relative to the matched hero block (starting at faction), not the compressed file. Integer decoding in `lib/util.py:787`, `bytoi()`, is **little-endian**. The header regex comment describing big-endian text lengths is misleading; actual metadata parsing calls the little-endian helper.

| Field | Default relative offset/representation | Constraints and coupling |
|---|---|---|
| Owner | 0, byte | Player 0–7 or neutral 255; ownership is not proof of active map status |
| Movement total/remaining | 31/35, u32 LE | Useful inspection; defer writes |
| Experience / level | 39, u32 LE / 49, u8 | Level table exists; HotA level cap in source is 74, default 75; do not silently derive one from the other |
| Secondary skill count | 43, four-byte region | Source reads/writes low byte; regex constrains remaining bytes to zero |
| Spell points | 47, u16 LE | 0–65535 storage range |
| Army | IDs at 113, counts at 141 | Seven u32 IDs plus seven u32 counts; occupied count 1–u32 max; empty encodings vary |
| Name | 169, 13 bytes | NUL-padded legacy text; display names can be renamed/non-ASCII |
| Skill mastery / order | 182 / 210 by default | ID-indexed bytes: 0 absent, mastery 1–3; order is one-based |
| Primary attributes | 238–241, one byte each | Raw bytes are not equivalent to effective stats; overflow/wrapping and artifact bonuses matter |
| Learned / available spells | 242 / 312 | Two arrays of 70 bytes, each 0 or 1; not packed bitfields |
| Equipment | 382–533 | Nineteen eight-byte slots, including war machines and spellbook |
| Backpack | 534–1045 | 64 eight-byte entries |
| Combination reservations | 1046–1055 area | HotA also stores unknown information here; preserve it |

Artifacts usually use an ID word plus sentinel word; spell scrolls encode a combined eight-byte value. `equipment.py:573`, `inventory.py:485` preserve unrecognized IDs through metadata additions and retain original bytes for unchanged entries. Equipment serialization handles reservations and preserves some HotA values above 5; that partial knowledge is insufficient for unrestricted combination-artifact editing.

`spells.py:193` updates learned and available arrays, considers spellbook presence, unions equipment-granted spells, and tries to preserve suppressed artifact spells for potentially map-banned spells. It does not fully parse map restrictions. Rust should patch only the intended spell and necessary coupled bytes, not normalize all 70 entries on every edit.

`common/army.py:514` preserves untouched empty slots, because the game uses both zero and FF forms. New empty slots use FF ID plus zero count. `is_readonly()` blocks town-army editing when a garrison hero exists. `hero/profile.py:144` reads location/on-map status from before the fixed block, backing over an optional biography; these offsets require bounded validation. H3SED does not expose a verified portrait ID in this profile parser. Mapping display names to portraits would fail for renamed heroes.

`hero/__init__.py` supplies typed casts, skill realization, primary-stat/equipment coupling, equipment-slot conflict solving, and inventory management. `metadata.py` and version modules supply IDs, spell schools, level thresholds, artifact slot/bonus/spell relationships, creature definitions and limits. Port these as Rust tables and explicit operations; do not port wx widgets, mutable global stores, or embedded `images.py` art.

### Version support in the reference

| Family | H3SED discriminator | Important difference / H3Forge policy |
|---|---|---|
| Restoration of Erathia | major 16–41, minor 0 | 18 equipment slots; inventory shifts to former side5 offset; no combo reservations; different profile/town handling |
| Armageddon's Blade | major 42, minor 1 | Detected before overlapping SoD range; additional definitions |
| Shadow of Death | major 42–43, minor 0–4 | Expanded artifacts/creatures; do not identify Complete separately without evidence |
| Heroes Chronicles | `HCHRONSVG` | Different header/magic adaptations and content definitions |
| HotA | major >=44 and minor >=5 | Open-ended detection is overly broad for an editor's write gate |

In `version/hota.py:461`, skill order moves to **1092**. The regex allows 30 mastery bytes at 182, then 26 legacy bytes, and 29–30 order bytes at the end (1121/1122-byte match). `BULWARK_MINOR=0x0A` controls Bulwark/Runes definitions and legacy class overrides. The source defines HotA towns through type 11 and town lengths 546–581. HotA primary-stat effective ranges differ from base game. It also caps its skills range at 29 while including 30 skill definitions: do not blindly equate storage capacity, usable slots and UI rules. Test the intended supported operation.

Known limits: heuristic positions, broad version detection, partial campaign handling, locale-dependent legacy decoding, incomplete town layouts, incomplete map spell restrictions, opaque HotA reservation data, and no verified identity-to-portrait field. Unknown/future format support must not be inferred from a successful regex match.

## Actual sample - verified observations, limited interpretation

`games/SaAndVla.GM2` was read only. Standard gzip decompression succeeded with CRC and ISIZE verification.

| Property | Observation |
|---|---|
| Compressed size | 1,512,923 bytes |
| SHA-256 | `f326b6ff04cb2b6d3fd7e44773dfa78eecca28f02277ff201b620c215b3856b5` |
| Decompressed size | 5,271,294 bytes |
| First 16 payload bytes | `48 33 53 56 47 00 00 00 2c 00 00 00 0b 00 00 00` |
| Content family | `H3SVG`, major 44 at offset 8, minor 11 at offset 12: matches H3SED HotA predicate |
| Map name | `- The Mysterious Island - COOP` (30 bytes) |
| H3SED locator replay | 215 candidates; first offset 4,657,602, last 4,921,020; sampled matches length 1122 |
| Owner bytes | 179 neutral (255); 14 owner 4; 12 owner 6; 5 owner 7; 5 owner 0 |

The replay used H3SED's HotA regex, equipment/name checks and window strategy; it was not a full H3SED GUI run or game-load test. Names include `AbsoluteZero`, `J. Lannister`, `Zehirmahnn`, and bytes intelligible as CP1251 Cyrillic (for example Orrin and Aiden in Russian). CP1251 is a display inference, not a reason to rewrite strings.

An owned spellbook-bearing candidate at offset **4,727,078** has owner 4, level 2, spellbook word 0, and learned spell IDs `3,15,17,20,25,33,35,37,42,43,44,47,64`. This is a useful controlled-edit candidate after validating its identity and status. Do not choose neutral/unused records merely because they match.

**Conclusion:** the content is a HotA-family gzip save, irrespective of `.GM2`. The local source recognizes minor 11 using its latest definitions but does not explicitly document minor 11 as an exact retail release. No write or game reload was attempted. Exact build compatibility, portrait IDs, complete town inventory and runtime validity of edits remain unproven.

### H3F-002 implementation evidence (2026-11-08)

Verified by full locator replays (development-time Python probes; H3Forge ships no Python) and the Rust parser in `crates/h3forge-save`:

- **Locator selectivity on the sample:** 215 structural matches, **0** rejected by the name/equipment checks; all owners in {0-7, 255}; distinct names 215, duplicates 0. Inter-record gaps: 164 of 214 are exactly 38 bytes, four larger (400-739). No u16/u32 count field equal to 215 exists in the 800 bytes before the first record - the record count is not verified against a schema field, so detection confidence stays labeled, not game-verified.
- **Exact experience table:** `metadata.EXPERIENCE_LEVELS` (levels 1-75; level 17 is 48279, not the 48300 some sources quote) yields **8** stored level/experience mismatches at offsets 4746076 (33 vs 36), 4753036, 4775039, 4809087, 4818431 (30 vs 33), 4821938, 4873272, 4911761 (all stored 1, derived 2). The map is heavily scripted, which likely explains scripted levels; mismatches are surfaced as diagnostics, not rejections.
- **Name charset:** 163 of 215 names carry high bytes; decoding as CP1251 yields coherent Russian hero names plus English custom names (52 ASCII, e.g. `Captain Nemo`, `J. Lannister`, `Zehirmahnn`). Renamed/scripted heroes exist, so roster membership was not used as a verification signal.
- **Research candidate @4727078 decoded:** owner 4, level 2, experience 1064, spell points 73, movement 1830/1830, primaries 3/7/6/6, skills [Wisdom, Intelligence, Mysticism], 13 learned spells `[3,15,17,20,25,33,35,37,42,43,44,47,64]`, army creature IDs 71/73/75/77/80/77/81 with counts 415/251/168/82/9/12/38.
- **Rust parity:** the native parser reproduces every recorded number (payload size, fingerprint, 44/11 detection, map name/description, 215 records, offsets, owner mix, candidate fields) via the opt-in `sample_save` test.

### H3F-003 implementation evidence (2026-11-08)

- **Sample through the session layer:** grant → open → list → detail reproduces the 215-record baseline; the research candidate at span 4,727,078 is selectable by opaque ID and shows owner Orange, spellbook carried, learned spells `[3,15,17,20,25,33,35,37,42,43,44,47,64]`, skills Wisdom/Intelligence/Mysticism. At least 100 of 215 names carry non-UTF-8 bytes consistent with the recorded CP1251 population; raw bytes are preserved and the UI re-decodes them locally (Windows-1251 `TextDecoder`), never rewriting stored bytes.
- **Spellbook semantics ported from h3sed** (`hero/stats.py` `parse_special`): the word at equipment slot 17 (offset 518) counts as carried unless all four bytes are 0xFF; the sample's owned candidate has word `00 00 00 00`, i.e. carried.
- **Name tables ported** from h3sed `metadata.py`/`version/hota.py`: 30 skill names, 3 mastery names, 69 spell names (flag slot 69 has no researched name and stays "unknown"), 8 player faction labels, 19 equipment slot labels. Detection remains structural, not game-verified.

### H3F-012 implementation evidence (equipment editing)

- **Observed 44/11 slot conventions** (all 215 sample records): occupied wear slots carry the artifact ID word with a `00 00 00 00` tail (h3sed writes `FF` tails; both decode); blank slots are `FF` x8; slot 16 (catapult) holds war-machine ID 3 on every record; the spellbook slot word is `00 00 00 00`/`00 00 00 00 FF FF FF FF` when carried, `FF` x4 otherwise; combination-reservation bytes 1046–1055 are zero on every sample record.
- **Stored attributes include equipped bonuses** (h3sed `ensure_primary_stats` model): an equip/unequip pair on a real 44/11 hero (Orrin, span 4,657,602, attack 2) restores the payload byte-for-byte, which only holds if `stored = base + bonus` is the game's own invariant. Bonus deltas follow h3sed `ARTIFACT_STATS`; spell availability follows `spells.py:serialize` (in-book or granted; bannable 0/1/7 not newly set while stored 0). Learned flags are never written.
- **Safety gates proven by tests:** wrong-slot artifacts, The Grail, unverified/combination IDs, war-machine/spellbook slot targets, unknown occupants, and non-zero reservation bytes are all rejected with actionable messages; identical patches from coupled edits collapse, divergent overlaps error explicitly.

Other local save references are `AUTOSAVE.GM2`, `BATTLE_[hotseat].GM2`, and `SaAndVla.GM2.20261107`. They were listed, not assumed equivalent or parsed. Battle/campaign saves require separate support evidence.

## Save editing surface - feature matrix (2026-11-08, H3F-007/008 baseline)

Classification: **VERIFIED** = parsed by H3Forge with sample evidence (or, for writes, additionally round-trip-verified on a private copy); **PARTIAL** = structure documented by h3sed / the h3svg specification / another editor but not yet verified against the sample; **UNKNOWN** = insufficient evidence. Write status refers to the current H3Forge edit pipeline (`plan_hero_edits` + `BytePatch` + Save As).

### Heroes

| Feature | Structure / offsets (HotA record) | Reference | Read | Write | Risks / notes | Next |
|---|---|---|---|---|---|---|
| Primary attributes | 238–241, u8 each; HotA game ranges 0–99 (might) / 1–99 (magic), handicap ≥231 | h3sed `stats.py`, `version/hota.py` | VERIFIED | **VERIFIED** (conservative ranges; raw overflow values preserved read-only) | recomputed by the equipment bonus delta only when equipment changes | - |
| Experience / level | 39 u32 / 49 u8; HotA level cap 74 | h3sed `metadata.py`, `version/hota.py` | VERIFIED | **VERIFIED** (independent fields, never auto-synced) | scripted maps set levels directly | - |
| Spell points | 47 u16 | h3sed | VERIFIED | **VERIFIED** (u16 range) | max mana derives from knowledge ×Intelligence; not recomputed | - |
| Learned / available spells | 242 / 312, 70 flag bytes; coupled rule ported from `spells.py:serialize` | h3sed | VERIFIED | **VERIFIED** (learned + available-with-spellbook; artifact-grant caveat documented) | Spell Scroll grants not decoded; map-banned spells preserved by never normalizing | - |
| Spellbook / war machines | equipment slots 17/13/14/15/16, ID word + sentinel | h3sed `stats.py` | VERIFIED (carried detection; machines decode as unknown-named IDs 3–6) | not exposed (read-only by design) | artifact IDs not verified for 3–6; spellbook equipping is an explicit-action candidate later | - |
| Secondary skills | count 43; mastery 182 (30 bytes); order 1092 | h3sed `skills.py` | VERIFIED | not yet | slot-order table coupling | H3F-010 |
| Army | IDs 113 / counts 141, 7×u32; empty encodings vary (0 and FF forms) | h3sed `common/army.py` | VERIFIED | **VERIFIED** (exact 44/11, parser/session round trip; army composition also user-verified in HotA, test metadata unspecified) | untouched empty/unknown words preserved; clear writes FF ID + zero count | H3F-009 |
| Equipment / backpack | 19×8 bytes @382; 64×8 @534; scrolls use two u32 words | h3sed `equipment.py`/`inventory.py` | VERIFIED backpack IDs/scrolls + decoded equipment | **VERIFIED** backpack + wear-slot equip/unequip with attribute-bonus and granted-spell coupling (round-trip inverse on the sample) | combination artifacts/assembly, equipped scrolls, war machines, spellbook stay read-only; reservation bytes 1046–1055 block edits when non-zero; artifact addition user-verified in HotA with placement unspecified; equipment coupling retains manual game-load gate | H3F-011/012 complete |
| Identity / portrait | 10/11/12 (type/class/portrait) | h3svg spec | VERIFIED (H3F-005) | n/a (identity) | - | - |
| Owner | 0, player 0–7 / 255 | h3sed | VERIFIED | deliberately not exposed | ownership changes have map-wide effects | - |
| Coordinates X/Y/Z | before the fixed block in profile area (biography caveat); town regex offsets 4–6 for towns | h3sed `profile.py`, h3svg spec | PARTIAL | closed | map object index must stay consistent | H3F-019 |
| Movement points | 31/35 u32 | h3sed | VERIFIED | not yet | refill semantics (total derives from Logistics etc.) | H3F-010+ |

### Towns / castles

| Feature | Structure | Reference | Read | Write | Next |
|---|---|---|---|---|---|
| Name / type / owner / x/y/z | 44/11 record `567 + name_len` (568-581); owner byte 0, type 3, x/y/z 4-6, name length u16 @69, name @71 | h3sed `TOWN_REGEX` (`version/hota.py:507`), types 0-11; SAVEGAME_FORMAT.md section D | VERIFIED_IN_H3FORGE (37/37 sample towns, map cross-checked) | none | H3F-019 for coordinates |
| Garrison / visiting heroes | 7 creature IDs @9, 7 counts @37; garrisoned/visiting hero type bytes @65/@66 resolve to hero records by `hero_type` | h3sed `common/army.py` (shared), `parse_towns`; h3svg `Town` | VERIFIED_IN_H3FORGE (sample: 17 garrisoned, 1 visiting) | garrison ID/count words only: exact 44/11, verified town, no garrisoned hero (H3F-013); in-game load pending | - |
| Buildings: built, visible and constructible masks | three u64 after the name (+89, +97, +105); bit numbers per h3svg `TownBuildingTypeH3SVG` | h3svg spec; SAVEGAME_FORMAT.md D.1 | VERIFIED_IN_H3FORGE (37/37) | verified subset: guild, tavern, citadel/castle, town/city hall, market, silo, blacksmith, dwellings (H3F-022); in-game load pending | Fort/Capitol/Shipyard/faction structures open |
| Mage guild level + spell lists | level byte @67; counts +84, 30 x i32 slots +113, banned mask +233 | h3svg spec; SAVEGAME_FORMAT.md D.1 | VERIFIED_IN_H3FORGE (37/37, 507 spells) | spell lists per level (H3F-022); in-game load pending | - |
| HotA town types (Cove/Factory/Bulwark) | type bytes 9/10/11 | h3sed `TOWN_TYPES` | labels; type byte equals the source map faction on the sample | none | - |

### Players and map state

| Feature | Structure | Reference | Read | Write | Next |
|---|---|---|---|---|---|
| Resources (7×u32 per player) | player-state block | h3svg spec; vlucas homm3-savegame-editor documents a writable classic layout (license/offsets to re-verify) | UNKNOWN | none | H3F-018 |
| Player ownership / color | +0 in each of eight player records | h3svg spec; hero type and town ID owner cross-checks | VERIFIED_IN_H3FORGE for 44/11 | identity stays read-only | H3F-018 |
| Hero movement on map | hero record + map object entry + town references | h3sed treats profile location as heuristic | UNKNOWN | closed | H3F-019 |

### H3F-007/008 implementation evidence (2026-11-08)

- **Write pipeline** (`h3forge-save`): `BytePatch { offset, expected, replacement, reason }` - equal-length, preimage-checked, overlap-rejected, all-or-nothing; `compress_gzip` emits a valid single member (MTIME 0, DEFLATE 6, correct CRC-32/ISIZE), matching h3sed's full-re-gzip strategy. No-op edits plan zero patches.
- **Sample round trip**: the owned spellbook-bearing candidate at span 4,727,078 (attack 3→9, learn Air Shield, unlearn Visions) reopens with a payload diff of **exactly** the 5 planned offsets and nothing else (`tests/sample_edit_roundtrip.rs`).
- **Save As** (`h3forge-desktop`): source fingerprint check → plan against the immutable payload → verified backup `<dest>.backup-<millis>` (original compressed bytes, read-back verified) → synced temp file → strict-reader reopen with exact payload compare → no-clobber rename. Repository `games/`/`Data/` and the open source are hard-rejected destinations. Failure paths (`file-exists`, `source-changed`, `nothing-to-save`, `same-file`, `forbidden-destination`, write/backup/validation failures) preserve edits and originals (`tests/edit_flow.rs`).
- **Pending gate:** the modified file has not been loaded by the actual game; parser round-trip is explicitly not a substitute (maintainer acceptance step).

## mmarch Rust reuse - verified

`mmarch-rust/Cargo.toml` defines a Rust 2021 CLI package with `miniz_oxide 0.7` and `rayon 1`. There is `src/main.rs` and no `src/lib.rs`. Relevant modules:

- `archive.rs`: `ArchiveEntry`, `ArchiveKind`, broad `Archive` trait including mutation/extraction concerns.
- `lod.rs:144`: `LodArchive::load()` currently reads the **entire archive** with `fs::read`. `load_h3()` reads a 92-byte header and 32-byte directory entries (16-byte name, offset, unpacked size, unknown word, packed size).
- `lod.rs:1057` onward: `read_entry_data()` seeks an individual entry through `read_raw_entry`; packed H3 entries use bounded zlib decompression. It automatically converts PCX to BMP when decoding succeeds. The archive API should instead return raw decompressed entry bytes; decoding should be explicit.
- `lod.rs:1444`: `zlib_decompress_bounded()` supplies useful safeguards and expected-length behavior. Adapt and test its semantics for H3 assets rather than replace it gratuitously.
- `image.rs:382`: Heroes PCX decoding, with a 12-byte header and indexed pixels plus 768-byte RGB palette, or 24-bit pixels. Output is BMP. `read_bmp()`/`Indexed` can inform a direct RGBA conversion.
- `image.rs` also supports Might and Magic bitmap/sprite formats and palette fixups. These are **not** a Heroes III DEF decoder. No Heroes III DEF frame decoder was found in the Rust source.

Adapt the relevant MIT-licensed H3 reading and PCX code into `h3forge-resources`, one internal workspace library. Retain provenance, keep the archive-writing/CLI/MM6–8 machinery out, replace whole-file loading with header/table reads, add checked indices/ranges and explicit allocation limits. The existing code can index `entries[index]` directly and can stop on a short directory instead of rejecting the claimed count: strengthen those boundaries. This is a focused extraction/adaptation, not a new archive implementation or a dependency on a sibling absolute path.

### Actual local resources

| Archive | Bytes | Directory observation |
|---|---:|---|
| `H3bitmap.lod` | 109,587,195 | Standard version 200; 4,912 entries, 4,809 PCX; portraits include `HPL000KN.PCX`, other HPL families |
| `H3sprite.lod` | 65,352,134 | Standard version 200; 4,013 entries: 2,565 DEF, 1,448 MSK |
| `HotA.lod` | 111,378,616 | Header version 200/count 5,232, but standard directory decoding gives non-text names and implausible ranges |
| `HotA_lng.lod` | 9,061,660 | Standard version 200; 225 entries including PCX, DEF, text and campaigns |
| `HotA_ext.lod`, `HotA_l_ext.lod` | 92 each | Valid-looking empty version-200 LOD headers, zero entries |
| `h3ab_bmp.lod`, `h3ab_spr.lod` | 0 each | Empty files, not valid archives; ignore with diagnostics |

Selected `H3sprite.lod` entries were individually read/decompressed and only their metadata inspected: `Artifact.def` (194,554 bytes, canvas 44x44), `CPRSMALL.def` (152,893, 32x32), `SpellInt.def` (158,256, 48x36), `TwCrPort.def` (624,230, 58x64). Each starts with type `0x47`, dimensions and one group. Frame-to-domain-ID mappings and transparency are not established by names alone.

**HotA directory gap:** first 16 header bytes are `4c4f4400c80000007014000044d7a4b5`; first directory record begins `9f18546e1859a6b5d291a4b59ec5a4b5`. Standard mmarch name/offset decoding cannot safely use it. An encoded/obfuscated variant is a hypothesis, not a verified algorithm; corruption or another layout has not been excluded. No matching HotA transformation was found in the inspected Rust code. Do not report its claimed count as successfully enumerated resources. Issue H3F-014 bounds the follow-up; base archives and placeholders keep editing usable.

### H3F-005 implementation evidence (2026-11-08): hero identity and portraits

- **Identity field found.** The runtime hero record carries `hero_type` (offset 10), `hero_class` (11), `hero_portrait` (12) per the h3svg specification (`Hero` struct, alexanderbelous/h3mtxt `h3svg` branch). Byte-level validation on the HotA 44/11 sample: surrounding fields align exactly (patrol/morale/luck bytes, `-1` spell levels at 5/6, destination `-1` words at 19–27, movement at 31/35, experience at 39, spell points at 47, level at 49); the class byte's 215-value histogram matches exactly the 24 known hero classes (16 SoD + HotA Captain/Navigator/Mercenary/Artificer/Planeswalker/Elementalist + Random + Placeholder); unchanged CP1251 roster names sit at type-consistent classes (type 0 `ce f0 f0 e8 ed` = Оррин/Orrin Knight, type 8 Рион/Rion Cleric, type 16 Мефала/Mephala Ranger); renamed heroes keep their type bytes (record 1 "AbsoluteZero" type 1, record 4 "J. Lannister" type 4, record 28 "Ben Kenoby" type 28). Earlier research's "no verified identity-to-portrait field" is superseded for the HotA layout; records 10/11/12 are identity, not unknown padding.
- **Portraits:** the game displays `hero_portrait` (== `hero_type` unless customized). The binary references `HPL<number><class>.PCX` (strings table, up to `HPL136Wi` plus special-class variants); `H3bitmap.lod` carries 000–136 in three families (`HPL` 58×64 large, `HPS`/`PS` small), entries 130–136 under mixed-case names (`HPL130Kn.pcx`) - case-insensitive name matching is required. The `HPS`/`PS` entries are separate artwork with different aspect ratios (e.g. 1.5), not scaled copies of the large portrait; `hpsrand.pcx`-style non-numbered names exist and are not hero portraits. Sample outcome: 131/215 portraits resolve from the readable archive; 84 (all 68 customized + 16 HotA defaults, max ID 244) are unavailable - likely in the unsupported `HotA.lod`; placeholders are shown with the reason. H3F-006+ can resolve assets the same way; H3F-014 still owns the HotA directory.

### Durable format references (verified 2026-11-08)

- **h3svg (savegame) spec:** `https://github.com/alexanderbelous/h3mtxt` branch `h3svg`, `doc/h3svg_specification.MD` - full `SavedGame` layout: header, map sections, `Player[8]` (hero slots reference `HeroType` directly), `Town[]`, runtime `Hero[156]` array, `hero_owner`/`hero_can_be_hired_by` tables indexed by `HeroType`, tiles/objects with `object_idx`. The spec targets SoD/early-HotA save versions; every field H3Forge uses was re-validated against the 44/11 sample before adoption (the 42/2 layout was never assumed compatible). Companion `doc/h3m_specification.MD` (master branch) documents maps: `HeroType` (hero identity, 156 values), `HeroPortrait` (display portrait), `CustomHero` (custom name+portrait+hireability), hero objects carrying `HeroType` as identity.
- **Runtime `Hero` struct** (offsets relative to the h3sed record start = `owner` byte): owner 0, patrol_radius 1, temp morale/luck 2–3, backpack_count 4, disguise/fly/water-walk/visions levels 5–9 (`-1` = 0xFF), **hero_type 10, hero_class 11, hero_portrait 12**, patrol x/y 13–14 (0xFF none), orientation 15, formation 16, level-up seed 17, destination x/y/z 19–27 (`-1`), move max/current 31/35, experience 39, skill count 43, spell points 47, level 49, visited-bonus bitmask block 50–112, army 113/141, name 169 (13 bytes), skill levels 182, primary 238–241, spells 242/312, equipment 382 (19×8), backpack 534 (64×8), combo locks 1046–1055, HotA skill slots 1092.
- **Save hero array is not the roster:** 215 records in the sample exceed the 156 `HeroType` values; records with class bytes 22/23 are Random Hero / Crossover Placeholder expansions, and HotA extends hero types past 156. `hero_portrait` may exceed the physical `HPL###` files (IDs up to 244 observed; files only 000–136 + special non-numbered smalls like `HPS000SH`/`hpsrand`).
- **Real installation cross-check** (`G:\Games\Heroes3_HotA`, recorded in AGENTS.local.md): `Data/HotA.lod` header/size identical to the workspace copy - the nonstandard HotA directory is a genuine game-version artifact; `h3ab_bmp.lod`/`h3ab_spr.lod` are 0 bytes in a real install too; the workspace `games/` saves come from that install's `games/` folder; stock `.h3m` maps are available under `Maps/` for future identity cross-validation.

### H3F-004 implementation evidence (2026-11-08)

- `h3forge-resources` now opens standard H3 LODs metadata-only (header + directory table; no payload reads), enumerates entries, resolves names case-insensitively (duplicate names rejected) and reads individual raw/zlib-packed entries with bounded decompression. Verified read-only against the supplied archives: `H3bitmap.lod` (4,912 entries), `H3sprite.lod`, `HotA_lng.lod` (225 entries) all enumerate; `HotA_ext.lod` opens with zero entries; zero-byte `h3ab_bmp.lod` is rejected `EmptyArchive`.
- Heroes PCX decoding verified on real entries: `HPL000KN.PCX` is 58x64 indexed (unpacked 4,492 = 12 + 3,712 + 768 palette, first palette entry cyan 0,255,255) and `HotA_lng.lod` `CAMPBACK.pcx` is 800x600 24-bit (unpacked 1,440,012 = 12 + 800·600·3; B,G,R triplets confirmed by pinned pixel values). `H3sprite.lod` entries remain raw DEF bytes; no DEF decoding in this issue.
- The supplied `HotA.lod` is rejected with `UnsupportedDirectory` ("entry name is not printable ASCII" on the first directory record): its claimed 5,232 entries are never enumerated. The encoded/obfuscated-variant hypothesis stands unchanged for H3F-014; H3F-004's only addition is that rejection is now enforced by strict directory validation rather than assumed.

## FlightTrace - verified and reusable choices

`Cargo.toml` separates core/storage/API/CLI crates and deliberately excludes its desktop shell for Linux GUI dependency reasons. `apps/flighttrace-desktop/src-tauri/Cargo.toml` uses **Tauri 2**, and `src/main.rs` runs an Axum server on a dynamic loopback port, then opens a webview on it. This is not the direct-command integration requested for H3Forge.

`web/package.json` uses React 18, TypeScript, Vite, React Router and ESLint; `web/src/{api,types,components,pages}` and `styles/{tokens,globals}.css` are useful organization references. Reuse typed boundaries, design tokens, small components, consistent error/loading states, and Rust library separation. Do not inherit its HTTP server, database, telemetry-specific crate count, Linux environment changes, old dependency ranges, or Tauri 2 features. H3Forge can include the desktop crate in one workspace with core libraries as default members.

## Toolchain and official API evidence

M0 implementation update: external dependency installation selected `tauri-runtime 3.0.0-alpha.5` through tauri/Wry alpha.5's `~3.0.0-alpha.4` requirements. Compilation failed at `on_webview_event` and `on_window_event`. Cached source and the [runtime alpha.5 release](https://github.com/tauri-apps/tauri/releases/tag/tauri-runtime-v3.0.0-alpha.5) confirm those methods were removed. H3Forge now explicitly constrains runtime to `=3.0.0-alpha.4`; Cargo.lock was regenerated by Cargo with that downgrade. The user subsequently downloaded the corrected package and confirmed the frontend production build, `cargo check --workspace --locked`, Windows native launch and the connected `get_app_info` response display all passed. H3F-001 is completed.

Rust 1.99.1 is an official release and exceeds the Tauri v3 manifest's Rust 1.95 minimum. Pin the requested version in M0. [Rust announcement](https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/)

The official v3 migration guide specifies a separate runtime dependency, explicit runtime selection, and v3-compatible plugins. Its examples are not a promise that every package shares one alpha number. The site still has some Tauri 2 branding; its migration text also says migration is not automated, while newer CLI release notes describe automation. Prefer version-tagged source for pinning. [Migration guide](https://v3.tauri.app/start/migrate/from-tauri-2/), [CLI alpha.4 release](https://github.com/tauri-apps/tauri/releases/tag/@tauri-apps%2Fcli-v3.0.0-alpha.4)

The `tauri-v3.0.0-alpha.5` workspace manifest lists Tauri alpha.5, Wry runtime alpha.5, build/macros/codegen/utils/runtime alpha.4; its API package manifest is alpha.3. This is the recommended same-snapshot baseline, not a claim of latest versions or a successfully resolved local build. [Tagged workspace manifest](https://raw.githubusercontent.com/tauri-apps/tauri/tauri-v3.0.0-alpha.5/Cargo.toml), [API manifest](https://raw.githubusercontent.com/tauri-apps/tauri/tauri-v3.0.0-alpha.5/packages/api/package.json), [Tauri release](https://github.com/tauri-apps/tauri/releases/tag/tauri-v3.0.0-alpha.5), [Wry release](https://github.com/tauri-apps/tauri/releases/tag/tauri-runtime-wry-v3.0.0-alpha.5)

Official releases also establish CLI alpha.4 and API alpha.3 availability. Direct registry requests from the shell failed due to network access; no packages were installed/resolved. M0 must verify published package availability, one compatible graph, and Windows launch before declaring foundation complete. Dialog's exact compatible package pin was not established; use a Rust-owned native picker and resolve that dependency in M0 rather than guess a plugin version. [API release](https://github.com/tauri-apps/tauri/releases/tag/@tauri-apps%2Fapi-v3.0.0-alpha.3), [CLI release](https://github.com/tauri-apps/tauri/releases/tag/tauri-cli-v3.0.0-alpha.4)

## Licensing and attribution

Local `h3sed_4.1/LICENSE.md` is MIT, copyright 2020 Erki Suurjaak. `mmarch/LICENSE` is MIT, copyright Tom Chen; the Rust manifest declares MIT. Preserve complete copyright/permission notices with copied or translated source and in distributed notices. M0 establishes `THIRD_PARTY_NOTICES.md`; code-adoption issues add `licenses/h3sed-MIT.txt`, `licenses/mmarch-MIT.txt`, source paths, snapshot IDs/hashes and a brief adaptation log. These are planned files, not created source copies in this session.

FlightTrace is MIT OR Apache-2.0; architectural inspiration alone needs no code copying. If copying source, preserve its selected license notices too. Do not infer game-art redistribution permission from any editor/tool license. Do not vendor embedded H3SED images, mmarch real-game fixtures, or the supplied archives. Prefer an MIT license for new H3Forge code, finalized in M0. Any future third-party DEF decoder needs its own license review before incorporation.

## Open questions and bounded next steps

1. Validate minor-11 edit ranges using the sample copy, minimal byte diffs and an actual matching-game load (H3F-007/008). No blanket future-HotA support.
2. Establish hero identity/portrait references and ID-to-icon frame mappings; renamed and custom heroes must not be guessed (H3F-005/006).
3. Identify the supplied HotA archive variant or explicitly leave it unsupported (H3F-014). No entire-archive extraction or startup decoding.
4. Validate town candidate spans and garrison relationships before writes (H3F-013).
5. Confirm further version fixtures only when widening support (H3F-015). Existing sample siblings are not an exhaustive matrix.


## H3F-009 army edit evidence (2026-11-09)

- Verified against the local MIT h3sed 4.1 source: `metadata.py` creature IDs and `army.count` range 1..u32::MAX; `version/hota.py` additions and Bulwark minor threshold 10; `common/army.py` parse/serialize empty-slot handling. Rust now contains 189 creature choices for exact 44/11 only. Unknown stacks and other formats cannot receive army writes.
- `is_readonly()` in that shared army plugin blocks **town** edits when a garrison hero owns the army; it does not prohibit the hero's own army edits. No town parsing or linkage writes were introduced. The reference permits an empty army (0–7 stacks); a last-stack prohibition has not been established.
- One safe session Save As round trip from a private SaAndVla copy (215 heroes) verified replacement/clearing, backup/source identity, discard/revert and a full payload comparison against the intended patches. Core tests independently constrain changes to the selected type/count words and preserve zero/FF empty forms. Army composition is now user-verified in HotA; extreme-quantity gameplay and unlisted variants remain unverified.

## H3F-006 spell icon evidence (2026-11-09)

- **Correction to the spell-name table.** h3sed `metadata.SPELLS` is an *alphabetical display list*; the flag-array index for a spell name comes from h3sed's separate `metadata.IDS` table (`hero/spells.py:parse` builds `{name: ids[name]}`). Porting `SPELLS` as an ID table mislabeled every spell in H3Forge. Fixed: `definitions.rs::names::SPELL_NAMES` is now indexed by game spell ID (0 Summon Boat … 69 Summon Air Elemental), identical to h3sed `IDS` values and the h3mtxt `SpellType` enum; all 70 slots named. Sample corroboration: the candidate's learned IDs `[3,15,17,20,25,33,35,37,42,43,44,47,64]` now read as sensible low-level spells (View Earth, Magic Arrow, Lightning Bolt, Frost Ring, Destroy Undead, Protection from Water, Dispel, Cure, Curse, Bloodlust, Precision, Disrupting Ray, Remove Obstacle).
- **Spell-ID → icon mapping is positional.** `H3sprite.lod/spells.def` has 70 frames, canvas 78×65, array index = game spell ID. Evidence: VCMI's spellbook renders `CAnimImage("Spells")` with `setFrame(spell->id.getNum())` (`client/windows/CSpellWindow.cpp:862`), and `Mods/vcmi/Content/Sprites/spells.json` documents the one known original-data art bug (View Earth ID 3 / View Air ID 5 pictures drawn swapped; H3Forge reproduces the original data as-is). Frame labels follow the same order with rename trivia (`Sp03Iden` = Visions' icon, development name "Identify", eye artwork visually confirmed; `Sp37Back` = Magic Mirror; `Sp52Wind`/`Sp53Muck` = Haste/Slow); several labels are author mislabels (e.g. Destroy Undead art at index 18, Implosion's slot) - positions govern, names do not. `SpellInt.def`/`SpellScr.def`/`SpellBon.def` mirror the order with a leading null frame (SpellInt) and trailing creature-ability frames matching the h3mtxt commented IDs 70–77.
- **Implementation:** `h3forge-resources/src/spell_icons.rs` (positional `DefFile::decode_index`, 78×65 assertion, per-ID PNG cache, IDs outside 0–69 → placeholder), desktop `get_spell_icon` command with the same SourceIdentity invalidation as creature art, frontend `SpellIcon` next to every spell name in the Spells tab (checklist + read-only lists). Real-archive tests: 70/70 IDs decode to 78×65 PNGs, all 70 frames carry transparent index-0 canvas padding, fallback verified for IDs ≥ 70. Existing creature portraits, spell editing, and dirty state unchanged.

## H3F-010 skill editing evidence (2026-11-09)

- Skill ID table verified against h3sed `metadata.IDS` (0x00 Pathfinding .. 0x1B First Aid + HotA 0x1C Interference, 0x1D Runes) - the ported 30-entry table was already ID-ordered; the alphabetical display list was not used as IDs (H3F-006 pattern). Semantics ported from `hero/skills.py` (presence = mastery && slot && slot <= count; serialize = dense one-based renumber + count write) and `version/hota.py` (`skills_slot: 1092`).
- `HeroEdit::SetSkills` (absolute ordered list) plans only count (43 low byte), mastery (182+id) and order bytes (1092+id); removals zero both bytes of the removed skill; unrecognized entries (zero slot/mastery, slot > count) and the legacy 212..238 region are preserved. Capacity 28 (0x1C) though 30 IDs exist; exact-44/11 1122-byte records only, duplicates/mastery-outside-1-3/unknown IDs rejected.
- Sample round trip on a private copy (candidate @4,727,078): reorder + Wisdom→Expert + add Logistics produced 7 patches, reopened parse shows exactly `[(Mysticism,1,1), (Intelligence,2,2), (Wisdom,3,3), (Logistics,1,4)]`, payload diff confined to the planned offsets, source untouched. User-reported HotA results now verify replacement of an existing skill and addition of another. Reordering/mastery changes and this exact fixture remain without manual confirmation.

### H3F-011 backpack evidence (2026-11-09)

Verified all 158 literal artifact mappings against H3SED binary IDS in
metadata.py and version/hota.py. Display ordering is not an ID source.
Writable set is deliberately narrower: ID 2 and 7–126, scrolls ID 1 with
corrected spell IDs 0–69. Expansion/combination items remain read-only.
`hero/inventory.py` parse checks the first word for FF; serialize writes
BLANK*4 + NULL*4 for removal, despite a reversed nearby hex comment.
Zero artifact IDs remain ambiguous/read-only in H3Forge. No count-byte
normalization is performed by the upstream inventory serializer.
See SAVEGAME_FORMAT.md C.5 and H3F-011 for the verified eight-byte encoding,
sentinel rules and successful application-level sample round trip. Artifact
icons and equipment were subsequently implemented (H3F-012 follow-ups).
Artifact addition now has a user-reported HotA pass, with placement unspecified;
scrolls, removal/replacement and equipment coupling remain manually unverified.


HotA skill icons follow-up: both local resource sets contain only the same 28 base skill triplets in readable SECSK DEFs. Interference/Runes artwork remains unverified behind the unsupported HotA.lod directory; no readable override found. Detailed directory/frame evidence and the H3F-014 dependency are recorded in GAME_RESOURCES_FORMAT.md, "HotA skill icon investigation".


### H3F-014 archive access resolved; skill identity still open

The homm3data reference's XOR layout matches both local HotA.lod files:
5232 validated opaque-ID entries, 3869 zlib/1363 stored, directory 80 and
key B5A4D744. Minimal bounded Rust support and a real DEF-to-PNG test replace
the former unsupported-directory conclusion. LZMA remains external-only and
returns a typed unsupported error. VCMI's named skill PNG paths do not yet
map to local opaque IDs, so all six Interference/Runes mastery icons remain
placeholders. H3F-014 and GAME_RESOURCES_FORMAT.md contain reproducible
evidence, read-only metadata probe scope and exact missing mapping.


### H3F-014 skill icons resolved by content identification

Supersedes "archive access resolved; skill identity still open". Filename-hash
guessing was abandoned; a bounded read-only scan decoded candidate 32x32
image containers and compared pixels with reference artwork from the VCMI
HotA mod repositories (identification only - nothing downloaded into the repo).
HotA.lod entry 48043b9164e06a6f8e626a6b6ca7caa1 is a 93-frame "D32F"
container (layout reverse-engineered locally, documented in
GAME_RESOURCES_FORMAT.md B.2). Frames 87/88/89 decode pixel-exactly (RGBA
SHA-256) to the verified Interference Basic/Advanced/Expert references;
frames 90/91/92 match the Runes references by identical alpha silhouettes and
per-frame minimum RGB distance on the correct diagonal (the published Runes
references are re-brightened re-encodes). Frame numbers continue the base
secsk32 formula 2 + mastery + 3 * ID. Verified fact: all six frames are fully
opaque (drawn square backgrounds), unlike secsk32.def palette transparency.
`HotASkillIcons` serves ids 28/29 from HotA.lod through the existing
skill_icon command; exact PNG bytes are CRC-32 pinned in
crates/h3forge-resources/tests/hota_skill_icons.rs. All six icons load from
the installed game. LZMA stays unsupported (no local fixture).


### HotA creature portraits resolved; hero custom-portrait gap closed (2026-11-10)

Creature artwork: HotA.lod D32F container `eb1f19b13b23e32ded0fd482e7266b54`
(202 frames, 58×64) is the TwCrPort successor. Creature ID `i` sits at frame
`i + 2`, established by content matching against all 145 base portraits: 94
pixel-exact (RGBA CRC equal), 36 within small channel distance, 11 same
silhouette with HotA recolored "portrait fix" art; base-blank slots (war
machines, reserved IDs) hold named filler frames exactly at `id + 2`, and
frames 0/1 are fillers. `HotACreaturePortraits` selects by index with exact
embedded-name verification (`twcrportA{id+2:03}`); 141 old + all HotA-exclusive
IDs (Oceanid 154 … Jotunn Warlord 199) now resolve; exact PNG bytes pinned by
CRC-32 in `tests/hota_creature_portraits.rs`. `CPRSMALL` (32×32) is the small
sibling container but its frame order is unverified and it is not used.
HotA.lod has no hero portraits at all: all 74 D32F containers enumerated.

Hero custom portraits: the shipped set is exactly HPL/HPS 000-136. Save
payload, installed loose files, and the gitignored reference map
`Maps/[HotA] The Mysterious Island - v1-5-j - ALLIES.h3m` (internal name
"- The Mysterious Island - COOP", matches the save) were all scanned - no
embedded or loose 137+ portraits. IDs 137-244 are map-distributed custom
portraits absent from this machine; the 84 missing hero portraits remain
placeholders with that explicit reason (session message updated).

### H3F-021: map link and the portrait table correction (2026-11-10)

Corrects "Hero custom portraits: the shipped set is exactly HPL/HPS 000-136"
above. Verified facts: a portrait ID indexes a name table (0-162 from the
game executable, 163-244 from `HotA.dat` record `newport0`); all 245 files
exist locally (`H3bitmap.lod` by name, `HotA.lod` by pinned entry ID, 54 of
the HotA ones as `P32F`); the sample's 84 placeholders are gone. `.h3m`
files carry portrait IDs only. Design choice: HotA entries are addressed by
entry ID because the name → ID function is still unknown. Unresolved:
that function; small HotA portraits; whether map hero positions equal save
coordinates (H3F-019). Details: `docs/issues/021-map-discovery.md`,
GAME_RESOURCES_FORMAT.md section F, MAP_FORMAT.md.

### H3F-012 visual artifacts follow-up (2026-11-10)

Verified 163 actual 44x44 icons using original local archives, without bundling:
141 base HrArt names plus 22 HotA D32F names. Crucially, base HrArt141–143 are
unavailable tiles, and HotA's artifactA numbering differs from IDs for many base
items. Exact mapping and excluded overlays are recorded in GAME_RESOURCES_FORMAT.
The frontend now uses compact equipment squares, a fixed 64-position backpack,
and a shared searchable lazy-image picker. Checked pending patches also drive the
primary-stat preview (the old response omitted equipment bonuses until saving).
Unknown equipment/reservations still block all equipment edits: independent coupled
effects were not proven. Diagnostics give precise item/slot and reservation bytes;
backpack moves remain atomic and preserve opaque data. H3F-013 remains untouched.

### H3F-018 evidence (2026-11-10)

- Eight 149-byte player records at sample payload 4,634,994, anchored before
  the town count. Resources are signed i32 at +98 in standard resource-ID
  order. Red, Orange, Teal and Pink have verified heroes/towns; the other
  four slots are inactive/unused and remain read-only. No human/AI or
  eliminated/never-used distinction is claimed.
- The classic h3svg field offsets align through the resources and visited
  masks, but its 145-byte stride does not. Unknown HotA tail bytes survive.
  All listed hero types and all 33 owned town IDs cross-check against the
  existing parsers. Details and values: SAVEGAME_FORMAT.md section E.
- Correction: current vlucas source/tree has no player-resource implementation
  or license file; earlier notes overstated that reference. No code reused.
- Seven original resource icons resolved and visually inspected in
  H3sprite.lod/RESOURCE.def (rsrc0.pcx..rsrc6.pcx). Runtime Rust decoding and
  PNG caching only; no proprietary assets added to source.
- 28 individual resource round trips and a two-player six-byte payload diff
  pass. Session Save/Save As, backups, Revert/Discard and source-change guard
  pass. Matching-game loading and native UI interaction remain manual gates.

### H3F-023 evidence (2026-11-10)

Implemented authored H3M player eligibility, team/allies/enemies, starting
town/hero details, original banners and read-only obelisk locations/visits.
Current human/AI controller and current GM2 alliances remain unverified.
Sample: 29 H3M obelisks and one authored Grail; GM2's count + 48 visit-mask
array immediately before Player[8] agrees with that count. Visit totals are
`[0,5,15,15,17,5,17,1]`; team pairs have equal visit bits. No object-index
mapping or complete puzzle update contract is established.

Both reveal operations remain disabled. Classic fog structure probing leaves
32 alignments that all pass the owned-town visibility cross-check; no byte
range was promoted to writable. VCMI illustrates team/counter coupling only.
Exact probes and offsets: SAVEGAME_FORMAT.md, H3F-023 section. H3M additions
and banner mapping: MAP_FORMAT.md and GAME_RESOURCES_FORMAT.md.

Private-copy session test verifies eight banners, two fixed town icons, two
identified starting portraits, all teams/eligibility, read-only visit totals,
resource Revert/Discard, and a Save As/reopen with exactly three changed
resource bytes. No H3M or exploration byte changes. Issue remains partial
with explicit revelation blockers; no unrelated issue started.


## H3F-025 theme reference (2026-11-11)

Verified FlightTrace provides implemented warm parchment/charcoal CSS palettes and light/dark/system preferences, with localStorage and an early startup script. H3Forge adapts those colors and behavior, retains the MIT notice, and uses an external synchronous script to satisfy its existing CSP. Design choices: keep compact Segoe UI typography, deepen light-mode text colors, localize a Settings selector. Automated build/storage/render checks pass; native visual/relaunch behavior remains unverified. See [H3F-025](issues/025-light-dark-themes.md).

## H3F-017 portable Windows release (2026-11-11)

Verified the pinned Tauri 3 CLI's `build --no-bundle --target
x86_64-pc-windows-msvc -- --locked` embeds the production frontend in one EXE.
Inspection matches all five Brotli asset payloads and all six configured ICO
frames; PE imports require Windows system/UCRT libraries, not a loose
WebView2Loader, Node or Python DLL. WebView2 Runtime remains a prerequisite.
Rust sysroot diagnostics initially exposed an absolute user path; remapping the
sysroot along with checkout/Cargo paths removes it. The development fixture guard
now discovers the checkout relative to the EXE, preserving local release safety
without embedding a source path. A focused read-only guard test passes.
FlightTrace scripts/workflows were inspected read-only for the version/tag flow;
its installers and runtime setup were not adopted. Clean-machine, visual,
placement and release-artifact checks remain unresolved. Feature-level HotA
passes are now recorded separately in the manual compatibility checklist. Full results and artifact
hash: [H3F-017](issues/017-windows-release.md).
## H3F-027 - current neutral armies (2026-11-11)

Reviewed H3save_reader's tile scanner, monster aggregation and GM1
compendium before extending current-save inspection. Its GM1 packed monster
quantity formula is not verified for HotA 44/11 and is deliberately unused.
The existing h3svg save specification plus independent sample structure
checks establish a 61-byte map-garrison record and complete 47-entry index
chain. A copied SaAndVla.GM2 has 291 current wandering groups and 47 garrisons
(28 occupied, 19 empty). H3M cross-check: 280 fixed monster identities and
11 originally random placements; only 29 of 47 garrison armies are unchanged.
Garrison type/count edits are exact-byte and save/reopen verified.
The later native survivor pair establishes wandering current quantities:
object 13926 / creature 60 at 62/53/0 references 33-byte record 517, whose
u32 quantity changes from the confirmed 72 to 56. The aligned tail shifts
303 bytes due to replay growth and otherwise remains unchanged. The earlier
defeat pair establishes disappearance only (1,186 -> 1,185 groups); its
unchanged early tables do not include this late monster table.

All 291 original sample groups now have current counts. Type changes reuse
existing unambiguous templates with identical fixed bytes except subtype,
patching tile subtype and object template index without modifying shared
templates. Two-group exact-diff/save-reopen and desktop persistence tests
cover type/count edits. No zeroing/removal or unsupported guarded armies.
Native loading of edited copies and actual 1280x720 UI inspection remain
manual checks. The compact UI separates category counts and uses dense
rows and a two-column army editor.

See [H3F-027](issues/027-neutral-creatures.md) for sources and verification,
and [current map armies](formats/SAVEGAME_FORMAT.md#h3f-027-current-map-armies)
for verified offsets and conservative gates. No upstream code or artwork
was copied for this issue.

## H3F-019 movement evidence (2026-11-12)

The three supplied movement saves are independent before/land-step/water-step
branches. Exact decompressed diffs establish hero front-matter coordinates,
tile occupancy, facing, destination cancellation, movement remaining and an
unresolved movement-associated byte (+1069). Moandor's underlying class is 3,
and his save also changes Nimbus's route. Nimbus's boarded boat stays byte-identical
with stale coordinates; his step changes a visibility-like mask and adds a
matching replay event. Replay insertions are 55/67 bytes, with identical aligned
tails. No RNG change is established. Templates, objects, boats and towns stay
unchanged in both steps. These observations do not prove arbitrary relocation.

Follow-up analysis withdrew the repeated land-step request. The supplied pairs
already establish passive occupancy restoration, while facing/cost/fog/replay
are distinct native-step effects. H3API corroborates the concepts, not save
offsets. Current tile/render references distinguish active objects from stale
monster/resource entries. Arbitrary-distance same-level/medium relocation is
implemented for matching passive destination shapes, preserving movement points,
facing and exploration as explicit editor policy. Cross-level and attachment/
interaction transitions stay blocked. Towns remain read-only. Native game-load
validation remains manual. See
[H3F-019](issues/019-map-coordinates.md) and the
[complete byte evidence](formats/SAVEGAME_FORMAT.md#h3f-019-coordinate-movement-evidence).


### H3F-028 - Keymaster Tent evidence

Verified: the native Pink/Red pair changes the global red-tent mask `01 -> 81`
at 4552404; eight masks start at last verified hero end +12+430+109. All eight
player bits and eight color IDs are supported by the shared layout and current
save templates. Exact aligned diff: 47 changed bytes plus a 51-byte replay
insertion, with a 90,987-byte identical suffix. No fog change or separate tent
counter was found. Original artwork is `H3sprite.lod/AVXkey{0..7}0.def`.
Design: player-only bit edits, no simulated movement, replay, guard removal or
ally propagation. Native HotA reload/access checks are still manual. See
[the format evidence](formats/SAVEGAME_FORMAT.md#h3f-028---keymaster-tent-visits-hota-4411)
and [H3F-028](issues/028-keymaster-tents.md) for exact offsets, gates and tests.


### H3F-027 - Later-save tile-boundary ambiguity

Verified on the current SaAndVla and red-tent-before saves: a length-only tile
walk falsely accepted header offset 1,277 in the later file alongside real start
2,253. Existing render object/anchor/relative-byte relationships reject the false
walk; the complete real walk passes. Discovery now validates these relationships
while walking backwards, retaining unique-boundary and write gates. Later/current
counts are 469 wandering groups and 46 garrisons; earlier supplied counts are
1,119 and 46. Differences do not establish defeat/removal history. See
[H3F-027](issues/027-neutral-creatures.md) and
[SAVEGAME_FORMAT.md](formats/SAVEGAME_FORMAT.md) for evidence and regressions.
