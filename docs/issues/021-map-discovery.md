# H3F-021 - Map discovery and H3M integration

Status: **completed 2026-11-10** (sample-verified; cleanup pass same day). Open confidence items: the file-name assignment of portrait IDs 187–244 is inferred, not proven (section 5), and the in-application visual check by the maintainer is pending. Milestone: M4. Priority: P1. Independent of H3F-012/013.

## Goal

Find the original `.h3m` behind an opened savegame, read the parts of it H3Forge can use, and show that information next to - never instead of - the save's current state. Use the map's portrait IDs to settle the 84 hero portraits that were placeholders.

The savegame stays authoritative for everything that changes during play. The map describes the scenario before turn one.

## What was built

| Part | Where |
|---|---|
| Bounded read-only H3M reader, discovery, matching | new crate `crates/h3forge-map` (no Tauri, no dependency on save parsing) |
| Save header fields used as match keys (size, underground flag, difficulty) | `crates/h3forge-save/src/parse.rs` (`MapMetadata`) |
| Portrait name tables, `P32F` decoder | `crates/h3forge-resources/src/hero_portraits.rs` |
| Link state, hero enrichment, DTOs | `apps/h3forge-desktop/src-tauri/src/map_link.rs`, `session.rs` |
| Commands `get_map_link`, `rescan_maps`, `select_map_candidate`, `choose_map_file` | `commands.rs` |
| Map row in the save Details section, "Map start" line on the hero header | `web/src/components/MapLinkRow.tsx`, `SaveBanner.tsx`, `HeroDetail.tsx` |

Format details: [MAP_FORMAT.md](../formats/MAP_FORMAT.md), portrait tables in [GAME_RESOURCES_FORMAT.md](../formats/GAME_RESOURCES_FORMAT.md) section F, save header fields in [SAVEGAME_FORMAT.md](../formats/SAVEGAME_FORMAT.md) B.1.

## 1. Supported H3M fields and versions

Header (always read; this is all discovery needs): format and HotA subrevision, HotA release that wrote the map, edge length, underground flag, name, description, difficulty, level limit, the eight player slots (human/computer, main town position, customized main hero with portrait and name, starting hero names, team) and the customized ("disposed") hero list with portrait ID, name and player mask.

Body (walked sequentially; the format has no section table or object lengths): predefined-hero flags, object templates, every object instance. Kept: hero objects (hero, prison, random hero - position, hero type, owner, quest identifier, custom name, custom portrait, experience) and town objects (position, faction, owner, identifier, name, garrison flag, built/forbidden building masks, event count), plus per-class object counts. Every other object is stepped over with bounds checks and not modelled.

| Format | Header | Body | Local evidence (615 maps in `Maps/`) |
|---|---|---|---|
| HotA `0x20`, subrevision 10 (1.8.x) | yes | yes | **sample map verified**; 69 of 72 walk to the end, 3 use scripted events |
| HotA subrevisions 0, 1, 3, 5, 6, 9 | yes | yes | 12 of 13 walk to the end, 1 uses scripted events |
| RoE `0x0E` | yes | yes | 417 of 419; 2 files fail with an invalid template index/count |
| AB `0x15`, SoD `0x1C` | yes | yes | 55 of 55, 56 of 56 |
| WoG `0x33` | yes | no | none locally; body reported as unsupported |
| HotA subrevision above 10 | yes | no | reported as unsupported |

"Walks to the end" means the reader stops exactly at the 124 padding bytes every stock-editor map ends with, which only happens if each of the tens of thousands of objects was stepped over with the right width. A body failure never hides the header: the map still links, and the UI says placement data is unavailable and why.

Not supported: maps that use the HotA 1.8 scripted-event system (4 local maps) - the script block is not parsed, so their body is unavailable. No writer exists.

## 2. How map matching works

1. **Where**: `Maps` beside the save's folder (the game and this workspace both keep `games/` next to `Maps/`), `Maps` inside the save's folder, and `Maps` of the selected installation. Sub-folders two levels deep are listed (the sample map sits in `Maps/New/`). At most 5,000 files; nothing outside these folders is touched.
2. **What is read**: only the first 256 KiB of each file, inflated to at most 256 KiB, parsed to the end of the header. Headers are cached in memory by path + size + modification time for the life of the application; a changed file is re-read.
3. **Match key** - what a save retains from its map: internal name bytes, description bytes, edge length, underground flag, difficulty. A candidate must equal the name and must not contradict any other field. The file name is never used (the sample file is `[HotA] The Mysterious Island - v1-5-j - ALLIES.h3m`; its internal name is `- The Mysterious Island - COOP`).
4. **Outcome**: one survivor is linked; several are listed for the user to pick; none leaves the save unlinked with a "Choose map…" action. A manually chosen file whose header contradicts the save is refused with an explanation and the previous state is kept.

Limit of this key: two different maps with byte-identical name, description, size, levels and difficulty cannot be told apart by header. They are reported as ambiguous rather than guessed. Saves do not store a map hash.

## 3. What the map adds

- Map file, format, HotA release, size and levels, player slot counts, town count, placed and random hero counts.
- Per hero: where that hero type starts the scenario (hero or prison, tile, level, starting owner), the name and portrait the map assigns, and whether the save's portrait ID agrees.
- Parsed but not yet shown: town positions, factions, owners, identifiers, names and building masks (for H3F-013 and H3F-019), player main-town positions, object class counts.

Positions are reported as the hero's tile (`stored x − 1`, the stock hero template offset) and the town gate (`stored x − 2`). These offsets are the format convention and have not been compared with save coordinates yet; H3F-019 owns that check.

## 4. Which heroes can be matched

The shared identity is the **hero type ID**: byte 10 of the save record and the type byte of the map's hero objects and header entries. Names and record order are never used.

Sample result: the map places 64 hero objects - 58 with a hero type (13 heroes, 45 prisons), each type used once, and each has exactly one save record of that type. The 6 random heroes have no type in the map (it is rolled at game start), so they cannot be tied to a save record and are only counted. The remaining 157 save records are hero types the map does not place; they are reported as "not placed on the map". A type placed more than once would be reported as ambiguous with no position.

## 5. Portraits: 84 of 84 resolve; 42 of them by an inferred mapping

All 215 records of `SaAndVla.GM2` now show a portrait. None came from the map file and none was missing from this machine. "Resolve" means an image is found and decoded; how sure the identity of that image is differs by ID range (table below).

The earlier conclusion ("IDs above 136 are custom portraits shipped with the map and absent here") was wrong on two counts:

- A portrait ID is an index into the game's portrait name table, not the number in a file name. The previous lookup read `HPL###` as the ID, so it stopped at 136 and - because `HPL000Kn`, `HPL000el`, `HPL000pl` and `HPL000Sh` all carry `000` - could also pick the wrong file for IDs 0–9.
- IDs 0–162 are the Shadow of Death table, all present in `H3bitmap.lod`. IDs 163–244 are HotA additions, all present in `HotA.lod`, which had only been searched for one container type.

| Portrait IDs | Sample records | Source | Identity confidence |
|---|---|---|---|
| 137–162 | 19 | `H3bitmap.lod`, by table name | exact |
| 163–186 | 23 | `HotA.lod`, by entry ID | exact (structural position, bounded by named neighbours) |
| 187–244 | 42 | `HotA.lod`, by entry ID | **inferred, high confidence - not conclusively verified** |

What the map contributed is confirmation: it sets a portrait for 67 hero types, and the save carries the same ID for all 67.

The pairing of HotA file names to `HotA.lod` entries is evidence-based, not read from the archive (HotA 1.8 hides entry names).

**The inferred range.** `HPL179`–`HPL236` (IDs 187–244, 58 names) sit in a run of 59 consecutive portrait images. The assignment assumes the first image of the run is HotA's redrawn Sir Mullich (`HPL130Kn`) and gives the 58 names to the other 58. If that assumption were wrong, the extra image would be the last one instead and **every portrait in 187–244 would be off by one** - each hero would show its neighbour's portrait. Supporting evidence: the extra image's nearest named portrait is Sir Mullich's in both the large and the small block; the large and small runs agree image-for-image at this alignment; hero genders fit at the positions spot-checked. Not available: a decoded name hash, or a pixel-exact reference for any portrait in the range. Cleanup review (2026-11-10) found no cheap conclusive check, so the status stays "inferred". Conclusive options for later: one in-game comparison of any hero with type 179–214 (for example Henrietta, type 179, portrait 187), or decoding the archive's name → ID function. A shift, if found, is a one-line change to the table offset.

Full evidence chain: GAME_RESOURCES_FORMAT.md section F.

## 6. Resources still unavailable

- Portraits for IDs above 244: none exist in this HotA version; such an ID shows a placeholder stating it is outside the tables.
- HotA portraits on an installation without a HotA 1.8 `HotA.lod`, or with a HotA version whose archive lacks the pinned entry IDs: placeholder naming the missing file. Nothing is substituted.
- Small (`HPS`) variants of HotA portraits are not mapped; the large image is used for both sizes.
- HotA's redrawn versions of some base portraits are not used; the `H3bitmap.lod` originals are shown.
- H3M files contain portrait IDs only. A map that needed artwork outside the game's tables could not be helped by parsing it.

## 7. Verification

Run on 2026-11-10 against the real save, map and archives:

- `crates/h3forge-map/tests/synthetic.rs` (5 tests, no game data): header fields, hero/town extraction with exact consumption, typed errors for unknown format, truncation, oversized text, bounded decompression; discovery depth/count bounds; match, ambiguity, rejection, name-only.
- `crates/h3forge-map/tests/sample_map.rs` (`H3FORGE_MAPS_ROOT`): sample found among 615 files by internal name; HotA format 10, release 1.8.1, 252×252 with underground, 8 playable slots (6 human), 214 header hero entries, 37,696 objects, 37 towns, 64 hero objects, 124 trailing bytes.
- `crates/h3forge-resources/tests/hero_portraits.rs` (`H3FORGE_FIXTURE_ROOT` = `Data`): all 245 table entries resolve, decode to 58×64 (one 58×65), and are pairwise distinct. This proves presence and distinctness, not which hero each image in 187–244 shows.
- `apps/h3forge-desktop/src-tauri/tests/map_link_flow.rs` (`H3FORGE_FIXTURE_ROOT` = `games`): opening the save links the right map with full strength; hero 28 enriches to a prison at 184,123 surface with map portrait 213; 84 of 84 formerly missing portraits resolve and 215 of 215 overall; two copies of the map give "ambiguous" with nothing linked; an unrelated map is refused; choosing a candidate links it.
- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets` reports nothing in the new code; frontend `npm run build` passes.

Cleanup pass (2026-11-10): the stale creature test in `portrait_flow.rs` was corrected and renamed `creature_portrait_command_resolves_hota_and_keeps_placeholder_for_unknown_id` - creature 153 (Nymph) now must resolve from `HotA.lod`, and ID 200, beyond the container's 0–199 range, must stay a placeholder with a reason. Re-run once after the fix: `portrait_flow` 2/2, `map_link_flow` 2/2, resources `hero_portraits` 1/1, `h3forge-map` synthetic 5/5 and sample 1/1. No known failing test remains in the targets H3F-021 touches; the full workspace suite was not repeated.

Note on fixtures: the resources tests read `H3FORGE_FIXTURE_ROOT` as the `Data` folder, the save and desktop tests as the `games` folder, so one workspace-wide run cannot enable both.

## Limitations

- Not checked in the running application by eye; the flow is verified through the same session methods the commands call.
- Portrait identity for IDs 187–244 is inferred (section 5); a wrong assumption would shift that whole range by one.
- The header cache is in memory only; a first open scans the folders again after each start (about a second for 600 maps).
- Verified against one save/map pair. Other format revisions are exercised only by walking local maps to their end, not by comparing values with a save.
- Map hero positions are not yet compared with save coordinates (H3F-019).

## For later issues

`ParsedMap` already carries what H3F-013 (towns: faction, owner, identifier, buildings), H3F-019 (hero and town positions) and future building or mage-guild work need. Further object kinds can be modelled inside `read_object` without touching callers. No editing was added here.
