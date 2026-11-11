# H3F-022 - Town buildings, Mage Guild and artwork

Status: partially completed (2026-11-10, second follow-up the same day). Town artwork, building pictures and names for all twelve factions, building-state parsing, Mage Guild parsing and Mage Guild spell editing are implemented and sample-verified. Building editing covers the verified subset below in all 37 sample towns, now including the Shipyard and most horde buildings. Fort, Citadel, Castle, Capitol, grail and structures with their own saved state stay read-only. Native-window and in-game loading are not confirmed. Milestone: M4. Priority: P1.

## Goal

Make the Towns section a practical editor: original town icons, the saved game's current building state, and the current Mage Guild spells, edited with the same pending-edit, Save and Save As workflow as heroes.

## Scope

- A. Original town icons for all twelve town types, in the list and the town header.
- B. Read the building masks of HotA 44/11 town records; construct and remove buildings where the coupled fields are verified.
- C. Read the Mage Guild level, per-level spell slots and counts; replace, add and remove guild spells.

Out of scope: ownership, coordinates, player resources, recruits, external dwellings, animated town screens, H3M edits.

## Relevant files or components

- `crates/h3forge-save/src/town_state.rs` (state parsing, model, validation), `edit.rs` (`TownEdit::SetBuildings`, `TownEdit::SetGuildSpells`, `plan_town_state`), `domain.rs` (`TownView::state`).
- `crates/h3forge-resources/src/town_icons.rs` (`TownIcons`, `HotATownIcons`), `building_icons.rs` (`BuildingIcons`, `HotABuildingIcons`).
- Desktop `session.rs` (`apply_town_request`, `town_icon`, building and guild projections), `dto.rs`, `commands.rs` (`get_town_icon`, `get_building_icon`).
- Web `TownIcon`, `TownBuildings`, `TownGuild`, `SpellCombobox`, `TownDetail` tabs.
- Formats: [SAVEGAME_FORMAT.md](../formats/SAVEGAME_FORMAT.md) section D.1, [GAME_RESOURCES_FORMAT.md](../formats/GAME_RESOURCES_FORMAT.md) section G.1.

## Dependencies

H3F-013 (town records), H3F-006 (spell icons), H3F-014 (HotA.lod containers), H3F-021 (map link, reference only). See the [issue index](README.md).

## Acceptance criteria

- [x] Letter placeholders are replaced by original town icons; all twelve town types resolve, fort and village variants, without bundled or substituted artwork.
- [x] Building state is read from the save (built, constructible, not available) and kept apart from H3M map settings.
- [x] A safe building change round-trips with a payload diff restricted to the intended fields.
- [x] Mage Guild level, per-level spells and counts are read from the save; map must/may masks are never shown as guild inventory.
- [x] A Mage Guild spell change round-trips with a payload diff restricted to the intended fields.
- [x] Revert, Discard, Save and Save As cover the new edits; a garrisoned hero blocks only the garrison.
- [x] Original building pictures in the Buildings tab for all twelve factions, with a placeholder only for slots a faction does not use.
- [x] Actual building names for every faction, including Cove, Factory and Bulwark structures and dwellings.
- [x] The Factory Mage Guild 4/5 anomaly is explained well enough to edit the rest of those towns; the guild chain itself stays locked there.
- [x] Shipyard and horde buildings are editable where the stored form matches what the sample shows.
- [ ] Fort, Citadel, Castle, Capitol, grail and structures with their own saved state are editable. Blocked: see Unresolved.
- [ ] An edited save loads in the matching game (manual maintainer gate).

## Outcome

### Verified facts

Town artwork:

- `HotA.lod` holds D32F replacements of `ITPT` (58x64, 48 frames) and `ITPA` (48x32, 51 frames). Decoding every frame shows fort pictures at `2 * type` (ITPA: plus 2), village pictures in a second block (ITPT 24, ITPA 26), with Cove, Factory and Bulwark as types 9-11. `H3sprite.lod` `ITPT.def`/`ITPA.def` cover types 0-8 and are selected by embedded frame name.
- All 37 sample towns resolve in both sizes; the twelve types of the sample are all present. HotA containers are preferred, the base archive serves types 0-8 without HotA.lod, other types show a placeholder.

Town record fields (offsets relative to the end of the town name, HotA 44/11, 37 of 37 sample towns):

- `+84` five bytes: spell count per guild level. `+89` u64 visible-graphic mask. `+97` u64 built mask. `+105` u64 constructible mask. `+113` thirty i32 spell slots, six per level. `+233` nine bytes banned-spell mask.
- Building bit numbers equal the h3svg `TownBuildingTypeH3SVG` enum (44 bits). The built mask carries every member of an upgrade chain; the visible mask keeps only the top graphic of each chain.
- The Mage Guild level byte equals the unbroken Mage Guild building chain in all 37 towns. Each guild level stores its spells first, then `-1`; capacity is 5, 4, 3, 2, 1, plus one with the Tower Library. The count byte equals the number of spells for built levels and zero for unbuilt ones. Every stored spell matches its level in the standard spell level table. 504 of 507 stored spells are clear in the banned mask; the three exceptions are presumably map-forced spells.
- The constructible mask encodes the faction guild limit: Stronghold and Fortress stop at level 3, Castle, Cove and Bulwark at 4.
- Sample totals: 855 built building bits and 507 guild spells parsed.

Editing:

- `SetGuildSpells { level, spells }` rewrites the level's slots up to its capacity and, for a built level, its count byte. Spells must belong to the level, be unique, fit the capacity, and not be banned unless already stored. Works for all 37 sample towns, including the two Factory towns below.
- `SetBuildings { built }` takes a full target mask. The session turns a toggle into a target by adding missing chain prerequisites or removing dependents. The planner then rewrites the built mask, the visible mask, the Mage Guild level byte and the guild counts together.
- Editable buildings: Mage Guild 1-5, Tavern, Town Hall, City Hall, Marketplace, Resource Silo, Blacksmith, Shipyard, horde buildings, dwellings 1-7 and their upgrades (see the second follow-up for the Shipyard, horde and fortification rules). Constructing requires the save's constructible bit. Chain rules: Mage Guild levels in order, hall levels in order, Marketplace before Resource Silo, base dwelling before its upgrade.
- Buildings are editable in all 37 sample towns. In the two Factory towns the Mage Guild chain is locked (see the follow-up below).
- Round trips on Port Balloon (private copies only): building Mage Guild 3 changes exactly four fields (level byte, one count byte, visible mask, built mask); removing it again plus the Blacksmith leaves exactly the two Blacksmith mask bytes different from the original; a spell replacement changes exactly one i32 slot; shrinking and restoring spell lists returns the byte-identical payload. Through the session: toggle-back clears the pending edit, Discard restores both edit kinds, Save As writes a file whose payload differs from the source in exactly five bytes, and the source file is untouched.

### Design choices

- Resources are never deducted and the in-game build tree across unrelated buildings (for example Blacksmith before a dwelling) is not enforced, like the original map editor.
- Banned spells cannot be added. This respects the saved game's own restriction; it is stricter than the game requires.
- Bits hidden by an unexplained faction-specific graphic replacement are preserved and locked. With a horde building present, dwellings can be added but not removed.

### Unresolved

- Fort, Citadel, Castle and Capitol: the save carries a separate adventure-map sprite template per faction for the village, fort, citadel, castle and capitol looks (`avc???x0`, `0`, `c0`, `f0`, `z0` `.def` names in the object template area). Which object record selects the template for a town is not located, so a change here could leave the map picture out of step. The Capitol is also limited to one per player. All four stay locked.
- Shipyard with a docked ship (bit 20) was never observed and stays locked. Factory uses the shipyard slot for the Mana Generator, which has its own state; locked.
- Special structures (bits 17, 21-23) and extra structures (27-29): several carry their own saved state (artifact merchant stock, Mystic Pond and Mana Vortex flags, Portal of Summoning creature, Library spells, Lookout Tower map reveal) or replace another graphic (Fountain of Fortune over Mystic Pond, Gunpowder Warehouse over the upgraded Frigate, Upg. Gantry over Gantry, Altar of the Runes over the Sieidi). None of it is located well enough to write; locked.
- Hordes still locked: Dungeon (Mushroom Rings; no Dungeon horde town in the sample), Cove's Roost (always appears with the Gunpowder Warehouse, so the dwelling it hides cannot be told apart), and any horde attached to a basic dwelling (that form was never observed).
- Grail: one per map, built from the grail artifact; only one grail town exists in the sample and the related map state is not located; locked.
- Whether the game recomputes the visible mask on load is unknown. The editor always writes it consistently with the shared chains.
- The 185 HotA-specific bytes after the h3svg fields; recruits and external dwelling counts (not adjusted when a dwelling is added or removed).
- Factory: whether Mage Guild 5 is always drawn beside level 4, and what bit 20 (never constructible, airship picture) means.
- Only one 44/11 sample backs the layout. Other versions expose no building or guild state.

### Follow-up: pictures, names, Factory (2026-11-10)

Verified facts:

- Building pictures. The Town Hall screen containers hold one 150x70 frame per building ID. `H3sprite.lod` has one DEF per original faction; `HotA.lod` has `HALLCOVE` and `hallfact` (classic DEF) and `hallblwk` (D32F). Base frame names describe the building at each index, which fixes frame index = building ID; the three HotA containers were decoded and inspected frame by frame. 444 of the 528 faction/building slots resolve to a picture. The other 84 are slots the faction does not use (filler frames: a copy of frame 0, or blank), listed per faction in `tests/building_icons.rs`; they show a placeholder, never another picture. Every building present in the sample towns has a picture except the three extra-structure bits 27-29, which have none in any faction.
- Names. Slot assignment comes from game data: `BldgSpec.txt` and `Dwelling.txt` for the nine original factions and the `casspcost10`-`12` / `casdwcost10`-`12` tables of `HotA.dat` for Cove, Factory and Bulwark (the local files are the Russian edition, so they fix which structure sits in which slot). English spellings follow the original game and the HotA wiki pages for Cove, Factory and Bulwark. All structures and dwellings of all twelve factions are named. Remaining generic labels are slots no faction table names: extra structures 27-29, unused special slots, and Factory bit 20.
- Factory specifics from that data: the shipyard slot (bit 6) is the Mana Generator, bits 22/23 are the Gantry and Upg. Gantry (Dreadnought dwelling), the horde (Pen) boosts level 3.
- Factory Mage Guild. Nostromo and Nautilus store guild level 5 with all five built bits and draw both level 4 and level 5. The same holds in all four local saves, for a player-owned and a neutral town, and no other faction shows it in any of them. This looks like regular Factory behavior (level 5 is drawn in addition to level 4), but only this one state was observed.

Design choices:

- A level drawn beside its upgrade no longer makes the whole town read-only. The drawn bits are preserved, and only that pair is locked. Mage Guild 5 is locked for every Factory town, so the unobserved transitions (building or removing level 5) are never written. Everything else in those towns follows the normal rules.
- A built building that a locked building depends on is shown as locked with a "required by" reason, instead of failing when clicked.
- The Buildings tab shows picture cards grouped by kind, with upgrade chains joined by arrows. Built, can-be-built and not-available cards differ by border, tint and label; a click applies at once as a pending edit; a locked card shows a one-line reason.

### Second follow-up: garrison ownership, Shipyard, hordes (2026-11-10)

Verified facts:

- Garrison ownership. All 17 sample towns with a garrisoned hero store seven empty garrison slots of their own; the hero's army lives in the hero record. The one town with only a visiting hero (OldtownLibrary) and the towns without heroes keep their own garrison words. So while a hero is garrisoned there is no separate town garrison to edit: the hero's army is the garrison. A visiting hero's army is separate from the town garrison.
- Shipyard. Record bytes 7 and 8 (h3svg `generated_boat_x/y`) hold a boat tile in exactly the towns whose Shipyard is constructible, plus one coastal town where the map prohibits it; all other towns hold `FF FF`. Six towns have the Shipyard built with bit 6 drawn and bit 20 clear. Factory's bit 6 is constructible without a boat tile, which matches it being the Mana Generator.
- Hordes. In all 25 built hordes of the verified faction/horde pairs, both bits (18+19 or 24+25) are set, the upgraded bit is drawn, and the upgraded dwelling of the boosted creature level is hidden: Castle 3, Rampart 2 and 5, Tower 2, Inferno 1 and 3, Necropolis 1, Stronghold 1, Fortress 1, Conflux 1, Cove Pub 2, Factory 3, Bulwark 6.
- Fortifications. The save contains separate town sprite templates for citadel and castle looks in addition to village, fort and capitol (see Unresolved).

Changes:

- Towns with a garrisoned hero show that hero's portrait, name and actual stacks in the Garrison tab. The stacks are edited in place through the hero's own army edit (by hero record ID), and "Edit hero army" opens that hero on the Army tab. The town's own garrison words are still never written while a hero is inside. Towns with only a visiting hero keep direct garrison editing.
- Shipyard is editable when the town has a boat tile, the save allows it, no ship is docked and the town is not a Factory. Only the built and visible masks change; the boat tile is never written.
- A horde building is edited as one unit of two bits. It can be built when the upgraded dwelling it boosts is built and both bits are constructible, and removed when stored in the observed form. Building hides the upgraded dwelling and removal shows it again.
- Citadel and Castle were editable before this follow-up and are now locked, for the same reason as the Fort. This is a deliberate step back based on the sprite templates found in the save.

Round trips (private copies): removing and rebuilding Unearthed Graves in Brill, and the Shipyard in Reptile Promnt, each change only the two mask fields and return the byte-identical original payload.

## Verification

- `crates/h3forge-save`: `town_state` unit tests (14), `tests/sample_town_state.rs` (5, `H3FORGE_FIXTURE_ROOT` = `games`).
- `crates/h3forge-resources`: `town_icons` unit tests (2), `tests/town_icons.rs` (2) and `tests/building_icons.rs` (1, `H3FORGE_FIXTURE_ROOT` = `Data`).
- Desktop: `tests/town_state_flow.rs` (4, including garrisoned, visiting and independent town armies), `tests/command_permissions.rs` (covers `get_town_icon` and `get_building_icon`).
- One workspace test run, clippy on touched code, frontend typecheck and build.
- Not verified: the native window, and loading an edited save in the game.
