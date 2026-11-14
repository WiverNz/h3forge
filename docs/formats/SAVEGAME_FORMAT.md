# Heroes III / HotA savegame format

Technical specification of the savegame formats H3Forge parses and edits.
Scope: what is actually implemented in `crates/h3forge-save` and
`apps/h3forge-desktop`, plus adjacent structures documented by external
references. This document is self-contained: every layout below is stated
directly, with a provenance appendix mapping claims to sources.

Confidence vocabulary used throughout:

| Class | Meaning |
|---|---|
| VERIFIED_IN_H3FORGE | Demonstrated by the H3Forge parser or focused tests against the local sample/archives. |
| VERIFIED_EXTERNAL | Supported by an external implementation/specification; not re-validated locally. |
| PARTIAL | Some parts understood; boundaries stated. |
| UNKNOWN | Insufficient evidence; nothing below should be treated as fact. |

Reference implementation (authoritative for offsets and semantics):
`crates/h3forge-save/src/` - `container.rs`, `detect.rs`, `parse.rs`,
`locate.rs`, `domain.rs`, `definitions.rs`, `edit.rs`, `patch.rs`.

Primary sample: `games/SaAndVla.GM2`, HotA 44/11 (see section H).

---

## A. General container structure

### A.1 File extensions

Saves are gzip members carrying an `H3SV*` payload. Observed extensions:

| Extension | Observation | Confidence |
|---|---|---|
| `.GM1` | Early-version save extension (RoE-era naming in external docs). | VERIFIED_EXTERNAL |
| `.GM2` | Standard save extension. Our sample `SaAndVla.GM2` is a **regular** `H3SVG` save - the extension does not distinguish regular/campaign or the game version. | VERIFIED_IN_H3FORGE |
| `.CGM` | Campaign save extension; payload magic is then `H3SVC`. | VERIFIED_EXTERNAL |
| `.GM3` | HotA HD-mod observed extension in external tooling; no local fixture. | UNKNOWN |

The extension is cosmetic for parsing: identification is by gzip magic and
payload magic only. VERIFIED_IN_H3FORGE.

### A.2 Gzip member layout

The whole file is one gzip member (RFC 1952). H3Forge validates the container
with a narrow native HotA CRC exception on import (`container.rs::decode`).
Editor-generated output is reopened with mandatory CRC validation
(`SaveDocument::open_strict`). All multi-byte fields are little-endian.

| Offset | Size | Field | Meaning |
|---|---:|---|---|
| 0 | 2 | ID1, ID2 | `1F 8B`. Mismatch → `NotGzip`. |
| 2 | 1 | CM | Compression method; must be 8 (DEFLATE). |
| 3 | 1 | FLG | Bit flags: 0x01 FTEXT (ignored), 0x02 FHCRC, 0x04 FEXTRA, 0x08 FNAME, 0x10 FCOMMENT, 0xE0 reserved (rejected). |
| 4 | 4 | MTIME | Unix time; ignored on read, written as 0. |
| 8 | 1 | XFL | Extra flags; ignored. |
| 9 | 1 | OS | Originating OS; ignored on read, written as 0xFF (unknown). |
| 10 | * | Optional fields | FEXTRA: u16 length + data. FNAME/FCOMMENT: NUL-terminated strings. FHCRC: u16 low bits of CRC-32 of the header so far (verified). |
| * | * | DEFLATE stream | Raw deflate payload (RFC 1951). |
| L−8 | 4 | CRC-32 | CRC-32 of the decompressed payload. Mismatch is a diagnostic only on regular H3SVG 44/11 import; otherwise `CrcMismatch`. Output verification always rejects a mismatch. |
| L−4 | 4 | ISIZE | Decompressed size modulo 2^32. Mismatch → `IsizeMismatch`. |

Behavioral rules (VERIFIED_IN_H3FORGE, from `container.rs`):

- Exactly one member. Trailing gzip magic → `MultiMemberGzip`; other trailing
  bytes → `TrailingData`. This is stricter than common gunzip tools.
- Decompression is bounded: `Limits::max_compressed` (default 64 MiB) and
  `max_payload` (default 256 MiB); the output buffer grows by doubling and
  never exceeds the cap.
- Native regular H3SVG 44/11 import accepts a trailer CRC mismatch only after
  complete bounded DEFLATE decoding, full trailer, matching ISIZE and exact
  single-member boundary checks. Header CRC, malformed streams, truncation,
  limits, trailing bytes and multiple members still fail. Other versions and
  campaign magic retain mandatory trailer CRC validation.
- This is a compatibility policy, not a way to distinguish all corruption
  from a native checksum defect. A payload bit change that preserves valid
  DEFLATE and size may be indistinguishable. A visible localized notice and
  `native-gzip-crc-mismatch` diagnostic expose this limitation and stored/
  computed values. Field-specific edit gates remain independent.
- Original compressed bytes are retained verbatim for backups. Output gets
  a correct CRC/ISIZE and is strictly reopened before replacement. The native
  import exception cannot be used to pass temporary-output verification.
- Evidence: the [h3svg compression specification](https://github.com/alexanderbelous/h3mtxt/blob/h3svg/doc/h3svg_specification.MD#compression)
  documents native invalid CRCs; H3SED's `metadata.py:1371` tolerates them on
  reads. Four supplied HotA 44/11 saves have complete streams and correct
  ISIZE but invalid CRCs. The bounded exception replaces H3Forge's previous
  unconditional rejection as part of H3F-030. No reference file is repaired.

### A.3 Compressed-file vs payload offsets

All offsets in sections B–G are **relative to the decompressed payload**
unless explicitly labeled "compressed file". The compressed layout is only
the gzip member above; nothing in H3Forge addresses bytes inside the deflate
stream.

### A.4 Payload magic and version

The decompressed payload starts with a 5–9 byte magic and u32 version fields:

| Payload offset | Size | Field | Meaning |
|---|---:|---|---|
| 0 | 5 | Magic | `H3SVG` regular save, `H3SVC` campaign save, `HCHRONSVG` Heroes Chronicles. Unknown → `UnknownFormat`. |
| 5 | 3 | - | Gap between magic and version fields (content not interpreted). |
| 8 | 4 | major | u32 LE. h3sed reads the single byte at offset 8; observed saves carry u32 LE fields whose low byte equals the h3sed value. |
| 12 | 4 | minor | u32 LE, same note. |

Detection (`detect.rs`) then assigns a family in release order - RoE, AB, SoD,
HC (magic-only), HotA - so AB `(42,1)` wins over the overlapping SoD
`(42..=43, 0..=4)` range:

| major | minor | Family | H3Forge compatibility |
|---|---|---|---|
| 16–41 | 0 | Restoration of Erathia | magic/version recognized read-only; no hero layout |
| 42 | 1 | Armageddon's Blade | recognized read-only; no hero layout |
| 42–43 | 0–4 | Shadow of Death | recognized read-only; no hero layout |
| any | any | `HCHRONSVG` magic | Heroes Chronicles, recognized read-only; no hero layout |
| ≥44 | ≥5 | Horn of the Abyss | **hero layout + editing gated to exact 44/11** |

Known-magic with unknown version bytes yields `Compatibility::Unsupported`
(the map text header remains parseable); unknown magic is an error.
ENDIANNESS: everything in the payload is little-endian (h3sed `bytoi`);
the h3sed header comment claiming big-endian text lengths is wrong.
VERIFIED_IN_H3FORGE.

HotA note: h3sed's HotA predicate is open-ended (`major ≥ 44, minor ≥ 5`).
H3Forge keeps that for recognition but gates all writes on
`major == 44 && minor == 11` - the only version with fixture evidence.
h3sed's `BULWARK_MINOR = 0x0A` marks HotA 1.80 (2025-12, adds Bulwark town and
Runes skill) and is the newest minor its tables know.

### A.5 String encoding

Hero/town/map names are legacy single-byte text. Storage is raw bytes
(NUL-padded fixed fields or length-prefixed variable fields); the bytes are
never rewritten by parsing. Display decoding is a frontend concern: strict
UTF-8 when valid, else Windows-1251 fallback for the Russian sample
(`web/src/lib/legacyText.ts`). The Rust core additionally offers lossy display
via ISO-8859-1 byte identity (`parse.rs::decode_legacy_text`). VERIFIED_IN_H3FORGE.

### A.6 Bounded parsing and corruption handling

Every read is bounds-checked; malformed input produces typed `SaveError`
variants (never panics). Structural limits: metadata text ≤ 1024 bytes,
hero scan bounded by payload length and a 5000-byte post-match window,
record-internal reads guarded by the located span.

---

## B. Savegame payload structure

The payload is only partially schema-mapped. Verified layout:

```
offset 0        magic + version u32s + header fields (B.1)
offset ~17+     map name / description (variable, length-prefixed)
offset 30,000+  hero scan region begins (skips campaign carry-over heroes)
...             towns: located before the hero array (section D, H3F-013)
offset 4,634,994 sample player table (8 x 149 bytes, section E)
...             other map objects: NOT located by H3Forge
offset ~4.63M   sample's town records (37 records, 568-581 bytes each)
offset ~4.65M   sample's hero records start (near the end of the 5.27M payload)
```

The sample's 215 hero records occupy 4,657,602-4,921,020+1122 - the hero
array sits close to the payload end, after the map/state body. H3Forge does
**not** assume fixed absolute offsets for records: variable-length fields
(map texts, scripting, object counts) shift everything after them, so records
are discovered by structural scan (section C.2). The sample's 37 town records
sit immediately before the hero array at 4,636,188-4,657,577 and are located
by the bounded town scan (section D). The player table is anchored immediately
before the town count (section E); other map objects remain unlocated.

### B.1 Map name and description (`parse.rs`)

h3sed's header regex finds the name within the first 2048 bytes: a greedy gap
of up to 100 bytes after the 5-byte magic, then a 4-byte field
`raw[q] != 0, raw[q+1..q+4] == 0` (the version bytes), then length-prefixed
texts at `q + 5`. Ported behavior:

| Step | Rule |
|---|---|
| Window | first `min(payload.len(), 2048)` bytes |
| Gap search | largest `k` in 0..=100 such that `q = k + 5`, `raw[q] != 0`, `raw[q+1..q+4] == 0`, and a valid name text follows at `q+5` |
| Text field | u16 LE length `L` (1..=1024), then `L` content bytes; content must not contain NUL |
| Sequence | name, then optionally description (same encoding); description failure is not an error |

Sample values: name `- The Mysterious Island - COOP` (30 bytes); the length
field sits at payload offset 95 and the text at 97. VERIFIED_IN_H3FORGE.

The four bytes the locator keys on are not version bytes: they are the map
header as copied from the `.h3m` (H3F-021, VERIFIED against the source map):

| Payload offset (sample) | Size | Field | Sample |
|---|---:|---|---|
| 16 | 12 | HotA release, three u32 LE | 1, 8, 1 (the map header carries the same three words) |
| `q` = 90 | 4 | map edge length, u32 LE | 252 |
| `q + 4` = 94 | 1 | has underground | 1 |
| `q + 5` = 95 | 2 + L | name (u16 length here, u32 in the map) | 30 bytes |
| after name | 2 + L | description | 295 bytes, byte-identical to the map's |
| after description | 1 | difficulty 0–4 | 3 |

`MapMetadata` exposes `size`, `has_underground` and `difficulty`; together
with the name and description bytes they are the key used to match a save
to its source map. The save stores no map file name or hash.

---

## C. Hero binary structure

### C.1 Record layout (HotA 44/11)

Record length is **1122 bytes** for HotA 44/10+ (30 trailing skill-slot bytes)
or **1121 bytes** for earlier HotA (29 bytes). Offsets are relative to the
record start = the owner byte. All integers little-endian.

| Offset | Size | Type | Field | Encoding / valid values | Confidence |
|---|---:|---|---|---|---|
| 0 | 1 | u8 | Owner | 0–7 player (Red, Blue, Tan, Green, Orange, Purple, Teal, Pink), 0xFF neutral | VERIFIED_IN_H3FORGE |
| 1 | 1 | u8 | Patrol radius | | VERIFIED_EXTERNAL |
| 2 | 1 | u8 | Temp morale | | VERIFIED_EXTERNAL |
| 3 | 1 | u8 | Temp luck | | VERIFIED_EXTERNAL |
| 4 | 1 | u8 | Backpack count | occupied backpack entries | VERIFIED_EXTERNAL |
| 5 | 1 | u8 | Disguise level | 0xFF = −1 (not active) | VERIFIED_EXTERNAL |
| 6 | 1 | u8 | Fly level | 0xFF = −1 | VERIFIED_EXTERNAL |
| 7 | 1 | u8 | Water walk level | 0xFF = −1 | VERIFIED_EXTERNAL |
| 8 | 1 | u8 | Visions level | 0xFF = −1 | VERIFIED_EXTERNAL |
| 9 | 1 | - | (padding/unknown) | | UNKNOWN |
| 10 | 1 | u8 | **HeroType** | hero identity; stable across renames; HotA extends past the 156 SoD values | VERIFIED_IN_H3FORGE |
| 11 | 1 | u8 | **HeroClass** | 0–23 observed (16 SoD + Captain, Navigator, Mercenary, Artificer, Planeswalker, Elementalist + Random + Placeholder) | VERIFIED_IN_H3FORGE |
| 12 | 1 | u8 | **HeroPortrait** | displayed portrait; equals HeroType unless the map customized it | VERIFIED_IN_H3FORGE |
| 13 | 2 | u8×2 | Patrol X/Y | 0xFF = none | VERIFIED_EXTERNAL |
| 15 | 1 | u8 | Orientation | | VERIFIED_EXTERNAL |
| 16 | 1 | u8 | Formation | | VERIFIED_EXTERNAL |
| 17 | 2 | - | Level-up seed | | VERIFIED_EXTERNAL |
| 19 | 12 | 3×u32 | Destination X/Y/Z | 0xFFFFFFFF = −1 (no destination) | VERIFIED_EXTERNAL (sample alignment verified: words at 19–27 are −1) |
| 31 | 4 | u32 | Movement total | | VERIFIED_IN_H3FORGE |
| 35 | 4 | u32 | Movement left | | VERIFIED_IN_H3FORGE |
| 39 | 4 | u32 | Experience | stored independently of level | VERIFIED_IN_H3FORGE |
| 43 | 4 | u32 | Secondary skill count | low byte 0..=0x1C; remaining 3 bytes zero | VERIFIED_IN_H3FORGE |
| 47 | 2 | u16 | Spell points remaining | 0–65535 | VERIFIED_IN_H3FORGE |
| 49 | 1 | u8 | Level | HotA write cap 74 (base game 75) | VERIFIED_IN_H3FORGE |
| 50 | 63 | bitmask | Visited-bonus bitmask block | | VERIFIED_EXTERNAL |
| 113 | 28 | 7×u32 | Army creature IDs | slot i at 113+4i; see C.4 | VERIFIED_IN_H3FORGE |
| 141 | 28 | 7×u32 | Army counts | slot i at 141+4i | VERIFIED_IN_H3FORGE |
| 169 | 13 | bytes | Hero name | content bytes ≥ 0x1A, NUL-padded; 13th byte must be 0 | VERIFIED_IN_H3FORGE |
| 182 | 30 | u8×30 | Skill mastery by ID | 0 absent, 1–3 Basic/Advanced/Expert | VERIFIED_IN_H3FORGE |
| 212 | 26 | bytes | Legacy skill-slot bytes | unused in HotA | VERIFIED_EXTERNAL |
| 238 | 1 | u8 | Attack | write range 0–99 (HotA game-valid) | VERIFIED_IN_H3FORGE |
| 239 | 1 | u8 | Defense | write range 0–99 | VERIFIED_IN_H3FORGE |
| 240 | 1 | u8 | Spell Power | write range 1–99 | VERIFIED_IN_H3FORGE |
| 241 | 1 | u8 | Knowledge | write range 1–99 | VERIFIED_IN_H3FORGE |
| 242 | 70 | u8×70 | Learned-spell flags | per spell ID, 0/1 (not packed) | VERIFIED_IN_H3FORGE |
| 312 | 70 | u8×70 | Available-spell flags | per spell ID, 0/1 | VERIFIED_IN_H3FORGE |
| 382 | 152 | 19×8 bytes | Equipment slots | see C.5 | VERIFIED_IN_H3FORGE (raw) |
| 534 | 512 | 64×8 bytes | Backpack inventory | see C.5 | VERIFIED_IN_H3FORGE (raw) |
| 1046 | 10 | bytes | Combination-artifact reservations | Byte 1046 observed 0; 1047–1055 are per-kind counters (helm, cloak, neck, weapon, shield, armor, hand, feet, side) of slots taken by worn combination artifacts. See the 2026-11-10 combination follow-up | VERIFIED_SAMPLE |
| 1056 | 36 | - | (mostly undocumented) | u32 at 1076 = land movement, u32 at 1080 = sea movement (section I.3); rest unknown | PARTIAL |
| 1092 | 30 (or 29) | u8×N | HotA skill-slot order | one-based slot per skill ID; 30 bytes 44/10+, 29 before | VERIFIED_IN_H3FORGE |

Notes:

- **Raw attributes are not effective stats**: overflow/wrap semantics and
  artifact bonuses apply at game level. H3Forge writes only game-valid ranges.
- The 70 spell flag slots are all named: `definitions.rs::names::SPELL_NAMES`
  is indexed by **game spell ID** (0 Summon Boat … 69 Summon Air Elemental),
  matching h3sed `metadata.IDS` and the h3mtxt `SpellType` enum. Do not use
  h3sed's alphabetical `metadata.SPELLS` display list as an ID table (H3F-006
  fixed exactly that mislabeling).
- Skill IDs 0–27 are the base-game skills, 28 Interference and 29 Runes are
  HotA (`version/hota.py`). The stored count field stays ≤ 0x1C even though 30
  skill definitions exist - storage capacity, usable slots and UI rules differ.

### C.2 Record discovery (`locate.rs`)

Heroes are found by a structural replay of h3sed's locator
(`Savefile.populate_heroes` + `metadata.HERO_REGEX` + `version/hota.HERO_REGEX`),
not by a schema pointer. Algorithm:

1. Scan starts at payload offset **30,000** (skips campaign carry-over hero
   structs stored ahead of the working set).
2. A candidate at offset `o` must satisfy, in order (cheap checks first):
   - record fits the scan window (`o + 1121 ≤ window_end`);
   - skill count low byte ≤ 0x1C and the other three bytes zero;
   - name first byte > 0x20 and byte at `o+181` (name offset 12) == 0;
   - all 70 learned and 70 available spell-flag bytes ∈ {0,1};
   - each of 19 equipment slots: first word is `FF FF FF FF` or `ID 00 00 00`;
   - each of 64 backpack words (128 words of 4 bytes): blank (`FF×4`) or
     `ID 00 00 00`;
   - 30 mastery bytes at 182 ∈ 0..=3;
   - trailing skill-slot run: prefer 30 bytes all ≤ 0x1C (record = 1122),
     accept 29 (record = 1121).
3. Secondary checks on a candidate (failures keep the record as a
   low-confidence `LocatedHero` with explicit `RejectReason`s):
   - name strip: content bytes ≥ 0x1A, then NUL padding to the end, and not
     purely 0xFF content (h3sed `rgx_strip`);
   - equipment grammar (h3sed `EQUIPMENT_REGEX`): not all-zero, not pure
     `00000000 FFFFFFFF` repetition; every slot blank, `ID+3NUL+00000000`,
     `ID+3NUL+FFFFFFFF`, or war-machine `ID+3NUL+?? ?? 00 00`;
   - owner byte ∈ {0..7, 0xFF} (H3Forge-added check).
4. After the first accepted record, scanning continues only within
   **5,000 bytes** after the previous match end (records are densely packed -
   38-byte gaps in the sample); a larger gap ends the heroes section.

A pass means "verified against every implemented structural check", not
game-verified. Sample: 215 candidates, 0 rejected, 214 gaps with 164 exactly
38 bytes. VERIFIED_IN_H3FORGE.

### C.3 Identity: HeroType / HeroClass / HeroPortrait

The triple at offsets 10/11/12 is the hero identity, per the h3svg
specification's runtime `Hero` struct. Evidence on the 44/11 sample
(H3F-005): surrounding fields align exactly (−1 spell-level bytes at 5–6,
−1 destination words at 19–27, movement at 31/35, experience at 39); the
class-byte histogram over 215 records matches exactly the 24 known classes;
unchanged CP1251 roster names sit at type-consistent classes (type 0 =
Orrin/Knight `ce f0 f0 e8 ed`, type 8 = Rion/Cleric, type 16 = Mephala/Ranger);
custom-renamed heroes keep their type bytes ("AbsoluteZero" type 1,
"J. Lannister" type 4). Display names are never used as identity, and record
order is never used as an index into any roster table. VERIFIED_IN_H3FORGE.

The save array is not the roster: 215 records exceed the 156 SoD HeroType
values; classes 22/23 are Random Hero / Crossover Placeholder expansions, and
HotA extends types past 156. `hero_portrait` may exceed the physical portrait
files (IDs up to 244 observed; portrait files only 000–136 - see the
resources document, section F).

### C.4 Army slots

Seven slots; ID at 113+4i, count at 141+4i, both u32 LE.

- Occupied: count 1..=u32::MAX, creature ID must exist in the creature table
  (189 entries for exact 44/11, ported from h3sed `metadata.py` +
  `version/hota.py`, in `crates/h3forge-save/src/creatures.rs`).
- Empty forms: the game uses both `ID=0, count=0` and `ID=0xFFFFFFFF,
  count=0` (and the sample carries an all-0xFF count variant). h3sed
  `common/army.py:parse` treats zero count or FF ID as empty. H3Forge
  preserves untouched empty encodings verbatim; a *clear* operation writes
  `ID=0xFFFFFFFF, count=0` (h3sed's new-empty form).
- Unknown creature IDs (with nonzero count) make the stack read-only:
  preserved verbatim, never edited.

### C.5 Equipment and backpack

19 equipment slots × 8 bytes at 382 (file-order labels: helm, cloak, neck,
weapon, shield, armor, lefthand, righthand, feet, side1–side4, ballista,
ammo, tent, catapult, spellbook, side5). Slot grammar: blank = first word
`FF FF FF FF`; otherwise `ID` + 3 NUL bytes + 4 remaining bytes (artifact,
scroll-with-spell-ID, or war-machine encoding). Slot 17 (offset 518) is the
spellbook: carried iff its first word is not `FF×4` (h3sed `parse_special`);
note that `00 00 00 00` therefore counts as carried.

64 backpack entries × 8 bytes at 534, same word grammar (two artifact words
per entry).

H3F-011 decodes scrolls as two little-endian u32 words: artifact ID 1,
then spell ID 0–69 from the corrected spell table. Ordinary writable IDs
are 2 and 7–126; new ordinary entries use an FF second word. All 158 named
artifact mappings were cross-checked against H3SED binary IDS. Expansion
and combined items remain read-only because version availability and
combination behavior are outside this write capability.

Empty: first word FF, regardless of second word (H3SED parse rule).
Untouched entries retain every byte. Removal writes `FF FF FF FF 00 00 00 00`
(H3SED executable serialization; its nearby comment reverses these words).
ID zero is ambiguous and read-only. Known ordinary IDs with second word zero
or FF are displayed; other trailing words, unknown IDs and invalid scroll
spell words remain unknown/read-only. No occupancy inference from those
unknown forms is used to select an empty slot.

Writes require exact HotA 44/11 and high-confidence records. Each operation
patches only one eight-byte slot at `hero_start + 534 + slot * 8`, preserving
all other bytes, including the offset-4 count byte and equipment. No automatic
compaction, sorting or count normalization is performed (H3SED also serializes
only the slot region). Combination reservations at 1046–1055 remain untouched.
VERIFIED_IN_H3FORGE: application Save As/reopen sample test changed only bytes
550–562 relative to the selected hero, in slots 2 and 3; equipment and all
other payload bytes identical. A user report recorded 2026-11-14 verifies
artifact addition to heroes in HotA but does not identify backpack versus
equipment placement. Scrolls and other inventory cases remain manually
unverified; see [MANUAL_COMPATIBILITY.md](../MANUAL_COMPATIBILITY.md).

#### Equipment editing (H3F-012)

Decoded per slot like the backpack: blank = first word `FF×4` (any tail);
artifact = verified ID with tail `00 00 00 00` or `FF FF FF FF` (the game
writes both; observed 44/11 saves use `00` tails, H3SED writes `FF`); scroll =
ID 1 + spell ID tail; anything else (war machines 3–6, unknown IDs) is
Unknown and read-only. War-machine slots 13–16 and the spellbook slot 17 are
never editable.

Wear slots (0–12, 18) accept equip/unequip of writable ordinary artifacts
(2 and 7–126, scrolls/Grail excluded) whose slot kind matches: helm
19–24/36/44/123/124, cloak 42/55/58/68/72/78/83/99/109, neck
33/43/54/57/66/71/76/97/100–108, weapon 7–12/35/38, shield 13–18/34/39,
armor 25–31/40, hand 37/45/67/69/70/77/94/95/110/113, feet
32/41/56/59/90/98, side everything else writable (h3sed `ARTIFACT_SLOTS`).

*(Superseded by the 2026-11-10 combination follow-up below: the write set is
now 2, 7–143 and 147–165, and reservations are modeled.)* Two hard
preconditions block the whole section with an explicit reason: any wear slot
occupied by an unmodeled item (equipped scroll, unknown entry, artifact in a
slot of another kind) and reservation bytes at 1046–1055 that differ from the
counters implied by the worn combination artifacts. A slot write uses the game-native encoding
(`ID 00 00 00 00 00 00 00`, blank `FF×8`).

Coupling, ported from h3sed `hero/stats.py` / `hero/spells.py` and verified
by round trip: the stored primary attributes (238–241) **include equipped
artifact bonuses**, so an equip/unequip rewrites them by the bonus delta -
`new_stored = (stored − old_total_bonus) + new_total_bonus`, wrapping mod 256
(`ARTIFACT_STATS`, e.g. Centaur's Axe 7 = +2 Attack, Thunder Helmet 24 =
−2 Power/+10 Knowledge). Available flags (312) become
`(learned && has_spellbook) || granted_by_equipped` (grants: Tomes 86–89,
Spellbinder's Hat 124), with h3sed's bannable conditional (Summon Boat 0,
Scuttle Boat 1, Water Walk 7 are not newly set while the stored flag is 0).
Learned flags (242) are never written, so unequipping never removes a
separately learned spell. VERIFIED_IN_H3FORGE: unit tests plus a sample
round trip - equip + unequip of Centaur's Axe on a real 44/11 hero restores
the payload byte-for-byte and limits the diff to the patch union.

Equipment↔backpack moves are composed from one `SetEquipmentSlot` plus one
`SetBackpackSlot` edit (swap or clear), so no duplication or loss occurs;
attribute/stat edit batches that compute the same bytes collapse into one
patch, divergent ones are rejected as an explicit conflict.

### C.6 Spells

Learned (242) and available (312) arrays: 70 one-byte flags each, indexed by
spell ID. The coupling rule (ported from h3sed `hero/spells.py:serialize`):
with a spellbook carried, `available[id] = learned[id]`; without a spellbook,
learning records the learned flag only (available grants from Spell Scroll
artifacts are not decoded, so a scroll-granted spell may need a re-equip).
Map-banned spell restrictions are not parsed; safety comes from patching only
the intended flag bytes and never normalizing the arrays.

### C.7 Coordinates, map status, current town

The front-matter layout is verified on a real pair in section I.1 and is
written only by hero dismissal (`is_visible`). Earlier note: h3sed `hero/profile.py` reads location/on-map status from
bytes *before* the fixed block (over an optional biography), which requires
bounded validation; h3svg documents hero X/Y/Z in the runtime struct with a
map object index that must stay consistent. No H3Forge support; H3F-019 owns
read-only coordinates first.

---

## D. Town structures

Town record discovery and parsing (H3F-013) is implemented in
`crates/h3forge-save/src/towns.rs` + `domain.rs`, replaying h3sed's heuristic
detector (`Savefile.parse_towns`, `metadata.TOWN_REGEX`,
`version/hota.TOWN_REGEX`) with the same "verified vs low-confidence
candidate" discipline as heroes. The layout below is VERIFIED_IN_H3FORGE on
the HotA 44/11 sample (all 37 sample towns, cross-checked against the source
`.h3m`). Offsets are relative to the town record start:

| Offset | Size | Field | Meaning | Confidence |
|---|---:|---|---|---|
| 0 | 1 | owner | player 0-7 or 0xFF neutral (h3sed `faction`) | VERIFIED_IN_H3FORGE |
| 1 | 2 | - | unknown (h3svg: `built_this_turn`, `unknown1`) | UNKNOWN |
| 3 | 1 | type | town type 0-11 (Castle, Rampart, Tower, Inferno, Necropolis, Dungeon, Stronghold, Fortress, Conflux, Cove, Factory, Bulwark); equals the source map's town faction on the sample | VERIFIED_IN_H3FORGE |
| 4 | 1 | X | coordinate 0-252 | VERIFIED_IN_H3FORGE |
| 5 | 1 | Y | coordinate 0-252 | VERIFIED_IN_H3FORGE |
| 6 | 1 | Z | 0 surface, 1 underground | VERIFIED_IN_H3FORGE |
| 7 | 2 | boat tile | x, y where boats built here appear (h3svg `generated_boat_x/y`); `FF FF` without one. Set in exactly the towns with a constructible Shipyard plus one coastal town where the map prohibits it | VERIFIED_IN_H3FORGE, never written |
| 9 | 28 | 7 x u32 | garrison creature IDs (`ID` + 0x00/0xFF tail bytes); empty = `FF FF FF FF` (also `00 00 00 00` with zero count) | VERIFIED_IN_H3FORGE |
| 37 | 28 | 7 x u32 | garrison counts; empty = zero count (occupied counts up to u32::MAX are valid stacks, e.g. the all-0xFF form) | VERIFIED_IN_H3FORGE |
| 65 | 1 | garrisoned_hero | enum HeroType of the garrisoned hero or 0xFF (h3svg `Town.garrisoned_hero`); when set, the hero's own army IS the garrison: the town's seven garrison slots are empty in all 17 such sample towns and stay read-only (h3sed `army.is_readonly`) | VERIFIED_IN_H3FORGE |
| 66 | 1 | visiting_hero | enum HeroType of the visiting hero or 0xFF (h3svg `Town.visiting_hero`); does not block garrison edits | VERIFIED_IN_H3FORGE |
| 67 | 1 | mage guild level | 0-5, read-only | VERIFIED_IN_H3FORGE |
| 68 | 1 | - | unknown | UNKNOWN |
| 69 | 2 | name length | u16 LE; reader supports 1-16, not a claimed format maximum | VERIFIED_IN_H3FORGE |
| 71 | n | name | legacy single-byte text, not NUL-terminated; FF is valid CP1251 "я" | VERIFIED_IN_H3FORGE |
| 71+n | 496 | - | recruits, building masks, Mage Guild spells, HotA extras; see D.1 | partly VERIFIED_IN_H3FORGE, rest byte-preserved |

Record length is `567 + name_len` (supported 568-583), learned from dense adjacency
exactly as h3sed adjusts `BYTES_MINLEN`: every record but the last ends where
the next begins (sample: 37/37 exact); the last record's end is computed from
the learned fixed length. Discovery is bounded to the region before the hero
array (`first_hero_start - 48*583 - 100` .. `first_hero_start`), candidates
run cheap structural checks first (owner byte, type <= 11, coordinate range,
garrison word shape `b 00/FF 00/FF 00/FF`, name length and content), and
secondary verification replays h3sed's army checks: whole-section blankness
agreement (ID words all-FF iff every count byte is 00/FF), blank IDs require
all-zero counts, and occupied stacks need a nonzero count plus a creature ID
from the verified 44/11 table. Failures keep the record as a low-confidence
candidate with explicit reasons; ambiguous or unverified towns never accept
writes.

Hero linkage: a town's garrisoned/visiting hero type bytes identify the hero
record by its immutable `hero_type` byte (hero offset 10), never by name or
record order. On the sample all 17 garrisoned towns and the single visiting
town resolve to exactly one hero record whose front-matter coordinates match
the town (the h3sed `profile.py` walk-back over the biography, bounded to 8
KiB: x i16 at -26, y i16 at -24, z at -22, visibility at -20 = h3sed
`on_map`); mismatches are reported as notes, not silently linked.

H3F-013 enables one write: garrison creature ID/count words (rules per C.4,
shared with heroes), exact 44/11, verified towns, no garrisoned hero.
Building masks and Mage Guild spells are covered by D.1. Ownership changes,
economy edits and town coordinates stay closed.

### D.1 Building masks and Mage Guild (H3F-022)

Implemented in `crates/h3forge-save/src/town_state.rs`. Field names follow
the h3svg `Town` struct; that struct begins one byte earlier than the
H3Forge span (its `id` byte is the last byte of the previous span). Offsets
are relative to the end of the town name (`71 + n`). Verified on all 37
towns of the HotA 44/11 sample (`tests/sample_town_state.rs`).

| Offset | Size | Field | Meaning | Confidence |
|---|---:|---|---|---|
| +0 | 28 | recruits | 7 x u16 basic, 7 x u16 upgraded (h3svg) | VERIFIED_EXTERNAL, not interpreted |
| +28 | 56 | external dwellings | 2 x 7 x u32 (h3svg, second copy duplicates the first) | VERIFIED_EXTERNAL, not interpreted |
| +84 | 5 | guild spell counts | spells offered per guild level 1-5; zero for unbuilt levels | VERIFIED_IN_H3FORGE |
| +89 | 8 | visible mask | u64, building graphic actually drawn (h3svg leaves this unknown) | VERIFIED_IN_H3FORGE for the shared chains |
| +97 | 8 | built mask | u64, logical built set (h3svg `built` + 2 reserved) | VERIFIED_IN_H3FORGE |
| +105 | 8 | constructible mask | u64 (h3svg `can_be_built` + 2 reserved): faction limits and map prohibitions | VERIFIED_IN_H3FORGE |
| +113 | 120 | guild spells | 30 x i32, six slots per level; `-1` empty | VERIFIED_IN_H3FORGE |
| +233 | 9 | banned spells | one bit per spell ID (h3svg `banned_spells`) | VERIFIED_IN_H3FORGE (504 of 507 stored spells are clear) |
| +242 | 61 | - | h3svg `unknown4` | UNKNOWN |
| +303 | 7 | unique building properties | mana vortex / mystic pond bits, summoning portal creature (h3svg) | VERIFIED_EXTERNAL, not interpreted |
| +310 | 185 | - | HotA-specific data | UNKNOWN |

Building bits (h3svg `TownBuildingTypeH3SVG`): 0-4 Mage Guild 1-5, 5 Tavern,
6 Shipyard, 7 Fort, 8 Citadel, 9 Castle, 10 Village Hall, 11 Town Hall,
12 City Hall, 13 Capitol, 14 Marketplace, 15 Resource Silo, 16 Blacksmith,
17 special, 18/19 horde 1 (basic/upgraded dwelling), 20 Shipyard with ship,
21-23 special, 24/25 horde 2, 26 grail, 27-29 extra structures, 30-36
dwellings 1-7, 37-43 upgraded dwellings 1-7. Bits 44-63 are zero in the
sample.

Mask relation. The built mask carries every member of an upgrade chain; the
visible mask keeps only the top graphic. Chains shared by every faction and
confirmed on the sample: Mage Guild `n` hidden by `n + 1`, Fort by Citadel by
Castle, Village Hall by Town Hall by City Hall by Capitol, dwelling by its
upgrade, horde 18 by 19 and 24 by 25. Faction-specific replacements exist on
top (an upgraded horde hides the upgraded dwelling it boosts; Rampart special
21 hides special 17; Cove special 21 hides the upgraded level 3 dwelling;
Factory special 23 hides 22; Bulwark special 22 hides 21; bit 27 can be
visible without being built). Factory keeps Mage Guild 4 drawn beside level
5: both Factory towns show it in all four local saves, no other faction
does. These are PARTIAL: H3Forge preserves such bits and never edits them.
Faction slots differ from the generic names: for Factory, bit 6 is the Mana
Generator and bits 22/23 the Gantry and its upgrade (`town_state.rs` name
tables, from `BldgSpec.txt`, `Dwelling.txt` and `HotA.dat`). Village Hall (10) is set in every town. Bits built but
not constructible include the Village Hall and filler specials of factions
that lack that structure.

Mage Guild. The level byte (record offset 67) equals the unbroken chain of
bits 0-4 in all 37 towns. A level stores its spells in the leading slots and
`-1` after them, up to its capacity: 5, 4, 3, 2, 1, plus one when the Tower
Library (Tower type 2, bit 22) is built. Slots past the capacity hold
uninitialized data. Spells of unbuilt levels are already stored; their count
byte is zero until the level is built. Every stored spell matches its level
in the standard table (`SPELL_LEVELS`: 15, 15, 15, 15 and 10 spells for
levels 1-5). The constructible bits 0-4 give the faction limit (Stronghold
and Fortress 3; Castle, Cove and Bulwark 4).

Writes (exact 44/11, verified town record):

- Guild spells: slots `0..capacity` of one level and that level's count
  byte when the level is built. Requires the guild fields to follow the
  representation above.
- Buildings: the built mask, the visible mask, the Mage Guild level byte and
  the five count bytes, derived together from one target built mask. The
  visible mask is recomputed from the shared chains; bits the model does not
  explain keep their stored value. Editable bits: 0-5, 11, 12, 14-16,
  30-43, plus bit 6 (Shipyard: needs the boat tile, no docked ship, not
  Factory) and the horde pairs 18+19 and 24+25 as one unit (needs the
  upgraded dwelling of the boosted level; building hides that dwelling in
  the visible mask, removal shows it; Dungeon and Cove's second horde are
  excluded as unobserved). A level drawn beside its upgrade is preserved and that pair is
  locked; Mage Guild 5 is locked in every Factory town. A built building
  that a locked one depends on cannot be removed. Bits 7-10, 13, 17, 20-23 and 26-29 are never
  written. Fort, Citadel, Castle and Capitol also select the adventure-map
  sprite: the save holds per-faction templates `avc???x0` (village), `0`
  (fort), `c0`, `f0` (citadel, castle) and `z0` (capitol), and the object
  record that picks one is not located. The remaining bits are structures
  with their own saved state.

The source map's must/may spell masks and building settings describe the
scenario start. They are not read for this state.

## E. Player data (H3F-018)

Verified for the local HotA **44/11** sample only. `players.rs` first parses
read-only views, then `plan_player_edits` gates same-length field patches.
The table is anchored to the complete high-confidence town sequence, never
an absolute payload address or frontend offset. Eight ordered color records
of 149 bytes precede the one-byte town count and first town's ID byte.
In the sample: table 4,634,994..4,636,186, town count 37 at 4,636,186,
first town ID 0 at 4,636,187, first town owner 6 at 4,636,188.

| Relative offset | Size | Meaning | Evidence / policy |
|---|---|---|---|
| +0 | 1 | owner color 0-7 | All eight colors in order; record index is a separate ID |
| +1 | 1 | adventuring hero count | 0-8, matches non-FF hero slots |
| +2 | 1 | selected hero type or FF | Must reference a listed hero or be FF |
| +3 | 8 | hero type slots, FF empty | Each listed type resolves uniquely to a confirmed same-owner hero |
| +11 | 13 | tavern / personality / grail / days fields | Preserved; no writes or activity conclusions from these bytes |
| +24 | 1 | town count | 0-72, exact match to same-owner town IDs |
| +25 | 1 | current town | Preserved |
| +26 | 72 | town IDs | Counted prefix matches the complete same-owner town set; remainder preserved |
| +98 | 28 | seven signed little-endian i32 resources | Wood, Mercury, Ore, Sulfur, Crystal, Gems, Gold |
| +126 | 16 | visited-object masks | Preserved |
| +142 | 1 | tactics-message boolean | Validated as 0/1 |
| +143 | 6 | unknown tail | Preserved; includes four bytes beyond the classic 145-byte record |

The independent [h3svg Player definition](https://github.com/alexanderbelous/h3mtxt/blob/h3svg/doc/h3svg_specification.MD#player)
and [ResourceType enum](https://github.com/alexanderbelous/h3mtxt/blob/h3svg/src/h3mtxt/Map/Constants/ResourceType.h)
supply field order and signedness. The independent implementation
[Player.h](https://github.com/alexanderbelous/h3mtxt/blob/h3svg/src/h3mtxt/SavedGame/Player.h)
also confirms the eight adventuring-hero slots, 72 town IDs and following
resource field. No implementation code is reused. The classic 145-byte stride does NOT apply
to this sample. The vlucas repository tree at
`5bf1cb60b88c66bf1ecd1d10134a67efe070c105` and `src/core/game.ts` were checked
on 2026-11-10: no player-resource implementation or license file was found.
The previous claim of a writable classic resource implementation there was
not substantiated. No source code or offsets were copied from that project.

Sample values and absolute first-resource addresses (decimal):

| Color / record index | Resource start | Wood | Mercury | Ore | Sulfur | Crystal | Gems | Gold | State |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| Red / 0 | 4635092 | 735 | 1607 | 920 | 564 | 123 | 114 | 8721404 | Active, 5 heroes, 0 towns |
| Blue / 1 | 4635241 | 149 | 14 | 296 | 42 | 190 | 47 | 52210 | Unknown, no heroes or towns |
| Tan / 2 | 4635390 | 33 | 26 | 39 | 26 | 32 | 29 | 11262 | Unknown, no heroes or towns |
| Green / 3 | 4635539 | 40 | 26 | 40 | 29 | 26 | 29 | 11132 | Unknown, no heroes or towns |
| Orange / 4 | 4635688 | 382 | 66 | 352 | 48 | 85 | 13 | 552235 | Active, 5 heroes, 19 towns |
| Purple / 5 | 4635837 | 82 | 20 | 34 | 19 | 24 | 16 | 25355 | Unknown, no heroes or towns |
| Teal / 6 | 4635986 | 760 | 285 | 1009 | 376 | 248 | 125 | 89816 | Active, 4 heroes, 14 towns |
| Pink / 7 | 4636135 | 190 | 59 | 280 | 256 | 6 | 5 | 61676 | Active, 5 heroes, 0 towns |

Each subsequent resource is +4 bytes; Gold is resource start +24.
Activity here means a nonempty verified hero or town list. It does not infer
human/AI control, distinguish eliminated from never-used slots, or use
resource amounts as evidence of activity. Neutral owner 255 has no resource
record. H3F-023 corrects the old "Inactive / unused" label to Unknown.
No verified defeat or never-participated flag has been identified. The h3svg
`days_left` byte at +23 is a no-town countdown, not historical participation
evidence. Sample signed values by color are `[6,2,7,5,-1,-1,-1,6]`.
Map eligibility, visit masks and zero current assets do not establish defeat.

Writes require exact 44/11, a verified table anchor and per-player ownership
links and non-negative stored resources. Current asset presence is not a
write gate: all eight sample records have verified resource words, including
the four Unknown slots. Editing resources does not reactivate a player.
Supported input
is **0..2147483647** for each resource: the non-negative i32 domain, not the
full u32 domain. This is a storage bound, not proof of all extreme values'
in-game arithmetic behavior. Negative, overflowing, unknown-resource and
duplicate-field operations are rejected. Unknown or ambiguous tables return
no writable players. The whole original payload and all unrelated bytes stay
intact outside the explicit four-byte patch union.

Copy-based regression changes all seven fields independently for all eight
players (56 round trips), checks no-op/overlap/preimage/unsupported-layout
rejections, and saves Orange Gold to 2147483647 plus Teal Wood to 0.
That two-player diff is exactly six payload bytes:
`4635712, 4635713, 4635714, 4635715, 4635986, 4635987`;
the other **5,271,288** bytes are unchanged. Desktop session regressions cover
Revert, Discard, Save As, direct Save, backups, rebasing, and source-change
rejection. Actual matching-game loading remains a manual acceptance gate.

## F. Format versions

| Family | Version predicate | Record-size/layout notes | h3sed support | H3Forge support |
|---|---|---|---|---|
| Restoration of Erathia | major 16–41, minor 0 | **18 equipment slots**; backpack shifts to the former side5 offset; no combination reservations; different profile/town handling | full (read/write) | container + map text only |
| Armageddon's Blade | 42, 1 | detected before the overlapping SoD range | full | container + map text only |
| Shadow of Death | 42–43, 0–4 | 19 equipment slots; expanded artifacts/creatures | full | container + map text only |
| Heroes Chronicles | `HCHRONSVG` magic | different header/content definitions | read/write (adapted) | container + map text only |
| Horn of the Abyss | ≥44, ≥5 (sample 44/11) | skill-slot order moves to offset 1092; record 1121 (pre-44/10) / 1122; Bulwark/Runes from minor 0x0A; primary-attribute effective ranges differ; towns to type 11, lengths 546-581 (44/11: 567 + name) | full | hero + town read for HotA family; **editing gated to exact 44/11** |

Detection-order caveat: predicates overlap; release order (RoE → AB → SoD →
HC → HotA) is part of the format definition, not an optimization.

"Support" above means exactly: container validation, version detection, map
text parse, and (HotA only) hero location/parsing. Edits additionally require
`hero_layout_supported()` + `army_edit_supported()` (HotA, exact 44/11) and
per-record `HeroConfidence::High`.

## G. Editing and serialization

### G.1 Model

The original decompressed payload is immutable and is the single source of
truth. Editing never serializes a domain model back into bytes; it produces
checked, same-length, in-place byte patches.

`patch.rs`:

- `BytePatch { offset, expected, replacement, reason }` - absolute payload
  offset, expected old bytes (preimage), equal-length replacement, provenance
  string. Construction rejects empty and length-mismatched patches.
- `apply_patches(payload, patches)` - sorts by offset, then verifies
  **before writing anything**: bounds, non-overlap, and exact preimage match.
  Any failure returns an error and leaves `payload` untouched (all-or-nothing).
- `compress_gzip(payload)` - 10-byte header (`1F 8B 08 00`, MTIME 0, XFL 0,
  OS 0xFF), DEFLATE level 6, correct CRC-32 + ISIZE trailer. The compressed
  bytes therefore differ from the original file even for a "no-op" save; the
  guarantee is payload-level, matching h3sed's full-re-gzip strategy (the game
  reads any valid member).

`edit.rs`:

- Operations are semantic (`HeroEdit`: SetAttack/SetDefense/SetSpellPower/
  SetKnowledge(u8), SetExperience(u32), SetLevel(u8), SetSpellPoints(u16),
  SetSpellLearned{id,learned}, SetArmySlot{slot,creature_id,count}) - never
  frontend-supplied offsets. Each plans `BytePatch`es against the document's
  own bytes with the current bytes as expected preimage.
- Write ranges: Attack 0–99, Defense 0–99, Spell Power 1–99, Knowledge 1–99
  (HotA game-valid ranges from h3sed `PRIMARY_ATTRIBUTE_GAME_RANGES`;
  100+ values have cap/handicap semantics and are preserved read-only),
  Level 1–74, spell flags 0–69, army per C.4.
- Spell coupling: with a spellbook, toggling a learned flag also writes the
  available flag (section C.6).
- Level and experience are independent fields; H3Forge never syncs one from
  the other (`EXPERIENCE_LEVEL_MIN` backs UI hints only).
- No-op operations (value already stored) plan zero patches; reverting an
  edit to the stored value removes its patch.

### G.2 Persistence flow (desktop `session.rs::write_save`)

Order of operations for Save / Save As; the source file is never modified by
Save As, and Save itself writes only after a backup exists:

1. **Source fingerprint check** - `SourceIdentity { path, len, modified_ms }`
   captured at open; if the source changed on disk since, the write is
   rejected (`source-changed`).
2. Plan all pending edits against the immutable payload → patches.
3. **Backup** - the destination's existing bytes (for Save: the untouched
   original) are copied to `<dest>.backup-<millis>` and verified by read-back
   against the original bytes before continuing.
4. **Temp write** - the new gzip member is written to a fresh temporary file
   in the destination directory, synced, then re-read.
5. **Reopen validation** - the temp bytes are parsed by the strict reader
   (gzip + CRC/ISIZE + version + hero scan) and its decompressed payload must
   equal the patched payload exactly.
6. **Atomic-ish replace** - no-clobber rename onto the destination (a
   conflicting name wins via a new `.backup-<millis>`); temp cleaned up on
   failure. Repository `games/` and `Data/` and the open source itself are
   hard-rejected destinations.

### G.3 Worked example (VERIFIED_IN_H3FORGE, `tests/sample_edit_roundtrip.rs`)

Target: the owned spellbook-carrying record at absolute span 4,727,078
(section H). Operations:

1. `SetAttack(9)` - stored 3 → one patch at `4,727,078 + 238`:
   bytes `03` → `09`.
2. `SetSpellLearned { id: 0, learned: true }` (Air Shield) - `4,727,078+242`
   `00`→`01` (learned) and `4,727,078+312` `00`→`01` (available; spellbook
   carried, so the coupling applies).
3. `SetSpellLearned { id: 64, learned: false }` (Visions) - `+242+64`
   `01`→`00` and `+312+64` `01`→`00`.

Result: 5 patches; the reopened save differs from the original payload in
**exactly** those 5 offsets and nowhere else (sorted-offset diff assertion).
Attack reads back as 9, `spells_learned[0]=1`, `spells_learned[64]=0`.

**Compatibility claim:** this exact fixture has parser round-trip evidence.
The user now reports successful known-spell, mana and all four primary attribute
changes in actual HotA (recorded 2026-11-14), without identifying that fixture,
spell or exact game version. See [MANUAL_COMPATIBILITY.md](../MANUAL_COMPATIBILITY.md).
Do not infer untested variants or additional format coverage from either result.

### G.4 Currently writable vs frozen ranges

Writable (HotA 44/11, High-confidence records): hero bytes at record-relative
offsets 238–241, 39–42, 49, 47–48, 242–381 (spell flags), 113–168 (army
words, C.4 rules), 43 (skill count low byte), 182–211 (skill mastery bytes)
and 1092–1121 (HotA skill order bytes), all driven by the `SetSkills`
absolute-list operation (H3F-010: dense renumbering, removals zeroed,
unrecognized entries and the legacy 212–237 region byte-preserved), 534–1045
(backpack entries, H3F-011), and 382–533 (wear equipment slots 0–12/18 plus
their coupled attribute bytes 238–241 and available flags 312–381, C.5
equipment rules and blockers). Everything else - including the entire
non-hero payload, war-machine/spellbook equipment slots, movement, owner,
name, offsets 10–12, and reservation byte 1046 - must remain byte-identical.
Town records: garrison words 9–64 (H3F-013), the Mage Guild level byte 67 and,
relative to the name end, count bytes +84..+88, masks +89..+104 and spell
slots +113..+232 under the D.1 rules (H3F-022); the constructible mask, the
banned-spell mask and every other town byte stay identical.
Hero dismissal (H3F-026, section I.3) additionally writes hero bytes 0, 2-3,
19-26, 31-38, 47-48, 109-111, the army words 113-168 and words 1076-1083, the front-matter `is_visible` byte, one hero owner table
byte, player record bytes 1-10, three fields of one map tile and one boat
`exists` byte.
The reservation counters 1047–1055 change only with an equipment edit that
adds or removes a combination artifact. Unknown bytes survive by construction: patches are
equal-length, preimage-checked, and never resize the payload.

## H. Real sample evidence: SaAndVla.GM2 (HotA 44/11)

All values below are asserted by the opt-in fixture tests
(`crates/h3forge-save/tests/sample_save.rs`, `sample_edit_roundtrip.rs`,
`sample_portraits.rs`; run with `H3FORGE_FIXTURE_ROOT=D:\Projects\h3forge\games`).

| Property | Value |
|---|---|
| Compressed size | 1,512,923 bytes |
| SHA-256 | `f326b6ff04cb2b6d3fd7e44773dfa78eecca28f02277ff201b620c215b3856b5` |
| Decompressed payload | 5,271,294 bytes |
| First 16 payload bytes | `48 33 53 56 47 00 00 00 2c 00 00 00 0b 00 00 00` (= `H3SVG`, major 44, minor 11) |
| Container kind | Regular (`H3SVG`) despite `.GM2` |
| Map name / description | `- The Mysterious Island - COOP` / present |
| Hero records | 215 located, 0 rejected by secondary checks; first span 4,657,602; last span 4,921,020 (record length 1122) |
| Owner distribution | 179 neutral (0xFF), 14 owner 4, 12 owner 6, 5 owner 7, 5 owner 0 |
| Gaps | 164 of 214 inter-record gaps exactly 38 bytes; four larger (400–739) |
| Names | 163 of 215 carry high bytes; CP1251 decoding yields coherent Russian names plus English custom names (52 ASCII, e.g. `Captain Nemo`, `J. Lannister`, `Zehirmahnn`); 0 duplicate name byte-strings |
| Level/experience | 8 stored-vs-derived mismatches (scripted map); surfaced as diagnostics, not rejections |
| Controlled-edit candidate | span 4,727,078: owner 4 (Orange), level 2, experience 1064, spell points 73, movement 1830/1830, primary 3/7/6/6, skills Wisdom+Intelligence+Mysticism, 13 learned spells `[3,15,17,20,25,33,35,37,42,43,44,47,64]`, army IDs 71/73/75/77/80/77/81 counts 415/251/168/82/9/12/38, spellbook word `00 00 00 00` (carried) |
| Patch round trip | 5 planned offsets, exact-match payload diff (section G.3) |

Known discrepancies/limitations: the hero count (215) is not correlated with
any schema count field (none equal to 215 exists in the 800 bytes before the
first record); minor 11 is recognized by h3sed's tables but not explicitly
documented there as a retail release; the other local saves
(`AUTOSAVE.GM2`, `BATTLE_[hotseat].GM2`, `SaAndVla.GM2.20261107`) are listed
but not parsed - battle/campaign saves need separate evidence.

The sample is private, copyrighted data: it is never committed, copied into
fixtures permanently, or used as a write destination. Tests copy it into
ignored `.local/fixtures/` or OS temp.

---

## I. Hero dismissal (H3F-026)

Implemented in `crates/h3forge-save/src/dismiss.rs`. Evidence: one private
pair saved immediately before and after a native "Dismiss Hero" action with
no other action in between (`SaAndVla_before.GM2`, `SaAndVla_after.GM2`,
HotA 44/11, 252 x 252 x 2 map, 48 towns, 215 heroes, 28 boats). Hero:
Galthran, hero type 71, Pink, selected, aboard a boat at (3, 46, 0).
Payload 5,035,740 -> 5,035,766 bytes. Offsets below are decimal payload
offsets in the before-save. VERIFIED_SAMPLE unless noted.

### I.1 Structures located for the operation

Section order follows the h3svg `SavedGame` struct: tiles, object
templates, objects, object property tables (ending with boats and
obelisks), `Player[8]`, towns, heroes, hero owner table, later sections.

**Hero front matter.** Each hero struct starts 26 bytes before its
biography text; the located record (section C) starts at the owner byte
after the biography. Relative to the biography start:

| Offset | Size | Field |
|---|---:|---|
| -26 | 2 | x (i16) |
| -24 | 2 | y (i16) |
| -22 | 2 | z (i16) |
| -20 | 1 | `is_visible`: 1 on the adventure map, 0 otherwise |
| -19 | 4 | packed coordinates: x, y << 16, z << 26 |
| -15 | 4 | object class under the hero (u32) |
| -11 | 1 | `has_object_under` |
| -10 | 4 | properties of the object under the hero |
| -6 | 1 | `is_female` |
| -5 | 1 | `use_custom_biography` |
| -4 | 4 | biography length (u32), then the text |

The 12 bytes after each located record belong to the same hero struct
(`00 D0 07 00 00 FF FF 00 00 00 00 00` for the last hero).

**Hero owner table.** One byte per hero type, starting 12 bytes after the
last located hero record (4,605,238 for 215 heroes; record index equals
hero type for all 215). An owned hero carries the owner color (all 27
owned heroes agree with their record owner byte). Unowned heroes carry
0xFF (92: ordinary recruitable pool), 0x40 (31) or 0x41 (65). The h3svg
`hero_can_be_hired_by` table follows (215 bytes); dismissal leaves it
unchanged.

**Map tile.** Fixed 22 bytes plus a render list:

| Offset | Size | Field |
|---|---:|---|
| 0 | 6 | terrain, river, road type and sprite bytes (terrain 8 = water) |
| 6 | 2 | flags; bit 0x10 of byte 7 marks an actionable object |
| 8 | 2 | object class (34 = hero, 0 = none) |
| 10 | 2 | object subclass |
| 12 | 2 | object index, FFFF for a hero |
| 14 | 4 | object properties; the hero type for a hero tile |
| 18 | 4 | render entry count `n`, followed by `n` x 4 bytes |

Tiles are stored in z, y, x order. Walking 127,008 - 11,595 tiles forward
from Galthran's tile (346,097; index 46 * 252 + 3) lands exactly on the
object template table at 3,596,007 (`u32` count 6,591, then the `String16`
"AVWmrnd0.def"). H3Forge finds a hero's tile by its class/index/type
signature and accepts it only if this walk from the hero's own coordinates
lands on a template table, with exactly one such candidate.

**Boat table.** `u8` count plus 28-byte records, ending at the obelisk
table (`Player[8]` start - 49). 44/11 layout (one byte longer than h3svg):

| Offset | Size | Field |
|---|---:|---|
| 0 | 1 | `exists` |
| 1 | 1 | index of the record |
| 2 | 1 | boat subclass |
| 3 | 1 | direction |
| 4 | 1 | owner color or FF |
| 5 | 1 | hero type of the last or current rider, or FF |
| 6 | 1 | unknown, 0 in the sample |
| 7 | 1 | 1 while boarded by a hero |
| 8 | 6 | x, y, z (u16 each); stale while boarded |
| 14 | 1 | `is_visible`: 0 while boarded |
| 15 | 13 | packed coordinates and "object under" fields |

The count is accepted only when the id bytes enumerate all records.

**Replay events.** A `u32` count (4,922,520) and variable-length events,
each starting with a type byte and a player color. Not parsed by H3Forge.

### I.2 The native diff, byte for byte

| Offset | Bytes | Meaning |
|---|---|---|
| 907 | 10 | embedded save file name |
| 346,104 | `12 22` -> `02 00` | tile: actionable flag cleared, class 34 -> 0 |
| 346,111 | `47` -> `00` | tile properties: hero type -> hero's "object under" properties |
| 4,326,698 | `01` -> `00` | boat 23 `exists` |
| 4,327,931 | `03 47 47 4E 42` -> `02 FF 4E 42 FF` | Pink: count 3 -> 2, selected 71 -> FF, slots 71/78/66 -> 78/66/FF |
| 4,335,985 | `98` -> `9A` | town 13 (Pink), name end + 321, inside the HotA-specific block: unexplained |
| hero - 20 | `01` -> `00` | `is_visible` |
| hero + 0 | `07` -> `FF` | owner |
| hero + 2 | `02 01` -> `00 00` | temporary morale, luck |
| hero + 19 | 8 bytes -> FF | destination x, y |
| hero + 31, + 35 | `C4 09` -> `98 08` | movement total and left 2500 -> 2200 |
| hero + 47 | `23` -> `28` | spell points 35 -> 40 (knowledge 4 x 10) |
| hero + 109 | `04 C0 04` -> `00 00 00` | visited-bonus block |
| hero + 141 | `9B` -> `01` | army slot 0 count 155 -> 1 (creature 56 kept) |
| hero + 1076 | `26 07` -> `98 08` | u32 1830 -> 2200: land movement value |
| hero + 1080 | `C4 09` -> `DC 05` | u32 2500 -> 1500: sea movement value |
| 4,605,309 | `07` -> `FF` | hero owner table entry 71 |
| 4,922,520 | `E4` -> `E6` | replay event count 996 -> 998 |
| 4,924,201 | `86` -> `01` | byte inside an older replay event of another player: unexplained |
| 4,945,037 | +26 bytes | two appended replay events: `08 07 47 00 00 00 FF 07` (type 8, Pink, hero 71, owner FF, previous owner 7) and `0C 07 0C 00 00 00 01 00 00 00 17 00 00 00 B8 FF FF FF` (type 12, Pink, 12-byte body naming boat 23) |

Hero start is 4,438,181. The 90,703 bytes after the insertion are
identical. Artifacts, skills, spells, experience and the
`hero_can_be_hired_by` entry are unchanged; tavern slots of all players are
unchanged.

### I.3 Second clean pair: land hero

`SaAndVla_before2.GM2` -> `SaAndVla_after2.GM2`: Moandor (hero type 66,
Pink, not selected) dismissed on plain land at (38, 44, 0), six stacks,
Basic Intelligence, Basic Navigation, no Logistics. Payload 5,035,766 ->
5,035,774. Hero start 4,432,381. VERIFIED_SAMPLE.

| Offset | Bytes | Meaning |
|---|---|---|
| 913 | 5 | embedded save file name |
| 332,866 | `10 22` -> `00 00` | tile: actionable flag cleared, class 34 -> 0 (subclass `01 00` kept) |
| 332,873 | `42 00 00 00` -> `FF FF FF FF` | tile properties -> hero's "object under" properties |
| 4,327,931 | `02` -> `01` | Pink hero count (selected hero was already FF) |
| 4,327,934 | `42` -> `FF` | slots 78/66 -> 78/FF |
| hero - 20 | `01` -> `00` | `is_visible` |
| hero + 0 | `07` -> `FF` | owner |
| hero + 19 | 8 bytes -> FF | destination |
| hero + 31, + 35 | 1560 -> 2000 | movement total and left |
| hero + 47 | 57 -> 72 | spell points: knowledge 6 x 12 (Basic Intelligence) |
| hero + 117 | 20 bytes -> FF | army creature IDs of slots 1-5 blanked |
| hero + 141 | 483 -> 1, then five counts -> 0 | army counts |
| hero + 1076 | 1560 -> 2000 | land movement word |
| hero + 1080 | 3250 -> 2250 | sea movement word (1500 x 1.5, Basic Navigation) |
| 4,605,304 | `07` -> `FF` | hero owner table entry 66 |
| 4,922,520 | `E6` -> `E7` | replay event count 998 -> 999 |
| 4,945,063 | +8 bytes | one appended replay event `08 07 42 00 00 00 FF 07` |

What this pair settles:

- Land dismissal is the boat recipe minus the boat record and its second
  replay event. The tile restore rule holds with properties `FF FF FF FF`.
- The visited block is NOT cleared as a whole: bytes 53, 81 and 85 keep
  their values. Only bytes 109-111 changed in the first pair.
- No town byte and no older replay event byte changed, so those two
  changes of the first pair are incidental. The town byte is one bit
  (spell 9) of a 9-byte per-spell mask at name end + 320 whose bits are
  spells of that town's Mage Guild; it carries no hero reference.
- Army: slot 0 keeps one creature, slots 1-6 become `FF FF FF FF` with
  zero counts, also for a six-stack army.
- The sea word drops the owner-wide +1000 seen before dismissal for both
  heroes (2500 and 3250) and keeps only the hero's own Navigation factor.

Reset values across all observed heroes (71 and 66 from the clean pairs;
66 and 150 again from the older, non-clean `SaAndVla.GM2.20261107` ->
`SaAndVla.GM2` pair):

| Hero | Spell points | Knowledge, Intelligence | Land word | Sea word | Notes |
|---|---|---|---|---|---|
| 71 | 40 | 4, none | 2200 | 1500 | wears Equestrian's Gloves |
| 66 (land pair) | 72 | 6, Basic | 2000 | 2250 | Basic Navigation |
| 66 (older pair) | 10 | 1, none | 2000 | 1500 | |
| 150 (older pair) | 80 | 8, none | 2000 | 1500 | |

Still PARTIAL: hero 150's slot 0 held creature 65 before and 56 after in
the older pair, so the game probably keeps the hero type's first starting
creature. Both clean pairs have that creature in slot 0 already, so they
cannot tell the two rules apart.

### I.4 What H3Forge writes

Same-length patches only, each with its preimage:

- front matter `is_visible` -> 0;
- record owner -> FF, bytes 2-3 -> 0, bytes 19-26 -> FF, bytes 109-111 -> 0;
- army words 113-168: slot 0 = first occupied stack's creature with count
  1, slots 1-6 blank (`FF FF FF FF` id, zero count);
- spell points 47-48 -> knowledge x 10 without Intelligence, x 12 with
  Basic Intelligence (mastery byte 206), knowledge 1-99; higher mastery
  keeps the stored points;
- movement 31-34, 35-38 and word 1076 -> 2000, only without Logistics
  (mastery byte 184) and without worn Equestrian's Gloves, Boots of Speed
  or Wayfarer's Boots (70, 98, 151);
- word 1080 -> 1500 without Navigation, 2250 with Basic Navigation
  (mastery byte 187), only without worn Necklace of Ocean Guidance, Sea
  Captain's Hat or Admiral's Hat (71, 123, 136);
- hero owner table entry -> FF;
- player record bytes 1-10: count, selected hero (FF when it was the
  dismissed hero), slots compacted in order and FF padded;
- tile: bit 0x10 of byte 7 cleared, class -> 0, properties -> the hero's
  "object under" properties;
- boarded boat: `exists` -> 0.

`tests/sample_dismissal.rs`: on the land pair the result equals the
game's after-save except for the file name and the replay log. On the boat
pair it additionally differs in the land movement words (2200, gloves
bonus not verified) and the incidental town byte.

Design choices, not verified game behavior: the creature kept in the army
(first occupied stack), and the replay log is not extended (its native
update resizes the payload; the log records the previous turn for
playback). The older replay event byte and the town mask byte are never
written.

Gates: exact 44/11; all heroes high confidence with record index = hero
type; the owner table consistent with every record; a verified player
table; an unambiguous boat table; known map size and levels; hero owned,
listed once in the owner's slots, `is_visible` 1, packed coordinates
matching, object class under 0, `has_object_under` 0, properties under
`00000000` or `FFFFFFFF`; not the garrisoned or visiting hero of any town;
exactly one validated tile; water terrain if and only if exactly one
boarded boat names the hero; a player without towns keeps one hero.
Garrisoned heroes are not in the player's slot list (sample: hero 69 in
Pink's town 13) and have no evidence, like visiting heroes and heroes on
objects. Loading a dismissed-hero save in the game is an open manual gate.

## Appendix: sources and provenance

External (verified = read at the recorded revision; facts extracted and
re-stated above, not copied wholesale):

| Source | Location | Revision | Used for | License |
|---|---|---|---|---|
| H3SED | local `D:/Projects/h3sed_4.1/src/h3sed/`; upstream `https://github.com/suurjaak/h3sed` | supplied 4.1 tree (no git metadata; files incl. Nov 2026 modifications) | `metadata.py` (byte positions, name tables, experience table, hero regex, gzip patch note), `version/hota.py` (HotA hero regex, skill order 1092, town regex, ranges, BULWARK_MINOR), `common/army.py` (empty encodings, ranges), `hero/{stats,skills,spells}.py` (spellbook semantics, spell coupling), `Savefile.populate_heroes/parse_towns` (scan strategy) | MIT (copyright 2020 Erki Suurjaak; `licenses/h3sed-MIT.txt`) |
| h3svg specification | `https://github.com/alexanderbelous/h3mtxt`, branch `h3svg`, `doc/h3svg_specification.MD` | branch head as of 2026-11-08 (no commit pinned) | runtime `Hero` struct field names (sections C.1 offsets 1–27), `SavedGame`/`Player[8]`/`Town[]` existence claims, HeroType/HeroPortrait identity model. Every field H3Forge uses was re-validated against the 44/11 sample before adoption. | not stated in doc; facts only |
| H3M specification | same repo, `master`, `doc/h3m_specification.MD` | branch head 2026-11-08 | map-format distinction: HeroType (156 values), HeroPortrait, CustomHero - used to avoid conflating H3M with GM1/GM2 | not stated |
| H3save_reader | `https://github.com/AlexZ98765/H3save_reader` (`02_format_docs/`, master) | branch head, accessed 2026-11-08 | GM1 compendium + `gm1_mapping.json` - documents a 42/2-era layout; explicitly **not** assumed compatible with 44/11 | not inspected for license |
| vlucas homm3-savegame-editor | `https://github.com/vlucas/homm3-savegame-editor` | branch head 2026-11-08 | prior player-resources claim not substantiated; tree/source inspected for H3F-018 (section E) | no license file found; no code reused |
| VCMI | `https://github.com/vcmi/vcmi`, `client/render/CDefFile.cpp` (develop) | branch head 2026-11-09 | DEF format cross-check (see resources document) | GPL-2.0+; format facts only, no code |

Internal (authoritative):

| Module | Content |
|---|---|
| `crates/h3forge-save/src/container.rs` | gzip member validation, bounded inflate |
| `crates/h3forge-save/src/detect.rs` | magic/version detection, family table |
| `crates/h3forge-save/src/parse.rs` | map name/description parsing |
| `crates/h3forge-save/src/locate.rs` | hero record discovery and checks |
| `crates/h3forge-save/src/definitions.rs` | all numeric offsets and name tables |
| `crates/h3forge-save/src/domain.rs` | record decoding (skills, army, spellbook, identity) |
| `crates/h3forge-save/src/edit.rs` | edit operations and patch planning |
| `crates/h3forge-save/src/patch.rs` | BytePatch, apply, gzip writer |
| `crates/h3forge-save/tests/sample_*.rs` | sample-verified assertions |
| `apps/h3forge-desktop/src-tauri/src/session.rs` | persistence flow (backups, temp, reopen validation) |
| `docs/RESEARCH.md` | dated verification evidence for every VERIFIED_IN_H3FORGE claim |

Listed but not individually inspected (their LOD/PCX descriptions agree with
the mmarch-derived implementation; no claim below depends on them):
`https://github.com/neunato/homm3-unpacker`,
`https://github.com/Laserlicht/homm3-explorer`,
`https://github.com/josch/lodextract`,
`https://pypi.org/project/lodfile/`,
`https://github.com/HeroWO-js/h3m2json` (+ `h3m-The-Corpus.txt`),
`https://github.com/potmdehex/homm3tools`,
`https://github.com/mapron/FreeHeroes`,
`https://github.com/cysun/Heroes3Editor`.

#### H3F-012 visual-editor follow-up (2026-11-10)

Equipment writes now patch only changed primary-bonus deltas and spells entering
or leaving the modeled equipment-grant set. They no longer normalize unrelated
available flags to a whole-record formula. Newly granted non-bannable spells are
set; withdrawn grants clear only when no remaining modeled artifact or learned
spellbook spell supplies them. Learned flags and unrelated stored deviations
remain unchanged. The UI previews primary stats from these same checked patches.

Unknown/combination/equipped-scroll occupants and nonzero reservations still block
**all equipment writes**. Allowing apparently independent edits was investigated
but not established: delta arithmetic alone does not prove unknown coupled effects
or reservation ownership. Diagnostics now identify each item's slot/name/ID (or
raw unknown bytes) and every nonzero reservation offset/value. Reservation-to-item
ownership remains unresolved. Backpack changes remain independent and preserve
these bytes; two-position moves are validated atomically in one pending request.
No new save layout, overwrite path, or reservation write is enabled.

### Combination artifacts, HotA artifacts and in-place equipment (2026-11-10)

Supersedes the blanket "combination/reservation blocks all equipment writes"
rule above for records whose reservation bytes are explained by the model.

**Reservation counters (VERIFIED_SAMPLE).** Offsets 1047–1055 hold one counter
per wear-slot kind, in the order helm, cloak, neck, weapon, shield, armor,
hand, feet, side (h3sed `HERO_BYTE_POSITIONS["reserved"]`). Each worn
combination artifact adds 1 per extra slot it takes besides its primary slot
(h3sed `ARTIFACT_SLOTS[name][1:]`). Byte 1046 is not a counter and was 0 in
every sample record. The counters are not positional: the save does not say
which hand or side slot is taken.

Evidence: all high-confidence heroes of the four 44/11 samples (4 × 215
records) satisfy `bytes == counters(worn combinations)` exactly. Worn cases:

| Artifact (ID, primary slot) | Bytes 1046–1055 |
|---|---|
| Power of the Dragon Father (134, armor) | `00 01 01 01 01 01 00 02 01 00` |
| Cloak of the Undead King (130, cloak) | `00 00 00 01 00 00 00 00 01 00` |
| Bow of the Sharpshooter (137, side) | `00 00 00 00 00 00 00 00 00 02` |
| Armor of the Damned (132, armor) | `00 01 00 00 01 01 00 00 00 00` |

In each case the reserved slots are empty in the record. Taking each worn
combination off and putting it back through the planner restores the payload
byte for byte apart from the slot word's blank half
(`sample_reservation_counters_match_worn_combinations`). The other twelve
combination rows follow h3sed only (VERIFIED_EXTERNAL): 129, 131, 133, 135,
136, 138–143 and 160.

**Write rules.** An equipment batch is planned as one desired state:

- Slot words as before; unchanged slots keep their original bytes.
- A combination artifact needs its primary slot plus, per kind,
  `worn + reserved ≤ capacity` (hand 2, side 5, others 1); otherwise the edit
  is rejected naming the items to unequip.
- Counters 1047–1055 are rewritten only where the modeled value changes.
- Attributes 238–241 move by the joint bonus delta; combination bonuses are
  totals including their components (Angelic Alliance 129 = +21 all, Power of
  the Dragon Father 134 = +16 all, Titan's Thunder 135 = +9/+9/+8/+8, Armor of
  the Damned 132 = +3/+3/+2/+2, Ironfist of the Ogre 143 = +5/+5/+4/+4,
  Armageddon's Blade 128 = +3/+3/+3/+6; HotA Trident 147 = +7 Attack, Shield
  of Naval Glory 148 = +7 Defense, Royal Armor of Nix 149 = +6 Spell Power,
  Crown of the Five Seas 150 = +6 Knowledge - h3sed `ARTIFACT_STATS`).
- Spell grants: Armageddon's Blade grants Armageddon (26), Titan's Thunder
  grants Titan's Lightning Bolt (57).

**Write set.** Artifact IDs 2, 7–143 and 147–165 are writable in the backpack;
all of them except The Grail are equippable in their primary slot kind. The
HotA primary kinds agree with every worn occurrence in the samples (147
weapon, 148 shield, 149 armor, 150 helm, 151 feet, 153–155/162/165 side,
156/163 hand); the remaining HotA kinds are h3sed-only.

**Still blocked / unresolved.** Equipped spell scrolls, unknown entries,
artifacts sitting in a slot of another kind, and reservation bytes the model
does not explain keep the record's equipment read-only. Assembling a
combination from worn components and disassembling it are not implemented.
Non-attribute effects (morale, luck, resources, HotA special effects) are
computed by the game and have no stored coupling known here. In-game loading
of an edited combination remains a manual check.

## H3F-023 - Read-only obelisk visits and exploration gates

Verified on private copies of HotA 44/11 `SaAndVla.GM2` and its linked H3M.
No exploration writes are supported.

### Obelisk visit masks

The [h3svg ObjectPropertiesTables/Obelisk specification](https://github.com/alexanderbelous/h3mtxt/blob/h3svg/doc/h3svg_specification.MD)
places a one-byte count and 48 one-byte visit masks immediately before
`Player[8]`. The sample matches: count **29** at **4,634,945**, masks at
**4,634,946..4,634,994**. The first 29 are meaningful; the other 19 are zero.
Bits 0-7 name owner colors Red through Pink. H3Forge only reads this shape
for exact 44/11 with the verified eight-player table, count at most 48 and
zero unused masks. UI counts require the linked H3M obelisk count to agree.

Read-only visit totals: Red 0, Blue 5, Tan 15, Green 15, Orange 17,
Purple 5, Teal 17, Pink 1. Blue/Purple, Tan/Green and Orange/Teal bits agree
on every mask, consistent with authored H3M teams. This does not establish
current GM2 alliance fields or prove every field needed for a puzzle edit.

H3M object ordinals are not proven indexes into this 48-slot array.
Separate puzzle counters, shared-team update semantics, and current buried
Grail availability have not been established. `Reveal Puzzle Map (Grail)`
stays disabled, even with an authored Grail placement and known visit totals.
No counter-only or mask-only write is offered.

### Fog-of-war investigation: not a verified layout

The classic h3svg `TileVisibility` is a visibility mask followed by an
adjacent-monster boolean, per tile. For this 252 x 252 two-level sample,
127,008 tiles would need 254,016 bytes. A bounded read-only probe after the
hero section found a 254,078-byte boolean lane from 4,922,707 to 5,176,785
(exclusive), admitting **32** candidate starts for a two-byte tile layout.
All 32 candidate visibility starts, 4,922,706 through 4,922,768 in steps of
two, mark all 33 currently owned town tiles visible under the hypothesized
z/y/x tile order. The town cross-check therefore cannot choose an alignment.
There is no verified section boundary, tile order or original-HotA sharing
rule. H3Forge does not expose candidate counts as actual revealed tiles and
does not add a reveal patch planner.

[VCMI's reveal operations](https://github.com/vcmi/vcmi/blob/develop/server/processors/PlayerMessageProcessor.cpp)
and [obelisk handling](https://github.com/vcmi/vcmi/blob/develop/lib/mapObjects/MiscObjects.cpp)
use team visibility, object visit flags and a team obelisk counter. These are
conceptual evidence of coupling, not a description of original GM2 bytes.
No VCMI code or serialization was copied.

### Controller and starting-state boundary

Read-only research found a scenario filename at 2451, a plausible classic
starting-faction array at 2410, a control-like array at 2802 and the authored
team byte pattern at 527. These are candidate matches, not structurally
walked save fields. In particular the control-like bytes also match H3M
eligibility, so they cannot establish the current human/AI controller.
None is exposed as verified current state or writable. Starting and team
facts shown by H3F-023 are explicitly H3M facts, not inferred from ownership.

H3F-023 changes **no** GM2 fields itself. The regression's existing resource
edit (Orange Gold 552235 -> 1234567) changes exactly bytes 4635712, 4635713,
4635714 inside its four-byte word; all other **5,271,291** payload bytes are
identical, including visit masks and the entire possible visibility region.
Actual matching-game loading remains a separate manual gate.
# H3F-027 current map armies

Verified against a private copy of `SaAndVla.GM2`, exact HotA 44/11.
These sample offsets are observations, never hard-coded write destinations.

| Structure | Sample offset / count |
|---|---|
| Current tile array | 7,682; 127,008 tiles |
| Template table | 3,683,810; 2,466 templates |
| Object table count | 3,799,124; 37,698 objects |
| Garrison table count | 4,631,040; 47 records |
| Garrison records | 4,631,041; 61 bytes each |
| Boat table count | 4,633,908; 37 records of 28 bytes |
| Player table | 4,634,994, preceded by the 49-byte obelisk block |

The tile layout from section I still applies. Its `object_idx` references
the current GM2 object table, not the H3M ordinal. Each template has a
String16 DEF name followed by 33 bytes: width/height, four six-byte masks,
class/subclass u16, two reserved bytes and ground flag. Object instances
are five bytes: X/Y/Z and a u16 template index. Action masks use six rows
of eight bits; relative to the bottom-right anchor, displacement `(dx,dy)`
selects row `5-dy`, bit `7-dx`. Garrison orientations have different action
offsets; a universal anchor-X-minus-one rule is incorrect.

Discovery validates complete template and object arrays and a unique full
tile walk, then checks each actionable army tile's object index, class,
subclass, level and action-mask position. The tile start must be within
the first 30,000 bytes and template table within 16 MiB; render counts are
bounded to 4,096. Unsupported/ambiguous layouts stay unavailable.

**Later-save boundary regression (2026-11-13).** A length-only reverse walk can
mistake header bytes for tiles. Discovery now checks each render entry against
its current object: u16 object index at entry +0 must exist; byte +2 must equal
`(dy << 4) | dx`, where `(dx,dy)` is object anchor minus tile coordinates,
`dx < 8`, `dy < 6`, and levels agree. Entry +3 remains uninterpreted/preserved.
The tile ordinal is derived backwards from the complete expected tile count,
not from a guessed start. This reuses the render relationship already validated
for relocation; it does not authorize any new write field. Zero-render tiles
remain valid; multiple structurally valid walks still fail closed.

Verified on private copies, both HotA 44/11, 252 x 252 x 2:

| Save | Payload bytes | Tile start | Template start / count | Objects | Current monsters / garrisons |
|---|---:|---:|---|---:|---|
| SaAndVla_red_tent_before.GM2 | 5,011,968 | 2,263 | 3,546,131 / 6,623 | 36,540 | 1,119 / 46 |
| SaAndVla.GM2 (later) | 4,948,677 | 2,253 | 3,493,237 / 6,644 | 36,540 | 469 / 46 |

All 187,423 older and 174,202 later render entries match these relationships.
The later file also admits a length-only start at 1,277, but that walk contains
27 invalid object references and 225 anchor/relative-position mismatches. It is
rejected structurally, not by choosing the greater offset or a filename rule.
All listed groups have verified current armies. The differing totals describe
current saves only, not proof of why particular objects disappeared.

Compressed SHA-256:
- Earlier: `A2EFEAB301BD319652FB4FD37CF93276EBA43B3E0773F9731FE96F9E72EEBBD2`.
- Later: `C92361B8887D2E89A3453669617E89093A3DA3B09B799073F94260F0AEE45B1F`.


For garrisons, tile +14 holds a u32 record index. All 47 indexes enumerate
the entire garrison table exactly once and all 47 record coordinates match
the corresponding current actionable tiles. The table is located backwards
from verified Player[8], the obelisk block and boat ID chain; a candidate is
accepted only when the count and every garrison index/coordinate agree.

| Garrison relative offset | Bytes | Meaning / write policy |
|---:|---:|---|
| 0 | 1 | owner 0-7 or FF; preserved |
| 1 | 28 | seven u32 creature IDs; editable |
| 29 | 28 | seven u32 quantities; editable |
| 57 | 3 | actionable X/Y/Z; preserved |
| 60 | 1 | removable troops flag; preserved |

Empty slots are FFFFFFFF/0. Unknown IDs and inconsistent slots make that
whole army read-only. Writes use checked same-length BytePatch values only
for the two words of each changed slot. Positive quantities are conservatively
limited to signed-32 range. No object creation/removal, ownership, permission,
aggression, reward, growth or joining flags are changed. Sample results:
47 garrisons, 28 occupied, 19 empty; all current class 33 (subtypes 0/1).
Current class 219 has no sample evidence and remains read-only; matching
structural checks are required for display if encountered. Incomplete index coverage closes all garrison writes.

There are 291 current class-54 wandering groups. Current IDs come from
matching tile/template subtypes; the tile property is a separate monster
table index, not the GM1 scanner's packed 12-bit count. See the survivor
evidence below. Empty garrisons and missing authored objects are not proof
of defeat. Under-hero objects and other guarded-object armies remain outside
this locator.

The linked H3M has 787 initial monster placements: 661 fixed, 126 random.
280 current groups match fixed IDs/anchors and 11 match random placements.
All 47 garrisons match initial anchors, but only 29 armies remain identical.
This establishes the separation of current and initial state, not the reason
each absent object disappeared. See [H3F-027](../issues/027-neutral-creatures.md)
for references, tests and requested before/after evidence.
## H3F-027 supplied native defeat pair

`SaAndVla_before_defeat.GM2` / `SaAndVla_after_defeat.GM2` were compared
on private copies. Payload lengths are 5,015,467 and 5,015,738 respectively.
Current wandering counts are 1,186 and 1,185. The missing current object is
13926, creature ID 60 at actionable coordinates (62,53,0). Before defeat,
its tile starts at 392,271. Bytes +8..+18 are
`36 00 3C 00 66 36 05 02 00 00`: class 54, subtype 60, object index 13926,
property word 517. This word is not verified as a creature quantity.

Both tile arrays start at 2,263. Their ends are 3,551,767 and 3,551,743:
the render lists lose 24 bytes. The entire before range
`[3,551,767, 4,394,911)` equals the after range
`[3,551,743, 4,394,887)` byte for byte. This includes the early current object
and property tables, not the later monster table: defeat does not establish a changed quantity field
there. Later hero, visibility and replay changes are separate from the
removed monster's identity; a naive unaligned whole-file diff is invalid.

The pair proves disappearance of this actionable monster, not a general
single-save defeated flag or a safe record deletion/zeroing operation.
The survivor pair below separately establishes the current quantity.

## H3F-027 native survivor table and edits

User-confirmed state: creature ID 60 at (62,53,0), object 13926, 72 before
and 56 in `SaAndVla_after_56.GM2`, still present. Before payload length is
5,015,467; surviving-after length is 5,015,770. The monster table has a u32
count of 1,899 at 4,930,736; 33-byte records start at 4,930,740. Tile property
517 indexes this table: quantity is the u32 at 4,947,801. In the after save,
replay growth shifts it by 303 bytes to 4,948,104. The aligned tail differs
only at the byte changing 72 to 56. These offsets are evidence, not constants
used by the locator. Original SaAndVla has count 787 at 5,235,111 and records
at 5,235,115; all 291 active groups resolve to positive current counts.

The locator requires a unique complete count-prefixed 33-byte chain after
the parsed heroes, bounded count <=65,535 and unique in-range active property
indexes. Conservative record checks (not a claim of complete field semantics):

| Offset | Size | Interpretation / accepted shape |
| --- | --- | --- |
| +0 | 4 | Current quantity u32, active editable range 1..2,147,483,647 |
| +4 | 1 | Opaque, <=15 |
| +5 | 4 | Disposition-shaped signed value: -4 or 1..10 |
| +9, +10 | 1 each | Boolean-shaped values |
| +11 | 4 | Opaque reference, preserved |
| +15 | 1 | Boolean-shaped value |
| +16 | 4 | Observed constant 100 |
| +20 | 4 | Signed value -1..1 |
| +24 | 4 | Signed value -1..7 |
| +28 | 1 | Boolean-shaped value |
| +29 | 4 | Nonnegative signed value, opaque |

Only +0 is patched for quantity changes. All remaining bytes, including
aggression/joining/reward/growth settings, remain unchanged. Nonmatching
variants are read-only rather than assigned invented semantics.

Type replacement requires a catalogued existing template with the same
33 fixed template bytes except the u16 subtype at +28. If multiple matching
templates for a target ID differ in DEF name or fixed bytes, that target is
excluded. The checked patches update tile subtype u16 at tile+10 and current
object template index u16 at object+3. No shared template is mutated; identical
dimensions/action/visibility masks preserve existing render references.
Quantity is a separate checked four-byte patch. Two independent groups pass
exact-diff and compressed save/reopen tests. Native loading of edited types
remains a manual validation step. No missing template, army, or object is
created. Wandering deletion/zeroing, splitting and resurrection are refused.

## H3F-019 coordinate movement evidence

2026-11-12, HotA 44/11, Element Land, 252 x 252 x 2. All offsets below
are decompressed offsets in the supplied movement saves, not write addresses
for other saves. Originals were read only; analysis/test copies are ignored.

| Private input | Compressed SHA-256 | Payload length |
|---|---|---:|
| `SaAndVla_before_movement.GM2` | `a617ec6293febf02db9ab469c1c960fe07525b5e3632e5777a5861a60aa59d65` | 5,012,342 |
| `SaAndVla_moandor_left_1.GM2` | `79190fe9f221a07b839b7642618b3394472e7cb1d61af571f829250df502552a` | 5,012,397 |
| `SaAndVla_nimbus_water_step.GM2` | `5ff56bed7daf260645fcb8073f10a9b882a419a5795a18fe05c61706cfeabd95` | 5,012,409 |

The two after-saves branch independently from the baseline. Moandor (type 66,
record start 4,381,921) moves (231,52,0) -> (230,52,0). Nimbus (type 75,
record start 4,392,361) moves (76,41,0) -> (77,42,0). Both biographies are
empty, so front matter starts at record -26. The existing h3svg reference
(`.local/research/h3svg_spec.md`, Hero/Tile/Boat/TileVisibility/ReplayEvent)
supplies structure names, with native differences providing the evidence below.

### Exhaustive changes before replay insertion

Ranges in this table may include unchanged bytes between fields; the regression
in `tests/sample_coordinates.rs` asserts the exact differing byte indexes.
No differences outside these ranges precede the insertion at 4,921,359.

| Moandor pair offset | Change | Interpretation |
|---|---|---|
| 908-926 | embedded filename | incidental save metadata |
| 389,550-551 | `02 03 -> 12 22` | destination tile +7 actionable flag, +8 class 3 -> hero 34 |
| 389,557-560 | `FFFFFFFF -> 42000000` | destination properties -> hero type 66 |
| 389,576-577 | `12 22 -> 02 03` | source actionable flag cleared, class restored to 3 |
| 389,583-586 | `42000000 -> FFFFFFFF` | source properties restored |
| 4,277,472 | `4B -> 42` | Pink selected hero: Nimbus -> Moandor; incidental UI state |
| 4,381,895; 4,381,902 | `E7 -> E6` | x u16 and duplicated packed x |
| 4,381,936 | `05 -> 06` | hero +15 facing, left |
| 4,381,940-947 | (229,50) -> (-1,-1) | hero +19/+23 destination X/Y cleared |
| 4,381,956 | movement remaining 1760 -> 1685 | hero +35, 75-point step; maximum stays 1760 |
| 4,382,990 | `00 -> 01` | hero +1069, movement-associated flag; exact semantics unresolved |
| 4,392,380; 4,392,384 | Nimbus destination (76,39) -> (77,41) | other hero's route changed, position/movement unchanged; incidental to Moandor step, cause unproven |
| 4,872,060 | replay count 2104 -> 2107 | three appended records |

Moandor's source tile starts at 389,569; destination at 389,543. His
`object_class_under` remains 3, `has_object_under` 0 and properties FFFFFFFF.
This establishes passive class-3 restoration; destinations with a different
underlying class/properties remain excluded. The external ObjectClass enum calls
3 ANCHOR_POINT, but no actionable anchor interaction is inferred or enabled.

| Nimbus pair offset | Change | Interpretation |
|---|---|---|
| 908-928 | embedded filename | incidental save metadata |
| 308,066-067 | `10 22 -> 00 00` | source tile actionable flag cleared, class hero -> 0 |
| 308,073 | `4B -> 00` | source properties restored to zero |
| 314,964-965 | `00 00 -> 10 22` | destination actionable flag set, class -> hero |
| 314,971 | `00 -> 4B` | destination properties -> hero type 75 |
| 4,392,335; 4,392,337 | x 76 -> 77, y 41 -> 42 | u16 front-matter coordinates |
| 4,392,342; 4,392,344 | same x/y change | packed coordinate duplicate |
| 4,392,376 | `00 -> 03` | hero +15 facing, diagonal down-right |
| 4,392,380-387 | (76,39) -> (-1,-1) | destination X/Y cleared |
| 4,392,396 | movement remaining 2500 -> 2359 | hero +35, 141-point step; maximum stays 2500 |
| 4,393,430 | `00 -> 01` | hero +1069, same unresolved movement-associated flag |
| 4,577,680 | `7E -> FF` | visibility-like mask, linked to replay event below |
| 4,872,060 | replay count 2104 -> 2108 | four appended records |

Nimbus's source tile starts at 308,059; destination at 314,957. Both are
water (terrain byte 8). His underlying class/flag/properties stay zero.
Visibility masks `7E -> FF` set bits 0 and 7, not just Pink's bit 7.
The replay names (83,44) and the same masks. If this is the classic two-byte
z/y/x TileVisibility array, it implies a start of 4,555,338. That is a
**hypothesis**, not a verified fog boundary or team-sharing rule, and is not
used for writing. No RNG mutation is established in either pair.

### Replay alignment and unchanged structures

At 4,921,359 the game appends these records (hex), not replacements:

| Pair | Size | Bytes |
|---|---:|---|
| Moandor | 22 | `0c071000000008000000420000000500000006000000` |
| Moandor | 18 | `0c070c000000090000000100000042000000` |
| Moandor | 15 | `01074200000006e7003400e6003400` |
| Nimbus | 22 | `0c0710000000080000004b0000000000000003000000` |
| Nimbus | 18 | `0c070c00000009000000010000004b000000` |
| Nimbus | 15 | `01074b000000034c0029004d002a00` |
| Nimbus | 12 | `0b07010053002c007e00ff00` |

Type 12 records carry facing/flag-like updates; type 1 contains hero type,
facing and old/new x/y. Type 11 corresponds to the visibility-like change.
Subevent semantics beyond these direct correspondences remain unverified.
After aligning these 55/67 inserted bytes, the entire 90,983-byte tail is
identical. There are no additional hidden tail mutations in these pairs.

The tile array starts at 2,263 and walks exactly 127,008 variable-length
tiles to templates at 3,550,135. Tile render counts/lists do not change.
All bytes from templates through the end of boats/obelisks (Player[0] at
4,276,427) are unchanged. All town records and owner/hire tables are unchanged.
Hero object indexes on the occupied tiles remain FFFF; no separate object-array
entry moves in these two samples. Do not extrapolate this to towns/objects or
boarding/disembarking.

The boat table starts at 4,275,482 with 32 records. Nimbus rides boat 30,
record 4,276,322: exists 1, owner 7, rider 75, boarded 1, visible 0,
direction 1, stale x/y/z (70,52,0), stale packed coordinates. All 28 bytes
are identical before and after his water step. His actual position is in
the hero front matter and current tile occupancy, not these boat coordinates.

### Relocation interpretation and implementation

The initial H3F-019 conclusion that another one-step land pair was required
was too broad. The existing pairs already establish the current-position and
occupancy relationships. A native step's full diff is not the desired patch set
for an editor teleport. The following distinctions control the implementation:

| State | Editor relocation |
|---|---|
| Front x/y/z and packed duplicate | update together |
| Source tile actionable/class/properties | restore from hero's underlying state |
| Destination tile actionable/class/properties | replace with hero occupancy |
| Tile subclass/index, render list, terrain | preserve; not copied from source to destination |
| Hero underlying class/flag/properties | preserve; only matching passive destinations accepted |
| Destination X/Y | clear to -1/-1, as in both native pairs |
| Destination Z and next three bytes | preserve; Z does not change in supported moves |
| Hero +1069 | accept only 0/1, set 1 as in both pairs; exact semantic name still unknown |
| Facing, movement totals/remaining and land/sea movement words | preserve, explicit editor policy; no path-distance cost |
| Boat record | preserve unchanged, including stale parked coordinates; no attach/detach |
| Owner roster, owner table, town links | validate relevant links, preserve |
| Other-hero route, selected hero, embedded filename | incidental to this hero's relocation, preserve |
| Fog and replay | preserve by editor policy; no simulated revelation or playback events |

This does not claim that exploration is incidental to native movement: Nimbus's
fog update is real. It is a separately omitted effect of the editor operation.
The exact fog-array boundary/sharing rule is still unverified, but no guessed
fog patch is needed to change position while retaining existing exploration.

**Current passability versus historical objects.** Moandor's before-save contains
old object 11999 (monster template AVWddrx0.def) and object 14391 (resource template
avtmerc0.def) at (231,52,0). Their masks would incorrectly block the demonstrated
move if all object-array entries were assumed active. Neither is in the current
source/destination render lists. Both live tiles render object 10238, a tree
sprite AVLSNTR2.def anchored at (233,53,0); their relative sprite tiles are 0x12
and 0x13 and are passable. Nimbus's destination renders object 11269,
wt_tvrn01.def anchored at (79,45,0), relative sprite tile 0x32, also passable
and not actionable there. The tavern entrance elsewhere does not make this
particular water tile an object visit.

The implementation reuses the bounded full tile/template/object discovery from
H3F-027. It validates current tile +7: bit 0x01 blocks access, bit 0x10 means
an actionable tile, bit 0x02 is preserved as the coastal/access flag; unknown
other access bits refuse relocation. Every current render entry must reference
an existing object/template at the matching anchor and relative sprite tile;
template passability (+8..+14) must allow that cell and action mask (+20..+26)
must not mark it actionable. Occupancy comes from the live tile and other visible
hero records. Adjacent live monster tiles are refused. This intentionally does
not reconstruct pathfinding or trigger encounters.

[H3API H3MapItem](https://github.com/RoseKavalier/H3API/blob/master/include/h3api/H3AdventureMap/H3MapItems.hpp)
corroborates access flags and draw-tile coordinates, and
[H3Hero supplied by the user](https://github.com/RoseKavalier/H3Plugins/blob/master/H3API/lib/h3api/H3Structures/H3Structures.hpp)
corroborates separate position, underlying state, destination and movement fields.
These are classic in-memory layouts, not HotA serialized layouts. Memory
`objectBelow` is +0xB, `objectTypeUnder` +0xC and setup +0x14; the save front
stores class +11, boolean +15 and properties +16. Memory destination +0x35/+0x39
maps conceptually to save-record +19/+23, and memory movement +0x49/+0x4D to
save-record +31/+35. No pointer, vector or memory offset is used as a save offset.
The provided header does not establish the identity of HotA save +1069.
No H3API implementation code was copied.

`SaveDocument::hero_coordinates` requires exact 44/11, high-confidence records,
a unique length-consistent biography boundary within 8192 bytes, boolean fields,
visible=1, full u16 X/Y/Z, packed-coordinate agreement and actual map bounds.
`plan_hero_relocation` adds an owner-roster match, verified town links, unique
current hero tile, known passive underlying class 0/3 with zero/FFFFFFFF
properties, observed inactive spell-state prefix, known movement-associated
byte 0/1, valid terrain and exactly one boarded boat for water heroes.
It rejects different levels, medium changes, differing underlying state,
occupied/actionable/guarded/impassable destinations and ambiguous references.
The inactive prefix at hero +5..+10 must equal FF FF FF 00 FF as in both pairs;
individual HotA spell subfield names/order are not inferred from the classic
header. Patrol radius must be FF. Water without a verified boat is refused.

The supported operation is arbitrary distance, not restricted to one tile:
Moandor (231,52,0) -> (108,77,0) and Nimbus (76,41,0) -> (118,71,0) pass
save/reopen and return-move exact-byte checks. Returning restores the original
payload except the intentionally cleared route and +1069. At the native-step
destinations every patch byte equals the game's byte; the only remaining diffs
are precisely filename/selection/other-hero route and the explicitly preserved
facing, movement cost, fog and replay. Negative tests cover coordinate mismatch,
rock, blocked/actionable tiles, bad render references, unknown movement/spell
state, missing boarded boat, bounds, levels and occupied heroes.

`HeroEdit::SetCoordinates` flows through pending edits and safe save. Two pending
moves cannot reserve the same destination; occupied sources cannot be pending
destinations, so swaps/chains are unsupported. Relocation and dismissal of the
same hero conflict in both orders. Original saves and H3M remain untouched.
Native HotA load/move/end-turn validation remains manual, not claimed by the
parser round trips. Cross-level, boarding/landing, town/object interactions and
changes of underlying shape stay disabled. No repeat one-step experiment is
requested; a boarding pair would provide new medium/boat-attachment information
only when that separate scope is pursued. See [H3F-019](../issues/019-map-coordinates.md).


## H3F-028 - Keymaster Tent visits (HotA 44/11)

Verified with `SaAndVla_before_visit_red_tent.GM2` and
`SaAndVla_after_visit_red_tent.GM2`: Pink visited the Red Tent at
(142,131,1). Original compressed SHA-256:

- Before: `94cc0f08cac1a9db3d239f157245309b5f9018834ff494910f0dba9ff5ffaf45`
- After: `c4daae6aee288a12fa19bf2c1e59be67346db042a6bf6e7868426934ded04314`

### Verified layout and edit boundary

Let `H` be the end of the last located fixed hero record. Require exact 44/11,
215 fully verified heroes ordered by type 0..214, and an owner table at `H+12`
consistent with every hero. The following 215 bytes are the hireability table.
The fixed globals begin at `G = H+12+430`; bounded header/date checks precede
access. The tent table is `G+109`, eight bytes, each a player bitmask.
This is a relative structural anchor, never an absolute sample offset or a
frontend-supplied offset. The classic specification's globals sizes agree:
17 bytes through date, 32 opaque bytes, 28 merchant bytes, 32 object-visit masks,
then eight tent masks. The surrounding globals are never written.

| Color ID | Tent color | Pair offset | Before mask | After mask | Original sprite |
|---|---|---:|---|---|---|
| 0 | Light Blue | 4552402 | 40 | 40 | AVXkey00.def |
| 1 | Green | 4552403 | 08 | 08 | AVXkey10.def |
| 2 | Red | 4552404 | 01 | 81 | AVXkey20.def |
| 3 | Dark Blue | 4552405 | 02 | 02 | AVXkey30.def |
| 4 | Brown | 4552406 | 04 | 04 | AVXkey40.def |
| 5 | Purple | 4552407 | 20 | 20 | AVXkey50.def |
| 6 | White | 4552408 | 7E | 7E | AVXkey60.def |
| 7 | Black | 4552409 | 10 | 10 | AVXkey70.def |

Masks are hexadecimal. Player bits 0..7 are Red, Blue, Tan, Green, Orange,
Purple, Teal, Pink. Tent colors have a DIFFERENT order from player colors.
For this pair `H=4551851`, owner table=4551863, `G=4552293`.
In the older `SaAndVla.GM2`, H=4922142 and tent base=4922693; masks are
`40 58 40 02 50 60 5C 00`. Absolute positions are not compatibility gates.

`TentEdit { player, color, visited }` sets or clears only `1 << player`.
All edits are composed before emitting at most one BytePatch per color, so
changing two players for Red cannot overwrite one another or create overlapping
patches. Duplicate player/color requests and IDs outside 0..7 are rejected.
Expected old bytes, same-length patch validation and existing verified-save
replacement remain enforced. No-op/change-back removes the pending operation.
The planner re-derives anchors from immutable bytes, ignoring mutable public DTOs.

These masks are the player-wide keys for matching Border Guards/Gates. The
editor does not remove guards, restore removed guards, simulate a visit/movement,
reveal fog, update replay history, or propagate the selected player's edit to
allies. Clearing access does not resurrect an already removed guard. There is no
separate tent visit counter in the supplied native action diff. Modified visited
tent states now have a user-reported HotA pass (recorded 2026-11-14), with
players/colors unspecified. Guard/gate interactions remain manually unverified;
only Pink/Red has an observed native before/after transition. The other 63 operations use the
verified shared bitmask structure, enum and current-save templates, with exact
independent-bit round-trip tests, not 63 additional native action pairs.

### Exhaustive aligned pair difference

Compressed sizes: 1,383,951 and 1,383,965 bytes. Payload sizes: 5,012,796 and
5,012,847. A naive same-position comparison reports 61,300 differing bytes in
15,234 runs, plus 51 trailing bytes, because the replay insertion shifts the
suffix. After alignment there are exactly **47 changed bytes and 51 inserted
bytes**. The entire 90,987-byte suffix at before 4921809 / after 4921860 is
identical. No other insertion or deletion is needed.

All replacement runs before that insertion (end offsets exclusive):

| Offset/range | Before hex | After hex | Interpretation |
|---|---|---|---|
| 908..912 | 6265666f | 61667465 | Stored save filename |
| 913..933 | 655f76697369745f7265645f74656e742e474d32 | 5f76697369745f7265645f74656e742e474d3200 | Stored save filename |
| 2764611 | 0a | 22 | Destination tile: tent to hero class |
| 2764617..2764621 | ffffffff | 46000000 | Destination tile: hero type 70 |
| 2771232..2771234 | 1022 | 0000 | Source tile flags/class cleared |
| 2771239..2771243 | 46000000 | ffffffff | Source tile properties cleared |
| 4383620 | 8f | 8e | Hero X: 143 to 142 |
| 4383622 | 84 | 83 | Hero Y: 132 to 131 |
| 4383627 | 8f | 8e | Packed coordinate X |
| 4383629 | 84 | 83 | Packed coordinate Y |
| 4383631 | 00 | 0a | Object-under class becomes tent |
| 4383635 | 00 | 01 | Object-under boolean |
| 4383661 | 00 | 07 | Hero facing |
| 4383681..4383683 | 7204 | e503 | Remaining movement 1138 to 997 |
| 4552404 | 01 | 81 | Red Tent: Pink's bit set |
| 4869145 | 7d | 80 | Replay total 2429 to 2432 (u32) |
| 4921675 | 04 | 05 | Existing replay-local counter; not a tent counter |

At 4921809 the after file inserts these 51 bytes:

`0c071000000008000000460000000000000007000000010746000000078f0084048e0083040c07080000000600000001000000`

Three replay records: type 12 (22 bytes, hero/facing update), type 1 (15 bytes,
hero 70, facing 7, old/new packed coordinates), type 12 (14 bytes, subkind 6,
value 1). Full replay subkind semantics remain unverified; these records and
both replay counters are incidental to editor access changes and are preserved.
All fog/visibility, resources, other player state, object templates, other hero
records, town records and later tail bytes are identical after alignment. No RNG
mutation is established by this pair.

The current save's tile at 2764603 has class 10/subtype 2 and its object table
contains exactly the supplied (142,131,1) Red Tent. All eight current-save class-10
templates map color c to `AVXkey{c}0.def`. The existing local H3M reference also
agrees on all eight subtype/sprite mappings, but has different tent coordinates;
it is not claimed to be this pair's exact scenario and is never a write anchor.

Sources: [h3svg SavedGame specification](https://github.com/alexanderbelous/h3mtxt/blob/h3svg/doc/h3svg_specification.MD)
and [KeymastersTentType enum](https://github.com/alexanderbelous/h3mtxt/blob/h3svg/src/h3mtxt/SavedGame/Constants/KeymastersTentType.h).
These are format facts only; no third-party implementation was copied.

## H3F-030 - Initialized teams and native CRC evidence (2026-11-14)

Status: verified initialization comparison, not verified ongoing alliance editing.
The supplied Crimson and CloverA_blue_in_2_team.h3m puts Blue in zero-based
team 1; the original Crimson and CloverA.h3m puts Blue in team 0. Both have
two teams. Rust map parsing confirms the other playable colors retain teams:
Red 0, Tan/Orange 1. The copied map also has a longer name and was converted
from RoE map format 14 to HotA format 32, so it is not a one-byte H3M diff.

A bounded research walk through the GM2 name/description, eight player specs,
normal victory/loss bytes, team count/array, zero custom-hero count and
starting-settings section corroborates the initialized team fields:

| Save | Count offset/value | Assignment offsets | Zero-based assignment bytes |
|---|---|---|---|
| same_teams, same_teams_restart, separate_teams | 474 / 2 | 475..482 | 0,0,1,0,1,0,0,0 |
| separate_team_map_2 | 476 / 2 | 477..484 | 0,1,1,0,1,0,0,0 |

The two-byte shift follows the longer map name; Blue changes from 0 at 476
in the old sample to 1 at 478 in the new sample. These are sample-specific
payload offsets, not constants for production parsing. The research walk is
restricted to this day-1 normal-victory/loss layout. Current alliance authority,
team-related duplicates/caches and sharing updates remain unverified. No team
writer or general-purpose header parser was added. No whole-save serialization
or original H3M edits are involved.

The new save is 79,999 compressed bytes / 580,032 payload bytes, SHA-256
9594a96725e5f692274aced96efd0e6db42d6e30cf43ee81587d3b40ff3342ad.
Stored/computed CRC32: EB55EEF7 / F8EAE189. The previous three native CRC
pairs are 51023D2A / 3C8830CD, E933111B / 74EA2243, 131D5339 / 13A89BC7.
See A.2 for the narrow import exception and mandatory strict output checks.

All four now open with 215 heroes. Existing town discovery returns 2 towns
and no verified player table for same_teams and separate_teams; it returns
6 towns and 8 players for same_teams_restart and separate_team_map_2. No
additional parser assumption was introduced to force missing player records.
The checksum exception alone does not grant player editing support.

### H3F-030 town/player discovery correction

Verified in the supplied native 44/11 saves: town names are length-prefixed
legacy text, so FF is not a string terminator. In same_teams.GM2 the third
town at 299111 has the 14-byte CP1251 name "Упавшая Звезда", including FF.
In separate_teams.GM2 the third town at 299119 has the 16-byte name
"Забытая Крепость", also including FF. The inherited name grammar rejected
both, ending the dense town scan after two records. Player discovery then
correctly rejected the incomplete town table because its declared count is 6.

The bounded reader now permits names through 16 bytes and FF text bytes.
This is the evidenced supported range, not a claim about all possible names.
Names still reject control bytes, leading spaces, out-of-range lengths and
nonzero high length bytes. Record/name reads respect both the payload end
and hero-section boundary; partial records are not clamped into valid spans.
Town garrison, coordinate and ownership checks are unchanged. The player
parser still requires the full count, contiguous towns, eight color IDs and
verified hero/town ownership links. No player resource offsets changed.

All four supplied saves now yield 6 high-confidence towns and 8 resource-editable
player slots. Across these fixtures, 224 independent resource edits preserve
every payload byte outside the selected documented four-byte word, including
all other players and team data. Save As/Save, backups and source-change rejection
pass through the application session. Existing larger-sample cross-validation
also passes with 48 map/save towns, all names, positions and fixed factions
matched. This supersedes the earlier H3F-030 observation of missing players.
