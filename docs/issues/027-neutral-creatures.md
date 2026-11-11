# H3F-027 - Neutral Creatures and Map Garrisons

- Milestone: M4; priority: P1
- Dependencies: H3F-006, 008, 009, 013, 021
- Status: **Implemented for verified HotA 44/11 wandering groups and class-33 garrisons. Other guarded object types remain unsupported. Native edited-save loading and desktop visual smoke remain manual checks.**

## Implemented

- Compact Creatures section: original icons, names, current X/Y/Z, map
  level, search by creature/ID/coordinates, object-type and level filters.
- Current tile references checked against current object indexes,
  templates and action masks; no H3M offsets used for GM2 writes.
- SaAndVla.GM2: **291 wandering groups and 47 map garrisons**. The latter
  contain 28 occupied and 19 empty armies; all 47 are editable.
- Seven-slot garrison editing: type/count changes, add, remove, replace,
  including clearing the entire army. Positive counts up to 2,147,483,647;
  empty slots use FFFFFFFF/zero. Ownership, coordinates, removal permission,
  template data and unknown bytes remain unchanged.
- Reuses creature catalog, portraits, searchable picker, ArmyEditor,
  typed commands, BytePatch, Save/Save As, Revert army and Discard changes.
- Initial H3M armies shown separately as reference facts. English, Russian,
  German and Spanish labels; no redesign of the existing editor.

## Evidence and limits

The linked H3M contains 834 authored armies: 661 fixed monsters, 126 random
monsters and 47 garrisons. Of 291 current wandering groups, 280 match fixed
placements and 11 match originally random placements. All 47 garrison
anchors match, but only 29 retain the exact initial stacks. Initial map
data is therefore not a substitute for current save state.

The native survivor pair establishes current quantities in a count-prefixed
33-byte monster table. Object 13926, creature ID 60 at (62,53,0), references
record 517: its current count changes from the user-confirmed 72 to 56.
All 291 original sample groups now expose verified-layout current counts.
Wandering quantity edits require a positive signed-32-bit count. Type edits
repoint the current object to an existing, unambiguous template with identical
dimensions, masks and all fixed fields except creature subtype, and update
the matching tile subtype. Shared templates and the remaining 29 monster
record bytes are preserved. The picker lists only compatible replacements.
No wandering stack creation, removal, splitting or resurrection is supported.

All sample garrisons use current class 33 with subtypes 0/1. Class 219 is
read-only under the same structural checks because it has no current-save fixture.
Border/key/quest gates are not assumed to carry armies. Heroes on objects
may hide underlying tiles; no army is invented, and incomplete garrison
index coverage closes garrison writes. Ambiguous/truncated structures are
refused. Template discovery is bounded to the first 16 MiB.

Removed/defeated monster state cannot be inferred from one save. Only current actionable
objects are listed. Missing authored objects are not labeled defeated;
empty garrisons are not labeled defeated. Banks, mines, dwellings, Pandora
boxes, guarded resources/artifacts and events remain unsupported because
their current army links are not verified.

## Compact UI

- Separate Wandering creatures and Garrisons/Gates buttons with category counts.
- Dense 34px list rows, 16px icons, names/current quantities and X/Y/Z;
  compact search and level filters, selected-category filtered counts.
- Compact seven-slot editor with searchable selectors and quantity inputs:
  two columns above 620px editor width, one below. Repeated captions and
  large portraits removed only in Creatures. Advanced / Diagnostics holds
  technical reasons and initial H3M reference facts.
- Pending edits, per-army Revert, Discard, Save and Save As retained.
  Actual native-window fit at 1280x720 remains a manual visual check.

## Native evidence and remaining checks

The supplied before/after defeat pair changes current wandering groups from
1,186 to 1,185 and removes object 13926. The 24-byte tile-render contraction
does not authorize deletion or zeroing of its persistent monster record.
The supplied `SaAndVla_after_56.GM2` fulfills the surviving-stack experiment:
record 517 changes from 72 to 56 after accounting for a 303-byte replay shift.
Only the count byte changes in the aligned tail. No further save pair is
needed for these implemented quantity operations.

Replacement types are structurally verified and roundtrip tested, but loading
edited copies in HotA remains a manual check. A native garrison transfer pair
would provide supplementary game-state validation; it is not supplied.
Class 219, other guarded objects, hidden/ambiguous records and nonmatching
monster-table variants remain unsupported or read-only. Supporting those
requires a separate specific native experiment, not guessed offsets.

## Verification

- `sample_wandering`: native 72/56 identity/count evidence, type and count
  changes to two independent groups, exact whole-payload diff restricted to
  planned 2-byte tile, 2-byte object-template and 4-byte count fields,
  compressed save/reopen, invalid removal/slot/ID refusal; all 291 original
  sample wandering groups have editable current quantities.
- `sample_neutral_defeat`: identity/removal, aligned early-table equality,
  two surviving identities, refusal of removal or recreation of defeated group.
- `sample_neutral`: 47 garrisons, multi-slot replacement/removal/addition,
  exact diff, stale preimage refusal, save/reopen and malformed-link refusal.
  Linked H3M cross-check passed during the initial implementation.
- `neutral_flow`: pending wandering type/count, Revert and direct Save/reopen;
  existing garrison Discard, multi-slot Save As, Save, backups and preservation.
- Frontend typecheck and production build pass. Two focused rendering tests
  pass, including seven compact selectors/inputs and pending markers.
- Save-library Clippy with warnings denied and Git whitespace checks pass.
- Original saves/assets and external repositories were never modified.
  No full workspace suite, commits or releases.

## Research sources

- [H3save_reader tile scanner](https://github.com/AlexZ98765/H3save_reader/blob/master/01_tools/tile_scanner.py),
  [merge_monsters](https://github.com/AlexZ98765/H3save_reader/blob/master/01_tools/merged_objects.py)
  and [GM1 compendium](https://github.com/AlexZ98765/H3save_reader/blob/master/02_format_docs/GM1_format_compendium.md):
  useful read-side leads, not proof of 44/11 write offsets.
- [h3mtxt H3M specification](https://github.com/alexanderbelous/h3mtxt/blob/master/doc/h3m_specification.MD)
  and [h3svg save specification](https://github.com/alexanderbelous/h3mtxt/blob/h3svg/doc/h3svg_specification.MD):
  independently checked tile/template/object and garrison structures.
- [HeroWO corpus](https://github.com/HeroWO-js/h3m2json/blob/master/h3m-The-Corpus.txt)
  and [h3_map_editor handlers](https://github.com/Shakajiub/h3_map_editor/blob/main/src/handler_08_objects.py):
  initial monster/garrison structures and gate distinctions only.
- [VCMI object format](https://github.com/vcmi/vcmi/blob/develop/docs/modders/Map_Object_Format.md):
  conceptual distinction between garrisons, gates and banks, not original save serialization.

Verified details: [SAVEGAME_FORMAT.md](../formats/SAVEGAME_FORMAT.md#h3f-027-current-map-armies)
and [MAP_FORMAT.md](../formats/MAP_FORMAT.md#h3f-027-initial-map-armies).
