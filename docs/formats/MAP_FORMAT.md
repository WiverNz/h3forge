# Heroes III / HotA map format (`.h3m`) as read by H3Forge

Scope: the fields `crates/h3forge-map` reads or must step over. This is not a full specification. Sources: the public format descriptions in [h3mtxt](https://github.com/alexanderbelous/h3mtxt) and [h3m2json](https://github.com/HeroWO-js/h3m2json), cross-checked against VCMI's `lib/mapping/MapFormatH3M.cpp` (used as documentation of the HotA 1.7/1.8 additions; no code copied). Status words: VERIFIED = checked on the local sample map; WALKED = local maps of that revision are consumed to the expected end.

All integers are little-endian. Text is a `u32` length followed by that many bytes in a legacy single-byte codepage. Booleans are one byte.

## Container

A single gzip member around the payload. H3Forge inflates with a 64 MiB output bound (256 KiB for header-only reads) and does not require a valid trailer. Maps written by the stock editors end with 124 padding bytes after the last section.

## Format word and revision flags

| First `u32` | Format | Flags used below |
|---|---|---|
| `0x0E` | Restoration of Erathia | - |
| `0x15` | Armageddon's Blade | AB |
| `0x1C` | Shadow of Death | AB, SoD |
| `0x20` | Horn of the Abyss | AB, SoD, HotA; then `u32` subrevision `v` |
| `0x33` | In the Wake of Gods | header only |

`HotA>n` below means subrevision greater than `n`. The sample map is subrevision 10.

HotA preamble after the subrevision: `HotA>7` three `u32` release numbers (sample 1, 8, 1 - VERIFIED, and equal to the three `u32` at payload offset 16 of the save); `HotA>0` 2 flag bytes; `HotA>1` `u32` terrain count; `HotA>4` `u32` town count + 1 byte; `HotA>6` 1 byte; `HotA>7` 1 byte; `HotA>8` `u32` count of shared variables, each a text + 5 bytes.

## Header (VERIFIED)

| Field | Type |
|---|---|
| has players | bool |
| edge length | `u32` (sample 252) |
| has underground | bool |
| name, description | text, text |
| difficulty | `u8` 0–4 |
| level limit | `u8`, AB+ |
| 8 player slots | below |
| victory, loss condition | below |
| team count, then 8 team bytes if non-zero | `u8` |
| allowed heroes | HotA: `u32` count + ⌈count/8⌉ bytes; AB/SoD 20 bytes; RoE 16 bytes |
| placeholder heroes | AB+: `u32` count + 1 byte each |
| customized heroes | SoD+: `u8` count, each: hero type `u8`, portrait `u8` (`0xFF` = default), name text, player mask `u8` |

Player slot: human bool, computer bool, AI tactic `u8`, SoD+ 1 byte, allowed factions (2 bytes AB+, 1 RoE), random-faction bool, main-town bool → [AB+: generate-hero bool, town type `u8`] position (x, y, z bytes); random-hero bool; main hero type `u8` (`0xFF` none) → portrait `u8` + name text; AB+: 1 byte, `u32` count of (hero type `u8`, name text).

Victory: `u8` (`0xFF` standard); otherwise 2 flag bytes and by kind: 0 → artifact ID; 1 → creature ID + `u32`; 2 → 5 bytes; 3 → 5 bytes; 4–7 → position; 8, 9, 11 → nothing; 10 → 4 bytes; 12 → `u32`. Loss: `u8` (`0xFF` standard); 0, 1 → position; 2 → `u16`. Artifact and creature IDs are `u16` in AB+, `u8` in RoE.

## Between header and objects (WALKED)

| Section | Layout |
|---|---|
| reserved | 31 bytes |
| HotA options | 4 bytes; `HotA>0` `u32` count + ⌈count/8⌉ bytes; `HotA>2` `u32`; `HotA>4` 8 bytes |
| HotA scripts | `HotA>8` bool; when set, a script block follows that H3Forge does not parse (body unsupported) |
| allowed artifacts | HotA: `u32` count + ⌈count/8⌉ bytes; SoD 18 bytes; AB 17 bytes; RoE none |
| allowed spells, skills | SoD+: 9 + 4 bytes |
| rumors | `u32` count of (text, text) |
| predefined heroes | SoD+: count is 156, or a `u32` in HotA; per hero a bool, and when set: [bool → `u32` experience] [bool → `u32` n + 2n bytes skills] artifacts, [bool → biography text] gender `u8` [bool → 9 bytes spells] [bool → 4 bytes primary skills]; then `HotA>4` 6 bytes per hero |
| terrain | 7 bytes per tile, `size² × levels` |
| object templates | `u32` count; each: sprite name text, 16 bytes of masks, `u32` class, `u32` subclass, 18 bytes |

Hero artifacts: bool; when set, 19 slots (18 before SoD) then `u16` backpack count, each entry an artifact ID plus, `HotA>4`, a 2-byte scroll spell.

## Object instances

`u32` count; each instance is position (x, y, z bytes - the bottom-right tile of the sprite), `u32` template index, 5 padding bytes, then class-specific data selected by the template's class. Only two kinds are modelled.

### Hero - classes 34 hero, 62 prison, 70 random hero (VERIFIED)

| Field | Type |
|---|---|
| quest identifier | `u32`, AB+ |
| owner | `u8` (`0xFF` none) |
| hero type | `u8` (`0xFF` for a random hero) |
| name | bool → text |
| experience | SoD+: bool → `u32`; before: `u32` |
| portrait | bool → `u8` |
| secondary skills | bool → `u32` n + 2n bytes |
| garrison | bool → 7 × (creature ID, `u16`) |
| formation | `u8` |
| artifacts | as above |
| patrol radius | `u8` |
| biography, gender | AB+: bool → text; `u8` |
| spells | SoD+: bool → 9 bytes; AB only: 1 byte |
| primary skills | SoD+: bool → 4 bytes |
| padding | 16 bytes |
| level settings | `HotA>4`: 6 bytes |

The hero stands one tile left of the stored position. Sample: hero type 28 is a prison at stored (185, 123, 0) with portrait 213 and name "Ben Kenoby".

### Town - classes 98 town (subclass = faction), 77 random town (VERIFIED)

| Field | Type |
|---|---|
| identifier | `u32`, AB+ |
| owner | `u8` |
| name | bool → text |
| garrison | bool → 7 stacks |
| formation | `u8` |
| buildings | bool → 6 bytes built + 6 bytes forbidden; otherwise has-fort bool |
| spells | AB+: 9 bytes obligatory; 9 bytes possible |
| spell research | `HotA>0`: bool |
| special buildings | `HotA>4`: `u32` n + n bytes |
| events | `u32` count; each: common event block, `HotA>4` 14 bytes, `HotA>6` 1 byte, 6 bytes buildings, 14 bytes creatures, 4 bytes |
| alignment | SoD+: `u8` |
| padding | 3 bytes |

Common event block: name text, message text, 28 bytes resources, player mask, SoD+ human bool, computer bool, 20 bytes schedule, `HotA>6` `u32`, `HotA>8` bool → 5-byte script reference.

### Other classes

Stepped over using the widths in `parse.rs::read_object` (events, Pandora's boxes, monsters, signs, seer huts and quests, witch huts, scholars, garrisons, artifacts, scrolls, resources, mines, dwellings, shrines, grail, quest guards, placeholders, banks, HotA reward objects, black markets, universities). Classes not listed there carry no instance data.

## After the objects

Global events: `u32` count of common event blocks (plus 14 bytes each for HotA subrevisions 5 and 6). Then the 124 padding bytes.

## Bounds

File ≤ 32 MiB, payload ≤ 64 MiB, any text ≤ 512 KiB, templates and objects ≤ 500,000 each, edge length 1–255; every read is range-checked and failures are typed (`MapError`), never panics.

## H3F-023 - Authored player details and puzzle objects

Player eligibility remains `can_human_play` / `can_computer_play`, distinct
from a save's current controller. Team count is now validated as 0-8;
playable slots must reference a team below the declared count. Unused slots
with out-of-range sentinel team IDs carry no team. Zero teams means no
team alliances, not an inferred grouping by color. Presentation uses
one-based team numbers while the parser retains zero-based IDs.

The sample's authored teams (zero-based) are `[0,1,2,2,3,1,3,4]`.
Red and Pink are computer-only; the other six permit Human or Computer.
Each main-town coordinate matches exactly one same-starting-owner town
entrance (`anchor.x - 2`). Red's faction is Inferno (3), Pink's is Cove (9);
the other six are random and are not replaced with current owned factions.
Named main heroes are Red's Night King (type 150, map portrait 161) and
Pink's John Silver (type 165, default portrait). For the latter, the UI
explicitly labels the unique corresponding save hero's current portrait.
The other six starting heroes are random/generated and stay unidentified.

Object class **57** has no extra instance payload: retain the H3M object
ordinal and stored x/y/z sprite anchor, validating it against map dimensions
and levels. These are not GM2 visit IDs or inferred actionable tiles.
Sample: **29 obelisks**, 12 surface and 17 underground. The first is object
4423 at (170,48,0), the last object 37316 at (197,163,1).

For class **36**, subclass below 1000, retain the existing four-byte unsigned
Grail radius and object anchor. Subclass 1000+ HotA markers are not Grails.
Sample: one authored placement at (75,15,0), radius 0. The UI exposes the
placement count, not a claim that the Grail is still buried/available.
Absence of an authored placement does not rule out a generated Grail.
All original sequential parsing and the sample's 124 trailing bytes remain
unchanged; there is no H3M writer.
# H3F-027 initial map armies

`MapBody.neutral_armies` retains authored monster and garrison armies while
walking the existing H3M object stream. Class 54 resolves its creature from
the template subtype; classes 71-75 and 162-164 remain random, with no
invented resolved ID. Monster quantity is the u16 following the optional
AB+ identifier. Class 33/219 garrisons contain seven interleaved creature-ID
and u16-count pairs after the owner word. IDs are u8 in RoE and u16 in AB+;
all-ones means empty. Existing version-specific tails are still consumed.

The linked Mysterious Island map has 834 such objects: 661 fixed monsters,
126 random monsters and 47 garrisons. Parsing still lands at the expected
124 trailing bytes. The desktop shows these armies only as initial H3M
reference facts, matched by unique anchor and object category, never as
current quantities, existence proof or GM2 patch destinations. Random or
zero initial quantities retain their original values. The current GM2
comparison and blockers are documented in
[H3F-027](../issues/027-neutral-creatures.md).


## H3F-028 - Keymaster Tent reference objects

The bounded object walker retains class-10 anchors, subtype 0..7 and raw template
sprite names in `MapBody.keymaster_tents`. Positions must fit the map dimensions
and level count; unknown subtypes refuse the body. No payload follows this object
class. The private reference maps subtypes to `AVXkey{c}0.def` for all eight colors.
This authored H3M information never supplies current visit state or GM2 offsets.
The supplied Pink/Red save pair's tent positions differ from the existing local
H3M reference; matching save evidence governs the feature.
