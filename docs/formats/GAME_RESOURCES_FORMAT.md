# Heroes III / HotA game resource formats

Technical specification of the resource files H3Forge reads: LOD archives,
the Heroes-specific "PCX" image format, DEF sprite containers, and the
portrait/icon mappings built on them. Self-contained: layouts are stated
directly; provenance is in the appendix.

Confidence vocabulary:

| Class | Meaning |
|---|---|
| VERIFIED_IN_H3FORGE | Demonstrated by `crates/h3forge-resources` or its tests against the local archives. |
| VERIFIED_EXTERNAL | Supported by an external implementation; not re-validated locally. |
| PARTIAL | Some parts understood; boundaries stated. |
| UNKNOWN | Insufficient evidence. |

Reference implementation: `crates/h3forge-resources/src/` - `lod.rs`,
`pcx.rs`, `def.rs`, `creature_portraits.rs`, `catalog.rs`, `discovery.rs`,
`png.rs`. All integers are little-endian unless stated otherwise.

Local archives under test (workspace `Data/`, byte-identical to the real
installation at `G:\Games\Heroes3_HotA\Data` for `HotA.lod`):

| Archive | Bytes | Directory |
|---|---:|---|
| `H3bitmap.lod` | 109,587,195 | standard, version 200, 4,912 entries (4,809 PCX) |
| `H3sprite.lod` | 65,352,134 | standard, version 200, 4,013 entries (2,565 DEF, 1,448 MSK) |
| `HotA.lod` | 111,378,616 | **nonstandard directory - rejected (section B)** |
| `HotA_lng.lod` | 9,061,660 | standard, version 200, 225 entries |
| `HotA_ext.lod`, `HotA_l_ext.lod` | 92 each | valid empty version-200 headers, 0 entries |
| `h3ab_bmp.lod`, `h3ab_spr.lod` | 0 each | empty files - rejected `EmptyArchive` |

---

## A. LOD archives

### A.1 Header and directory (VERIFIED_IN_H3FORGE, adapted from mmarch)

A standard Heroes III LOD has a 92-byte header followed by a directory of
32-byte records; entry payloads follow the table. `data offset` values are
**absolute file offsets**.

Header:

| Offset | Size | Field | Meaning |
|---|---:|---|---|
| 0 | 4 | magic | `4C 4F 44 00` ("LOD\0"). Mismatch → `BadMagic`. |
| 4 | 4 | version | u32 LE; Heroes III archives carry 200 (0xC8). Values > 0xFFFF indicate MM6/7/8-format LODs - out of scope, rejected `UnsupportedFormat`. |
| 8 | 4 | entry count | u32 LE; > 100,000 → `EntryCountLimit`. |
| 12 | 80 | reserved | Zero-filled in observed archives; not interpreted. |

Directory record (32 bytes, repeated `count` times immediately after the
header):

| Offset | Size | Field | Meaning |
|---|---:|---|---|
| 0 | 16 | name | ASCII, NUL-terminated or full-length; must be printable 0x20–0x7E and non-empty |
| 16 | 4 | data offset | u32 LE, absolute file offset of the stored data |
| 20 | 4 | unpacked size | u32 LE, decompressed byte count |
| 24 | 4 | unused | Not interpreted by H3Forge (mmarch also ignored it). |
| 28 | 4 | packed size | u32 LE; **0 means the entry is stored uncompressed**, otherwise this is the stored (zlib) size |

Validation at open (metadata only - no payload bytes are read):

- File must be non-empty, ≥ 92 bytes, and `92 + count*32 ≤ file_len`
  (`DirectoryTruncated` otherwise).
- Every entry's `[offset, offset + stored_size)` must lie inside the file and
  at/after the directory end (`UnsupportedDirectory` for record 0 -
  the HotA.lod signature - `InvalidDirectory` later).
- `stored_size == 0 && unpacked_size != 0` is rejected.
- Per-entry caps: stored and unpacked size each ≤ 32 MiB (`EntrySizeLimit`).

### A.2 Reading entries

`LodArchive::read_entry(index)`:

1. Seek to the entry's absolute offset, read exactly `stored_size` bytes.
2. Unpacked (`packed_size == 0`): return the bytes as-is.
3. Packed: zlib-decompress with bounded semantics adapted from mmarch
   `zlib_decompress_bounded()`:
   - The declared `unpacked_size` is the contract: decompress until exactly
     that many bytes are produced. Trailing bytes after the zlib stream are
     ignored (palettes/padding behind the stream survive); the stream is never
     required to consume its input.
   - Sanity cap before allocation: `unpacked_size ≤ stored_size * 1032 + 1024`
     (deflate cannot expand more than ~1032:1); violation → `ImpossibleSize`.
   - Output shorter than declared → `DecompressTruncated`; anything else
     inconsistent → `CorruptCompression`.

### A.3 Lookup semantics

- `entries()` returns the directory in archive order.
- `find_entry(name)` matches **case-insensitively** (real archives contain
  mixed-case names such as `HPL130Kn.pcx`); more than one match is an
  `AmbiguousName` error rather than a silent pick.
- Archives are opened `Read + Seek`; only the 92-byte header and 32-byte
  records are read at open. Individual entries are read on demand - full
  archives are never slurped or extracted.

### A.4 Real archive examples

From the local `Data/` (VERIFIED_IN_H3FORGE):

| Archive | Result |
|---|---|
| `H3bitmap.lod` | opens; 4,912 entries enumerated; hero portraits `HPL000KN.PCX`… present |
| `H3sprite.lod` | opens; 4,013 entries enumerated (DEF/MSK); individual entries (TwCrPort.def, SpellInt.def…) read on demand |
| `HotA_lng.lod` | opens; 225 entries (PCX, DEF, text, campaigns); `CAMPBACK.pcx` decoded |
| `HotA_ext.lod` / `HotA_l_ext.lod` | open with zero entries |
| `h3ab_bmp.lod` / `h3ab_spr.lod` | zero bytes → `EmptyArchive` (also zero bytes in the real install - legitimate) |
| `HotA.lod` | opens; 5,232 entries with opaque 16-byte IDs; stored/zlib entries read on demand (section B); LZMA unsupported |

### A.5 Error taxonomy

`ResourceError`: `EmptyArchive`, `BadMagic`, `UnsupportedFormat {version}`,
`DirectoryTruncated`, `EntryCountLimit`, `UnsupportedDirectory {reason}`,
`InvalidDirectory {index, reason}`, `EntrySizeLimit`, `EntryNotFound`,
`AmbiguousName`, `BadIndex`, `ImpossibleSize`, `DecompressTruncated`,
`CorruptCompression`, plus PCX/DEF-specific variants below and `Io`.

## B. HotA.lod: the opaque-ID directory (VERIFIED_IN_H3FORGE)

Status: **supported for stored (method 0) and zlib (method 3) entries**.
Method 2 (raw LZMA1) and any other method fail with
`UnsupportedCompression` on read. Supersedes the first pass's
"explicitly unsupported" classification (H3F-014).

Both local copies (workspace `Data/HotA.lod`, installed
`G:/Games/Heroes3_HotA/Data/HotA.lod`) are byte-identical, 111,378,616
bytes. The layout below was validated against the reference description
(homm3data `lodfile.py`, MIT - provenance appendix) and independently
re-verified on the local bytes.

### B.1 Header and directory

The 92-byte header keeps the standard magic and version/count words; the
directory starts at **80**, not 92:

| Offset | Size | Type | Field | Notes |
|---|---|---|---|---|
| 0 | 4 | bytes | magic `LOD\0` | standard |
| 4 | 4 | u32 LE | version | 200 locally |
| 8 | 4 | u32 LE | entry count | 5,232 locally |
| 0x0C | 4 | u32 LE | XOR key | 0xB5A4D744 locally; standard archives carry 0 or 0x007E0213 here |
| 0x10 | 76 | bytes | unused | not interpreted |

Directory: count × 32-byte records starting at offset 80, immediately
followed by entry data.

| Record offset | Size | Type | Field | Notes |
|---|---|---|---|---|
| 0 | 16 | bytes | opaque ID | not a filename; exposed as 32 lowercase hex digits, never invented names |
| 16 | 4 | u32 LE | data offset | XOR key |
| 20 | 4 | u32 LE | unpacked size | XOR key |
| 24 | 4 | u32 LE | packed size (0 = stored) | XOR key |
| 28 | 1 | u8 | compression method | **not** XORed: 0 stored, 3 zlib, 2 raw LZMA1 (unsupported) |
| 29 | 3 | bytes | unknown | retained/ignored |

Semantics identical to standard entries otherwise: bounded zlib output to
the declared unpacked size, range/allocation checks before use. Detection
order: a structurally valid standard first record at offset 92 wins over
the HotA detector, so ordinary archives (including the historical
0x007E0213 reserved word) are never misclassified. Local census: 5,232
records - 3,869 zlib, 1,363 stored, **zero LZMA**.

### B.2 The D32F image container

HotA stores some interface artwork - including the HotA-only secondary
skill icons - not as DEF/PCX but in a `D32F` container (little-endian word
`0x46323344`). Verified from local entry `48043b9164e06a6f8e626a6b6ca7caa1`;
reverse-engineered from local bytes, no third-party decoder reused.

32-byte header:

| Offset | Size | Type | Field | Notes |
|---|---|---|---|---|
| 0 | 4 | u32 LE | magic | `0x46323344` ("D32F") |
| 4 | 4 | u32 LE | version | 1 |
| 8 | 4 | u32 LE | constant | 24 |
| 12 | 4 | u32 LE | canvas width | 32 in the verified resource |
| 16 | 4 | u32 LE | canvas height | 32 |
| 20 | 4 | u32 LE | group count | 1 |
| 24 | 4 | u32 LE | constant | 8 |

Group record:

| Offset | Size | Type | Field | Notes |
|---|---|---|---|---|
| 0 | 4 | u32 LE | record size | `16 + 17 × frame count` (self-describing) |
| 4 | 4 | u32 LE | group id | 0 |
| 8 | 4 | u32 LE | frame count | 93 in the verified resource |
| 12 | 4 | u32 LE | constant | 4 |
| 16 | 13 × count | bytes | frame names | NUL-padded printable ASCII |
| 16+13c | 4 × count | u32 LE | frame record offsets | absolute, within the entry |

Frame record (40-byte header + pixels):

| Offset | Size | Type | Field | Notes |
|---|---|---|---|---|
| 0 | 4 | u32 LE | header size word | 32 |
| 4 | 4 | u32 LE | pixel bytes | `sw × sh × 4` |
| 8 | 4 | u32 LE | full width | equals canvas width |
| 12 | 4 | u32 LE | full height | equals canvas height |
| 16 | 4 | u32 LE | stored width sw | |
| 20 | 4 | u32 LE | stored height sh | |
| 24 | 4 | u32 LE | x placement | |
| 28 | 4 | u32 LE | y placement | |
| 32 | 4 | u32 LE | constant | 8 |
| 36 | 4 | - | unused | |
| 40 | sw×sh×4 | bytes | pixels | BGRA, rows bottom-up |

Decode: place the `sw × sh` block at (x, y) on the canvas; unpainted canvas
pixels stay transparent; convert BGRA → RGBA with a vertical flip.
VERIFIED fact: the six skill-icon frames are **fully opaque** (alpha 255
everywhere; drawn square backgrounds) - unlike `secsk32.def`, whose
palette index 0 is transparent.

### B.3 HotA-only skill icons (Interference 28, Runes 29)

Container `48043b9164e06a6f8e626a6b6ca7caa1`, one group of 93 frames named
`secsk32._000`..`secsk32.A092` on a 32×32 canvas:

| Skill | Binary ID | Basic | Advanced | Expert |
|---|---|---|---|---|
| Interference | 28 | frame 87 `secsk32.A087` | 88 `secsk32.A088` | 89 `secsk32.A089` |
| Runes | 29 | frame 90 `secsk32.A090` | 91 `secsk32.A091` | 92 `secsk32.A092` |

Formula: `frame = 2 + mastery + 3 × binary ID` (mastery 1–3) - the
base-game SECSK32 formula continued. Identification evidence (H3F-014,
2026-11-09): frames 87–89 decode **pixel-exactly** (RGBA SHA-256 equal) to
the verified Interference references published in the VCMI HotA mod
repository; frames 90–92 match the Runes references by identical alpha
silhouettes and per-frame minimum RGB distance on the correct
(frame, mastery) diagonal (the published Runes reference PNGs are
re-brightened re-encodes of the same art, which is why they are not
byte-identical). Frames are selected by **exact embedded name**, never by
directory position. Implementation: `hota32.rs` (bounded parser),
`skill_icons.rs` (`HotASkillIcons`); exact decoded PNG bytes pinned by
CRC-32 in `tests/hota_skill_icons.rs`.

## C. Heroes III "PCX" images

Despite the extension, this is **not** the ZSoft PCX format. It is a raw
raster: a 12-byte header of three little-endian i32s, raw pixel data, and for
8-bit images a 768-byte RGB palette (RSLod.pas `TRSLod.UnpackPcx` lineage;
adapted from mmarch `image.rs::decode_pcx`). VERIFIED_IN_H3FORGE.

### C.1 Layout

| Offset | Size | Field | Meaning |
|---|---:|---|---|
| 0 | 4 | i32 | image size in bytes = `width * height * bytes_per_pixel` |
| 4 | 4 | i32 | width in pixels, > 0 |
| 8 | 4 | i32 | height in pixels, > 0 |
| 12 | image size | pixels | 1 byte/pixel (indexed) or 3 bytes/pixel (24-bit), row-major, top-down, no RLE and no row padding |
| 12+size | 768 | palette | indexed images only: 256 × R,G,B |

Bytes-per-pixel is derived, never stored: `image_size / (width*height)` must
be exactly 1 or 3. There is no row stride beyond `width * bpp` (rows are
tightly packed). Bounds: all three header ints > 0; pixel count ≤
`max_image_pixels` (16,000,000 default); `12 + image_size ≤ data.len`;
indexed images additionally require the 768-byte palette
(`PcxTruncated`/`PcxMissingPalette`/`PcxBadDimensions`/`PcxBadPixelLength`/
`PixelLimit` otherwise).

### C.2 Colour and transparency

- 8-bit: pixels are palette indices; the palette is 256 R,G,B triplets.
- 24-bit: pixels are **B,G,R** triplets - the buffers feed directly into
  Windows 24-bit bitmap byte order (confirmed on the sample by pinned pixel
  values).
- `to_rgba()` converts to straight RGBA with **alpha always 255**: no
  transparency convention for Heroes PCX has fixture evidence. (Do not infer
  transparency from a cyan-looking palette entry; e.g. `HPL000KN.PCX` has
  first palette entry cyan 0,255,255 and renders opaque.)

### C.3 Verified examples

| Entry | Archive | Decoded form | Byte check |
|---|---|---|---|
| `HPL000KN.PCX` (58×64) | `H3bitmap.lod` | indexed | unpacked 4,492 = 12 + 3,712 + 768; first palette entry cyan |
| `CAMPBACK.pcx` (800×600) | `HotA_lng.lod` | 24-bit BGR | unpacked 1,440,012 = 12 + 800·600·3 |

## D. DEF sprite containers

DEF is the packed sprite format used inside `H3sprite.lod` (and
`HotA_lng.lod`). H3Forge implements a bounded reader for **interface-type
(0x47) DEFs** with compression formats 0 (raw) and 1 (per-row RLE);
`def.rs` was written from the format structure cross-checked against VCMI
`client/render/CDefFile.cpp`. No third-party decoder code is copied.

### D.1 File layout

| Offset | Size | Field | Meaning |
|---|---:|---|---|
| 0 | 4 | u32 | type. VCMI enum: 0x40 SPELL, 0x41 SPRITE, 0x42 CREATURE, 0x43 MAP, 0x44 MAP_HERO, 0x45 TERRAIN, 0x46 CURSOR, **0x47 INTERFACE** (H3Forge's supported type), 0x48 SPRITE_FRAME, 0x49 BATTLE_HERO |
| 4 | 4 | u32 | full canvas width |
| 8 | 4 | u32 | full canvas height |
| 12 | 4 | u32 | group count |
| 16 | 768 | 256 × R,G,B | palette (one 3-byte entry per index) |
| 784 | per group | group blocks | see below |

Group block (repeated `group count` times):

| Offset in block | Size | Field |
|---|---:|---|
| 0 | 4 | u32 group ID |
| 4 | 4 | u32 frame count in this group |
| 8 | 8 | unknown / skipped (VCMI: "8 unknown bytes") |
| 16 | count × 13 | frame names: 13 raw bytes, NUL-terminated ASCII |
| 16+13·count | count × 4 | u32 frame data offsets, absolute within the DEF |

Frame data (at the recorded offset; 32-byte header then pixel data):

| Offset | Size | Field |
|---|---:|---|
| 0 | 4 | u32 size of the pixel data region following this header |
| 4 | 4 | u32 pixel format: 0 raw, 1 row-RLE, 2/3 other RLE schemes (below) |
| 8 | 4 | u32 full canvas width |
| 12 | 4 | u32 full canvas height |
| 16 | 4 | u32 frame width |
| 20 | 4 | u32 frame height |
| 24 | 4 | u32 left margin (X offset of frame within canvas) |
| 28 | 4 | u32 top margin (Y offset) |

Validation: frame data must not overlap the directory (`offset ≥ end of
group blocks`); dimensions non-zero and within `max_image_pixels`; the frame
crop `(x..x+w, y..y+h)` must lie inside the canvas.

### D.2 Pixel formats

- **Format 0 - raw:** `height` rows of `width` literal palette indices.
- **Format 1 - per-row RLE (VERIFIED_IN_H3FORGE):** a `height`-entry u32 row
  offset table first; each row starts at its table offset (must be ≥ the table
  size) and is a sequence of packets: `{type u8, length_minus_1 u8}`. `type ==
  0xFF` → copy `length` literal bytes; otherwise fill `length` pixels with
  `type` (a palette index). Packets must exactly cover the row width; a raw
  run may not overrun the row.
- **Format 2 (VERIFIED_EXTERNAL, VCMI):** starts with a u16 base offset;
  each row is a sequence of single-byte segments: `code = segment / 32`,
  `length = (segment & 31) + 1`; `code == 7` → copy `length` literal bytes,
  otherwise fill `length` pixels with `code`.
- **Format 3 (VERIFIED_EXTERNAL, VCMI):** per-row u16 offsets, addressed per
  32-pixel band (`offset table entry i` at `base + i*2*(width/32)`); the same
  5/3-bit code/length segments as format 2.

VCMI also special-cases two "old format" defs (`SGTWMTA.DEF`, `SGTWMTB.DEF`):
format-1 frames whose stored width/height exceed the canvas are re-interpreted
without margins (16-byte shorter header). H3Forge does not implement formats
2/3 or the old-format special case - such frames fail with
`DefCompression`/`DefInvalid`, never a wrong image.

### D.3 Transparency

For interface-type defs, palette **index 0 is transparent** (`to_rgba` emits
alpha 0 for index 0, 255 otherwise), with the canvas outside the frame crop
also index 0. Shadow/selection semantics used by map/battle def types are not
supported.

### D.4 Pipeline diagram

```
LOD archive (H3sprite.lod)
  └─ read_entry("TwCrPort.def")  → raw DEF bytes
       └─ DefFile::parse          → palette + group/frame directory
            └─ decode_named(name) → canvas indices (58×64 for creature portraits)
                 └─ to_rgba        → RGBA (index 0 transparent)
                      └─ encode_png → PNG bytes for the frontend
```

### D.5 Verified DEF metadata (local archives, headers only)

| Entry | Archive | Unpacked size | Canvas |
|---|---|---:|---|
| `TwCrPort.def` | `H3sprite.lod` | 624,230 | 58×64, 1 group, 151 frames (all compression 1) |
| `CPRSMALL.def` | `H3sprite.lod` | 152,893 | 32×32 |
| `SpellInt.def` | `H3sprite.lod` | 158,256 | 48×36 |
| `Artifact.def` | `H3sprite.lod` | 194,554 | 44×44 |

All four start with type 0x47 and one group. VERIFIED_IN_H3FORGE.

---

## E. Creature portraits

Verified mapping from creature ID to portrait frame (VERIFIED_IN_H3FORGE,
H3F-006 partial; cross-checked against the
[Creature portraits](https://heroes.thelazy.net/index.php/Creature_portraits)
table, external display reference).

Container: `TwCrPort.def` inside `H3sprite.lod` (151 frames, all compression
1, canvas 58×64). Frame names normally follow `CPrL{id:03}{suffix}.pcx`; the suffix is
determined by the creature ID range:

| ID range | Suffix |
|---|---|
| 0–13 | C |
| 14–27 | R |
| 28–41 | T |
| 42–55 | I |
| 56–69 | N |
| 70–83 | D |
| 84–97 | S |
| 98–111 | F |
| 112–121, 123, 125, 127, 129–131 | E |
| 132–144 | N |

IDs outside these ranges (including blanks, war machines and reserved
entries) have no frame and resolve to a placeholder. **141 standard creature
IDs resolve.**

Correction verified 2026-11-14: the Inferno portrait numbers are not creature
IDs for Gog/Magog and Hell Hound/Cerberus. IDs 44, 45, 46, 47 resolve to
`CPrL046I.pcx`, `CPrL047I.pcx`, `CPrL044I.pcx`, `CPrL045I.pcx`, respectively.
Local frame inspection and Xyron's supplied native screenshot establish the
identities; the standard PNG CRC-32 values are `e83e6686`, `40aadf17`,
`81da69e4`, `aff3baad` in that creature-ID order. This is artwork mapping,
not a permutation of saved army slots.

### E.1 HotA creature portraits (VERIFIED_IN_H3FORGE, 2026-11-10)

The `id+2` rule below uses the corrected portrait number for IDs 44..47:
their D32F frame indices are 48, 49, 46, 47, respectively. Visual inspection
on 2026-11-14 confirmed the same Inferno exception in both containers.
The original comparison established correspondence between the containers,
but did not establish that those four portrait numbers equal creature IDs.

HotA 1.8 ships full creature artwork (base + HotA-exclusive IDs 0–199) in the
`HotA.lod` D32F container `eb1f19b13b23e32ded0fd482e7266b54` - 202 frames,
58×64 canvas, frame names `twcrport_000`, `twcrportA001`…`twcrportA201`.

Mapping: **creature ID `i` → frame `i + 2`** (`hota_creature_frame_index`).
Frames 0 and 1 are container fillers.

Identification evidence (content-based, no ordering assumed):

- For every one of the 145 base IDs with standard TwCrPort art, frame `id+2`
  carries the same creature: 94 frames are pixel-identical to the base
  portrait (RGBA CRC-32 equal after palette conversion), 36 more match within
  a small per-channel distance, and the remaining 11 (IDs 0, 1, 44–47, 88,
  89, 134, 138, 144) share the full opaque silhouette with HotA's recolored
  "portrait fix" artwork.
- Slots whose base ID has no portrait (war machines 145–150, reserved 122,
  124, 126, 128, 152) hold named filler frames (`twcrport_000` / 124–130
  `twcrport_001`), corroborating the offset: filler at frame `f` ⟺ ID `f−2`
  has no artwork.
- `CPRSMALL_…` (32×32, 202 frames, opaque squares) is the small-portrait
  sibling container; its frame order was **not** verified against creature
  IDs and it is not used.

Hard rules (same as base): selection is by index **and** verified embedded
frame name (`twcrportA{id+2:03}` must match exactly, else no image - this
rejects fillers and any future reordering); decoded frames must be exactly
58×64; missing/filler IDs render a placeholder, never an unrelated frame.

## F. Hero portraits

Rewritten by H3F-021, 2026-11-10; replaces the H3F-005 reading in which the
digits of `HPL###` were taken to be the portrait ID. Status by part:
VERIFIED_IN_H3FORGE for the name table (F.1), the `H3bitmap.lod` lookups,
the `P32F` layout (F.3) and the `HotA.lod` entries of IDs 163–186;
**INFERRED (high confidence, not conclusively verified)** for the
`HotA.lod` entries of IDs 187–244 - see step 4 in F.2.

### F.1 Portrait ID → file name

The save record's `hero_portrait` byte (record offset 12) and the portrait
bytes of an `.h3m` are indexes into the game's portrait **name table**:

| IDs | Large file | Source of the table |
|---|---|---|
| 0–127 | `HPL{id:03}{class}.pcx`, class by `id / 8`: Kn Cl Rn Dr Al Wz Hr Dm Dk Nc Ov Wl Br Bm Bs Wh | game executable |
| 128–135 | `HPL000pl.pcx` … `HPL007pl.pcx` | game executable |
| 136–143 | `HPL000el.pcx` … `HPL007el.pcx` | game executable |
| 144–162 | `HPL130Kn`, `000Sh`, `128Qc`, `003Sh`, `004Sh`, `005Sh`, `006Sh`, `007Sh`, `009Sh`, `008Sh`, `001Sh`, `131Dm`, `129Mk`, `002Sh`, `132Wl`, `133Nc`, `134Nc`, `135Wi`, `136Wi` | game executable |
| 163–181 | `HPLP01 02 03 05 06 13 15 16 04 07 08 09 10 11 12 14`, `HCVC01`, `HCVC02`, `HPLP17` | `HotA.dat`, record `newport0` |
| 182–186 | `HPL003KX`, `HPL122WX`, `HPL071DX`, `HPL116BX`, `HPL020RX` | `HotA.dat`, record `newport0` |
| 187–244 | `HPL179.pcx` … `HPL236.pcx` | `HotA.dat`, record `newport0` |

Small portraits replace `HPL` with `HPS` for IDs 0–162.

Evidence for the table: the portrait name strings in the installed
`h3hota.exe`, read in reverse storage order, give exactly this sequence for
0–162 (the first 128 follow the regular pattern, which fixes the direction).
`HotA.dat` is a `HDAT` container of named records; `newport0` holds 82
CRLF-separated large names followed by 82 small names, and its comment field
reads "first portrait: № 163". Cross-checks on the sample save: Roland (hero
type 152) carries portrait 152 = `HPL009Sh`, Xeron (155) = `HPL131Dm`, and
Cove hero 156 carries portrait 163, the first `newport0` entry, which
`HotA.dat`'s own `hero156` record also names (`HPLP01.pcx`).

Consequences of the old reading, now fixed: lookups stopped at 136; several
names share the digits `000`–`009` (`Kn`, `el`, `pl`, `Sh`), so the previous
directory scan could return the wrong hero for those IDs; and IDs 128–136
were mapped to campaign art instead of the Conflux heroes.

### F.2 Where the files are

- IDs 0–162: `H3bitmap.lod`, by exact name (case-insensitive). All 163 large
  and 163 small entries are present locally. Heroes PCX, 58×64 (large).
- IDs 163–244: `HotA.lod`. All 82 are present locally: 28 as Heroes PCX and
  54 as `P32F` (F.3). 58×64, except `HCVC02` at 58×65.

`HotA.lod` has no entry names (section B), so each HotA name is pinned to
its 16-byte entry ID in `hero_portraits.rs`. The pairing was established as
follows.

1. **The directory is in file-name order.** 1,887 entries carry an embedded
   frame name (DEF and D32F containers); their three-letter prefixes ascend
   through the directory with 27 exceptions in 1,886 adjacent pairs. Entry
   data offsets are strictly increasing in directory order.
2. **Named anchors surround every HotA name.** `HotA.lod` also ships
   replacement art for many base portraits. Matching each 58×64 image in the
   portrait run against the named `H3bitmap.lod` portraits (mean absolute
   RGB difference) labels those entries, and they appear in name order:
   `HPL000el`, `HPL000pl`, `HPL001el`, … `HPL127Wh`. The five `X` names fall
   at the only unmatched positions between their neighbours (`HPL003KX`
   between `HPL003Kn` and `HPL004el`, `HPL020RX` between `020Rn` and
   `021Rn`, `HPL071DX` after `071Dk`, `HPL116BX` between `115Bs` and
   `119Bs`, `HPL122WX` between `119Bs` and `124Wh`).
3. **`HPLP01`–`HPLP17` and `HCVC01`/`02`** are the only places where a 58×64
   PCX is immediately followed by a 48×32 PCX (`name.pcx` sorts before
   `names.pcx`): exactly 17 such pairs in the `HPL` run and two in the `HC`
   position, the counts the name table requires.
4. **`HPL179`–`HPL236`** follow `HPL127Wh` as a run of 59 images, then three
   placeholder images that have no small counterpart. The small-portrait
   block has the same 59-entry run. One entry too many for 58 names: the
   first one's nearest named portrait, in both the large and the small
   block, is Sir Mullich's (`HPL130Kn` / `HPS130Kn`), which sorts exactly
   there, so it is taken as HotA's redrawn `HPL130Kn` and the 58 names are
   assigned to the remaining 58 entries. Large and small runs agree
   image-for-image at this alignment (mean block-colour distance 21 at
   offset 0 against 37–38 at ±1 and ±2).

Step 4 is the one inference in the chain. It is supported by the points
above and by the heroes' known genders at several positions, but it is not
a byte-level proof. Failure mode if it is wrong: the extra entry is the
last of the run rather than the first, and all 58 portraits of IDs 187–244
are shifted by one. A single in-game comparison of one hero in that range,
or a decoded name hash, would settle it. Everything else is exact. At run time only the pinned entry IDs are used - never a directory
position - and a missing ID resolves to a placeholder.

The HotA.lod name → ID function is unknown. Tried without a hit: MD5, SHA
and BLAKE2 family digests and 128-bit FNV of the name in several case,
extension and encoding variants, and Mersenne Twister streams seeded from
the archive key, directory index or simple name hashes. `HotA.dll` contains
FNV-32 and MT19937 constants but no MD5/SHA tables.

### F.3 The `P32F` image

| Offset | Size | Field | Value in portraits |
|---|---|---|---|
| 0 | 4 | magic | `P32F` |
| 4 | 4 | u32 | 0 |
| 8 | 4 | u32 | 32 |
| 12 | 4 | u32 total size | 14,888 |
| 16 | 4 | u32 pixel offset | 40 |
| 20 | 4 | u32 pixel bytes | `width × height × 4` |
| 24 | 4 | u32 width | 58 |
| 28 | 4 | u32 height | 64 |
| 32 | 8 | - | 0 |
| 40 | … | pixels | BGRA, rows bottom-up |

`HotA.lod` holds 269 `P32F` entries (168 at 58×64, 92 at 48×32). Decoder:
`hero_portraits.rs::decode_p32`, bounded by the image pixel limit.

### F.4 Sample outcome

`SaAndVla.GM2`: 215 of 215 records resolve (131 with IDs up to 136, 19 with
137–162, 23 with 163–186, and 42 with 187–244 under the inferred
assignment). The source map sets a portrait for 67 hero types
and the save carries the same ID for each. Neither the save nor the map
embeds image data. Pinned by `tests/hero_portraits.rs` (all 245 table
entries decode and are pairwise distinct) and the desktop
`tests/map_link_flow.rs`.

Not used: HotA's redrawn base portraits and the HotA small-portrait entries
(the large image serves both sizes).

## G. Spell and artifact graphics

Spell icons - **VERIFIED_IN_H3FORGE** (2026-11-09, H3F-006):

- `H3sprite.lod/spells.def` is the spellbook icon set: 70 frames on a 78×65
  canvas, one per game spell, **array index = game spell ID** (0 Summon Boat
  .. 69 Summon Air Elemental; IDs per h3sed `metadata.IDS`, identical to the
  h3mtxt `SpellType` enum, already verified against the save flag arrays).
  Palette index 0 is transparent on every frame (~2,500 transparent canvas
  pixels each).
- The mapping is positional, not name-based: VCMI renders the spellbook via
  `CAnimImage("Spells")` + `setFrame(spell->id.getNum())`
  (`client/windows/CSpellWindow.cpp`), and
  `Mods/vcmi/Content/Sprites/spells.json` documents the one known art bug in
  the original data - the View Earth (ID 3) and View Air (ID 5) *pictures*
  were drawn swapped. H3Forge reproduces the original data as-is; the swap is
  a documented quirk, not corrected silently.
- Frame-name labels follow the same ID order with rename trivia: `Sp03Iden`
  is Visions' icon under its development name "Identify" (eye artwork),
  `Sp37Back` = Magic Mirror, `Sp52Wind`/`Sp53Muck` = Haste/Slow. Slots whose
  labels drift from their position (e.g. Destroy Undead art at index 18) are
  author mislabels; positions govern.
- `SpellInt.def`, `SpellScr.def`, `SpellBon.def` mirror the same order with a
  leading null frame (`Si00Null`, SpellInt only) and trailing creature-ability
  frames (IDs 70–77: Stone, Poison, Bind, Disease, Paralyze, Aging, Death
  Cloud, Thunderbolt - names match the h3mtxt commented enum exactly);
  SpellInt's 48×36 canvases are the in-battle cast icons. Only `spells.def`
  is used for UI icons.

H3Forge decodes `spells.def` through the interface-DEF reader (`decode_index`
for positional access), asserts the 78×65 canvas, caches per-ID PNGs, and
returns a placeholder for IDs outside 0–69 - never a neighbouring frame.

### G.1 Town icons (VERIFIED_IN_H3FORGE, 2026-11-10, H3F-022)

Two families: `ITPT` (58x64, town and hero screens) and `ITPA` (48x32,
adventure-map town list). Every town type has a fort picture and a village
(no fort) picture, each followed by a "built this turn" variant with a red
cross that H3Forge never selects.

| Source | Container | Frames | Fort picture | Village picture |
|---|---|---:|---|---|
| `H3sprite.lod` | `ITPT.def` | 36 | name `itpt<stem>` (`itptcast`, `itptramp`, `itpttowr`, `itptinfr`, `itptnecr`, `itptdung`, `itptstrn`, `itptfort`, `itptelem`) | name `itpt<3-letter stem>3` |
| `H3sprite.lod` | `ITPA.def` | 39 | name `itp<stem>1` (`cast`, `ramp`, `towr`, `infr`, `necr`, `dung`, `strg`, `fort`, `elem`) | name `itp<stem>3` |
| `HotA.lod` | D32F `f6f650f6c7c757b801daf48c5d7f2010` (frames `ITPT.defA000`..) | 48 | index `2 * type` | index `24 + 2 * type` |
| `HotA.lod` | D32F `f4fd5aaccf7268ccfd64d4d39649e87c` (frames `ITPA.defA000`..) | 51 | index `2 + 2 * type` | index `26 + 2 * type` |

Town types 0-8 are Castle, Rampart, Tower, Inferno, Necropolis, Dungeon,
Stronghold, Fortress, Conflux; the HotA containers add Cove (9), Factory (10)
and Bulwark (11). Evidence: all frames of both HotA containers were decoded
and inspected; the nine base subjects appear in base order, followed by the
three HotA towns, and the base DEF frame names carry the faction stems.
`ITPA` frames 0, 1 and 50 are blank, unknown and random-town fillers.

`town_icons.rs` selects base frames by embedded name and HotA frames by
index plus the expected embedded name, asserts the canvas size, and caches
PNGs per (type, fort, size). The fort variant follows the save's Fort bit
(SAVEGAME_FORMAT.md D.1). Types above the covered range return no image.

### G.2 Town building pictures (VERIFIED_IN_H3FORGE, 2026-11-10, H3F-022)

The Town Hall screen uses one container per faction with a 150x70 frame per
building. **Frame index = building ID** of the savegame building masks
(SAVEGAME_FORMAT.md D.1): 0-4 Mage Guild, 5 Tavern, 6 Shipyard, 7-9 Fort to
Castle, 10-13 halls, 14 Marketplace, 15 Resource Silo, 16 Blacksmith, 17-26
faction structures and grail, 30-36 dwellings, 37-43 upgraded dwellings.

| Town type | Archive | Container | Kind | Frames |
|---|---|---|---|---:|
| 0 Castle .. 8 Conflux | `H3sprite.lod` | `HALLCSTL`, `HALLRAMP`, `HallTowr`, `HALLINFR`, `HALLNECR`, `HallDung`, `HALLSTRN`, `hallfort`, `HallElem` `.def` | DEF | 44 |
| 9 Cove | `HotA.lod` | `5769101d44726072eec526bc8cce5102` (frames `HALLCOVEA000`..) | DEF | 44 |
| 10 Factory | `HotA.lod` | `f76f7faf3112ab704b85de3719fead74` (frames `hallfactA000`..) | DEF | 45 |
| 11 Bulwark | `HotA.lod` | `6c77e5296feb30309e7d810483d3a9db` (frames `hallblwkA000`..) | D32F | 44 |

Evidence: base frame names describe the building at each index (Castle:
`ThCsMag1`-`4` at 0-3, `ThCsTav1` at 5, `ThCsCas1`-`3` at 7-9, `ThCsHal1`-`4`
at 10-13, `ThCsPik1` at 30, `ThCsPik2` at 37). The HotA containers were
decoded and inspected frame by frame and show the same layout with the
faction structures at the slots the game's name tables assign.

Slots a faction does not use hold a filler: a copy of frame 0 in the DEF
containers (base DEFs reuse the frame name, HotA DEFs repeat the pixels) or a
blank frame in the Bulwark D32F. `building_icons.rs` treats a uniform frame,
or any frame after 0 that equals frame 0, as "no picture". 444 of 528 slots
resolve; the unused slots per faction are pinned in
`tests/building_icons.rs`. `HotA.lod` also carries redrawn copies of several
base containers (`HALLCSTL`, `HALLINFR`, `hallnecr`, `HALLRAMP`, `HALLTOWR`,
and a Conflux set named `HotA_ext`); H3Forge reads the base archive for
types 0-8.

Artifact icons - PARTIAL: `Artifact.def` (44×44 canvas) and `CPRSMALL.def`
(32×32) are verified containers only; no evidence-backed artifact-ID→frame
mapping exists yet (H3F-011/012 territory). External candidates (H3save_reader's
`gm1_mapping.json`) were consulted and do not contain a validated mapping.


## H. Runtime resource loading (H3Forge design)

Format rules vs design decisions: sections A–G are file-format facts; this
section is how H3Forge uses them.

- **Discovery** (`discovery.rs`): candidates in order - the executable's
  directory (portable layout), `H3FORGE_DEV_DATA_ROOT` (debug builds only),
  a previously stored selection (re-validated). Each candidate is tried as
  `<dir>` then `<dir>/Data`. No recursive drive search; nothing under an
  installation is ever written.
- **Archive selection**: the fixed probe list `KNOWN_ARCHIVES` = H3bitmap.lod,
  H3sprite.lod, h3ab_bmp.lod, h3ab_spr.lod, HotA.lod, HotA_lng.lod,
  HotA_ext.lod, HotA_l_ext.lod; only files actually present are reported,
  each validated through the bounded reader (metadata only). Resource sets
  are derived from readable archives (e.g. both base LODs readable ⇒ "Heroes
  III: Complete (SoD) artwork"); one bad archive never blocks the others.
- **Lookup**: by exact directory name (case-insensitive, duplicates rejected);
  only the requested entry's bytes are read and decoded.
- **Decoding/transfer**: PCX/DEF → RGBA in Rust → hand-rolled PNG
  (`png.rs`: 8-bit RGBA, filter 0, zlib level 6, CRC-32 chunks) → base64 DTO
  to the frontend.
- **Caching**: creature portraits cached per decoded PNG, bounded to the 141
  verified IDs; hero-portrait cache capped at 512 entries. Cache invalidation
  is by `SourceIdentity { path, len, modified_ms }` - a changed or replaced
  archive rebuilds the whole state.
- **Missing assets**: structured placeholder (with reason) - never a wrong
  image, never an index fallback.
- **Copyright**: game graphics are decoded from the user's own installation
  at runtime; nothing is bundled, committed, or extracted to disk.

---

## Appendix: sources and provenance

| Source | Location | Revision | Used for | License |
|---|---|---|---|---|
| mmarch (Rust) | local `D:/Projects/mmarch/mmarch-rust/`; upstream `https://github.com/might-and-magic/mmarch` | commit `fc104f2a7d8b170ed67bc05b9bfac5d0bb9a60b8` (Cargo package 7.0.1) | LOD header/directory layout and `zlib_decompress_bounded` semantics (`src/lod.rs`), Heroes PCX decoding (`src/image.rs::decode_pcx`) - adapted into `h3forge-resources/lod.rs` + `pcx.rs` | MIT (copyright Tom Chen; `licenses/mmarch-MIT.txt`) |
| VCMI | `https://github.com/vcmi/vcmi`, `client/render/CDefFile.cpp` (develop) | branch head 2026-11-09 | DEF cross-check: type enum 0x40–0x49, header/group/frame layout, compression formats 1–3 semantics, SGTWMTA/B special case. Format facts only - no code copied (GPL-2.0+). | GPL-2.0+ |
| thelazy.net wiki | `https://heroes.thelazy.net/index.php/Creature_portraits` | accessed 2026-11-08 | creature portrait ID↔frame-name cross-check (display reference) | wiki content |
| h3sed | local `D:/Projects/h3sed_4.1/` | supplied 4.1 tree | creature ID tables behind portrait eligibility (189 IDs for 44/11); spell name table | MIT (Erki Suurjaak) |
| H3save_reader | `https://github.com/AlexZ98765/H3save_reader` (`02_format_docs/gm1_mapping.json`, master) | branch head 2026-11-08 | external spell/artifact icon mapping candidate - **not locally validated** | not inspected |
| H3M2JSON corpus | `https://github.com/HeroWO-js/h3m2json` (+ `h3m-The-Corpus.txt`) | listed 2026-11-08 | naming-convention reference only | - |

Listed but not individually inspected (standard LOD/PCX descriptions agree
with the mmarch-derived implementation above; nothing here depends on them):
`https://github.com/neunato/homm3-unpacker`,
`https://github.com/Laserlicht/homm3-explorer`,
`https://github.com/josch/lodextract`,
`https://github.com/vcmi/vcmi/blob/develop/docs/modders/File_Formats.md`,
`https://pypi.org/project/lodfile/`.

Internal (authoritative):

| Module | Content |
|---|---|
| `crates/h3forge-resources/src/lod.rs` | bounded LOD reader, entry semantics |
| `crates/h3forge-resources/src/pcx.rs` | Heroes PCX decoder, RGBA conversion |
| `crates/h3forge-resources/src/def.rs` | interface-DEF reader, RLE 0/1 |
| `crates/h3forge-resources/src/creature_portraits.rs` | ID→frame-name mapping, portrait cache |
| `crates/h3forge-resources/src/catalog.rs` | HPL/HPS/PS portrait indexing |
| `crates/h3forge-resources/src/discovery.rs` | installation discovery, archive probes |
| `crates/h3forge-resources/src/hota32.rs` | HotA D32F container parser (section B.2) |
| `crates/h3forge-resources/src/skill_icons.rs` | secsk32.def + HotA D32F skill-icon mapping |
| `crates/h3forge-resources/src/spell_icons.rs` | spells.def positional spell-icon mapping |
| `crates/h3forge-resources/src/png.rs` | PNG encoding |
| `apps/h3forge-desktop/src-tauri/src/session.rs` | runtime cache/invalidation, asset pipeline |
| `docs/RESEARCH.md` | dated verification evidence |


### Secondary-skill icons (Skills UI follow-up, 2026-11-09)

Verified `H3sprite.lod/secsk32.def` through the existing bounded DEF decoder:
28 base skill IDs (0–27), 32×32 pixels. Basic mastery artwork is at zero-based
frame `3 + 3 * binary_skill_id`; three initial blank frames precede the
basic/advanced/expert triplets. The same Basic artwork is used in the current
skill rows and Add picker; mastery remains a separate editing control.
Mapping corroborated by [VCMI CSkill::registerIcons](https://github.com/vcmi/vcmi/blob/develop/lib/CSkillHandler.cpp),
which registers SECSK32 at `2 + mastery + 3 * id` (mastery 1–3).
Only this format fact is reused, not VCMI implementation code.

All 28 mapped frames decoded to nonempty 32×32 images and PNGs against the
local read-only archive. IDs 28/29 (HotA Interference/Runes) were resolved
from HotA.lod later (section B.3); other unknown IDs return placeholders. Missing archives,
unsupported decoding and frontend image failures also render placeholders.
Rust caches decoded PNGs by skill ID within the archive identity; frontend
shares requests and invalidates skill icons when the installation changes.
No images are bundled or extracted into source assets.


### HotA skill icon investigation (2026-11-09)

Read-only directory inspection covered workspace Data and the installed
G:/Games/Heroes3_HotA Data plus HD compatibility LODs; no loose SECSK
resources were found. In both installations H3sprite.lod contains identical
SECSK32 (32x32), SECSKILL (44x44), and SECSK82 (82x93) resources, each with
87 frames: three blanks plus 28 base-skill mastery triplets. SSkilBon has
84 frames (28 triplets without the leading blanks). The trailing filenames
ending in `28a/b/c` in SECSK resources are one-based labels for First Aid
(binary ID 27), **not** Interference. Base mapping agrees with
[VCMI CSkill::registerIcons](https://github.com/vcmi/vcmi/blob/develop/lib/CSkillHandler.cpp).

Resource SHA-256 prefixes match between both locations: SECSK32
`e56aeeaa81e36d08`, SECSKILL `298f31e75e045fcb`, SECSK82
`a04c3bc1871fca84`, SSkilBon `091bae5d9efafb61`.
H3bitmap (4912 entries) and HotA_lng (225 entries) have no matching skill
resources; HotA_ext/HotA_l_ext have zero entries. Base h3ab archives are
empty; HD's #sod_ru h3ab_bmp has 13 entries with no matching skill artwork.

Superseded by section B: the opaque-ID directory is now validated and
readable (stored/zlib), and the six Interference/Runes icons are identified
and integrated (section B.3). The paragraph below records the first-pass
state only.

HotA.lod in both locations used to fail the existing directory grammar at
record 0 (header `LOD\0`, version 200, count 5232). Its contents could not
then be reliably indexed by current support, and no verified Interference
(28) or Runes (29) entry or Basic/Advanced/Expert mapping was established.


### Mastery-aware skill icons (2026-11-09)

Supersedes the Basic-only display choice above. Skill IDs 0–27 and mastery
1/2/3 select SECSK32 frame `2 + mastery + 3 * id`, corroborated by the
previously inspected VCMI CSkill::registerIcons mapping. Local verification
now decodes all 84 variants, checks actual frame names, compares cached PNGs
to direct frame decoding, and confirms distinct adjacent mastery images.
Early frame filename numbers are historical artwork labels, not binary IDs:
IDs 0–13 use labels 12,1,7,13,3,10,6,14,9,8,2,4,11,5; IDs 14–27 use 15–28.
Suffix a/b/c identifies Basic/Advanced/Expert. Never derive skill IDs from
these filename numbers. Mastery outside 1–3 remains unresolved; IDs 28/29
were resolved later from HotA.lod (section B.3, 2026-11-09 second pass).

Rust and frontend caches now key by (skill ID, mastery); the typed command
carries both. Existing rows and replacement choices use current pending
mastery; Add choices use the selected addition mastery. Icon state tracks
both keys and ignores obsolete asynchronous responses. No save-byte or edit
operation behavior changed.


### H3F-014: local HotA opaque-ID directory verified (2026-11-09)

Supersedes prior unsupported-directory findings for the supplied archive.
Version word 200, XOR key u32 LE at 0x0C, directory at 80, count from offset 8.
Each 32-byte record: opaque ID[16]; offset/unpacked-size/packed-size u32 LE
at +16/+20/+24, each XOR key; plain compression byte +28; three unknown bytes
retained/ignored. Stored method 0 uses packed size 0; method 3 uses zlib.
Range, entry-count, allocation and output-size checks precede use. A valid
standard first record at 92 wins over the HotA detector; historical standard
reserved values 0 and 0x007E0213 do not trigger it. All directory records must
validate. Zero-length entries cannot point beyond EOF.

Both local files: key B5A4D744, 5232 records, 3869 zlib and 1363 stored.
No method-2 entries: reference raw LZMA1 parameters remain external-only;
method 2 and other methods fail with UnsupportedCompression on read. No
runtime Python, new compression dependency, extraction or modification.
One verified interface DEF token 7aa5a6b612d85711b32e3e287b850e66 decodes via
existing Rust DEF and PNG code. See H3F-014 for provenance, licensing,
metadata probe limits and tests. The six Interference/Runes resource
identities that were open at this note's first pass are resolved in
section B.3.

### Artifact artwork and visual editor follow-up (2026-11-10)

Verified local resource mapping: **163 real 44x44 icons**, covering IDs 0–143
and 147–165 (including scroll 1 and special-slot artwork). Of these, 141 come
from `H3sprite.lod/Artifact.def`; 22 come from the HotA D32F described below.
All 158 named catalog entries plus the scroll resolve; four additional icons
are war machines. Artwork is loaded from the selected installation, never bundled.

- The base DEF contains 146 frames. `HrArt###` names use a legacy numbering,
  **not** save artifact IDs: names 000–001 and 127–140 equal the ID, names
  002–121 are ID − 5 (IDs 7–126, e.g. Centaur's Axe 7 is `HrArt002.pcx`), and
  names 122–126 are IDs 2–6 (The Grail 2 is `HrArt122.pcx`). Verified by the
  nearest-pixel probe against the HotA by-ID `artifactA###` frames for every
  name 000–140. Resolve frames by name through this table, never by DEF order.
- Base `HrArt141/142/143.pcx` are visibly labeled unavailable-artwork tiles
  (Diplomat's Suit, Pendant of Reflection, Ironfist of the Ogre); exclude them.
  `HrArtSel` and `HrArtNOT` are overlays, not artifact icons.
- HotA opaque entry `4ae18dc00814380facbef89026300713` is a 167-frame, 44x44
  D32F. Verified artwork names: `artifactA141`–`143` and `artifactA147`–`165`.
  These were decoded and visually crossed to the existing ID/name catalog
  (cloak, pendant, fist; trident, naval shield, armor, crown, boots, runes,
  horseshoe, puppet, mask, rings, pendant, cape, goose, horn, charm, seal,
  plate and Sleepkeeper). Every returned HotA PNG is CRC-pinned in fixture tests.
- Do **not** use `artifactA{id}` for arbitrary IDs: pixel comparison with all
  base frames shows `HrArt122` maps to `artifactA002`, `HrArt007` to `artifactA012`,
  and IDs 2–121 follow the displaced frame order. Frames 144 and 166 are
  selection borders; 145 is a lock; 146 is not an enabled artifact mapping.
- Missing entries/names or unsupported IDs resolve to placeholders. Missing
  individual base frames do not disable otherwise available artwork; duplicate
  names are rejected. Existing bounded LOD/DEF/D32F/RGBA/PNG code is reused.

Reproduction: `cargo run -p h3forge-resources --example probe_artifacts` reads
local archives, prints nearest pixel comparisons, and writes inspection PNGs
only under ignored `.local/artifact-probe/`. This is research tooling, not an
application runtime dependency. `tests/artifact_icons.rs` verifies dimensions,
name mapping, pinned pixels and unavailable-tile/unknown-ID fallback.

## H3F-018 resource icons (2026-11-10)

Verified local `H3sprite.lod/RESOURCE.def` has seven 32x32 frames named
`rsrc0.pcx` through `rsrc6.pcx`. Visual inspection and the h3svg `ResourceType`
order match Wood, Mercury, Ore, Sulfur, Crystal, Gems, Gold respectively.
`ResourceIcons` requires exactly those seven names in order, validates each
canvas, decodes with the existing bounded DEF reader and caches PNGs.
ID 7 and larger, unavailable archives and unexpected layouts yield no icon.
The installation identity invalidates the runtime cache. No original game
artwork is copied into source or release assets. The local probe and focused
icon regression verify all seven PNGs and cache/unknown-ID behavior.

## H3F-023 - Original player banners

Local `H3sprite.lod/CREST58.def` contains nine 58x64 frames. `PlayerFlags`
requires the exact name sequence `crest0L.pcx`, `crest1G.pcx`, `crest2R.pcx`,
`crest3D.pcx`, `crest4B.pcx`, `crest5P.pcx`, `crest6W.pcx`, `crest7K.pcx`,
`crest8N.pcx`. Visual inspection verifies frames 0-7 as Red, Blue, Tan,
Green, Orange, Purple, Teal, Pink. Legacy filename letters do not describe
the final palette. Frame 8 is neutral and is not a player slot.

Existing bounded DEF decoding and PNG encoding/cache are reused; cache
identity follows the installation archive. The list scales the original
banner to 24x27, details use 58x64. Missing/unsupported assets show a
placeholder. No copyrighted artwork is added to source or release assets.
The existing town and portrait loaders serve the identified starting
factions/heroes; there are no new town/portrait mappings.

`itgflags.def` was also inspected: eight 15x20 flags, with legacy names
`iTG0LtBl.pcx` through `iTG7Blak.pcx`. It was not selected because the
current generic decoder retains a cyan background in that asset. No global
transparency-rule change or guessed palette rewrite was introduced.


## H3F-028 - Original Keymaster Tent icons

Verified in installed `H3sprite.lod`: `AVXkey00.def` through `AVXkey70.def`.
Class-10 templates in both the current GM2 and the H3M reader identify subtype
0..7 as Light Blue, Green, Red, Dark Blue, Brown, Purple, White, Black. Each
DEF has type 0x43, one group/frame named `AVXkey{c}0.pcx`, canvas 96x64,
96x55 crop at (0,9), and compression 3. All eight were decoded and visually
checked. No opaque HotA.lod name guessing or replacement artwork is involved.

The bounded Rust DEF reader additionally accepts map type 0x43 and format 3:
width must be divisible by 32; each row has width/32 little-endian u16 block
offsets relative to the frame body. A tag's top 3 bits select palette index
0..6 or literal code 7; low 5 bits encode run length minus one. Each independently
addressed block produces exactly 32 pixels. Tables, offsets, literal lengths,
run extents, canvas/crop and resource limits are checked. Format 2 and other
DEF types remain unsupported. Format facts were cross-checked with
[VCMI CDefFile](https://github.com/vcmi/vcmi/blob/develop/client/render/CDefFile.cpp);
no GPL implementation code was copied.

`KeymasterIcons` verifies names/count/dimensions and caches PNGs by color.
For compact icons only, special map shadow/selection indices 0..7 are transparent;
indices 8..255 and the original canvas are preserved. This is not a full map
shadow renderer. Existing interface palette conversion is unchanged. The desktop
cache uses archive source identity; frontend requests only a typed color ID.
No original DEF/PNG files are bundled; absent archives yield placeholders.
