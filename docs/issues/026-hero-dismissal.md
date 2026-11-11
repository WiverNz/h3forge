# H3F-026 - Hero dismissal

Status: implemented and verified against two real before/after pairs (boat and land), 2026-11-11. In-game validation pending. Priority: P1. Dependencies: H3F-008 (safe save), H3F-018 (player table), H3F-019 (hero front matter).

The original request named `docs/issues/025-hero-dismissal.md`; number 025 was already taken by the themes issue, so this is H3F-026.

## Requested outcome

Dismiss a hero directly from a HotA 44/11 save, reproducing the game's native "Dismiss Hero" action as far as it can be verified, with confirmation in the hero editor and full integration with pending edits, Revert, Discard, Save and Save As.

## Evidence

Two clean pairs, each saved immediately before and after one native dismissal with no other action (private, read-only, in `games/`):

1. `SaAndVla_before.GM2` -> `SaAndVla_after.GM2`: Galthran (hero type 71), Pink, selected, aboard a boat, one stack.
2. `SaAndVla_before2.GM2` -> `SaAndVla_after2.GM2`: Moandor (hero type 66), Pink, on plain land, six stacks, Basic Intelligence, Basic Navigation.

Supporting, not clean: `SaAndVla.GM2.20261107` -> `SaAndVla.GM2` (weeks of play apart), where heroes 66 and 150 left their owners and show the same reset form.

Every changed byte of both clean pairs is listed in [SAVEGAME_FORMAT.md section I](../formats/SAVEGAME_FORMAT.md#i-hero-dismissal-h3f-026). References consulted: the h3svg specification (structure names and order) and h3sed's hero profile walk. VCMI was not used as a format source.

## What the game does and what H3Forge writes

| Structure | Native change | H3Forge |
|---|---|---|
| Hero front matter | `is_visible` 1 -> 0 | reproduced |
| Hero record | owner -> FF; temporary morale/luck -> 0; destination -> -1; visited bytes 109-111 -> 0 | reproduced |
| Hero army | one creature in slot 0, slots 1-6 blank | reproduced; kept creature = first occupied stack (see limitations) |
| Spell points | maximum: knowledge x 10, x 12 with Basic Intelligence | reproduced for no or Basic Intelligence |
| Movement total, left and land word | 2000 for heroes without bonuses (2200 with Equestrian's Gloves) | reproduced without Logistics and without worn movement artifacts |
| Sea movement word | 1500, or 2250 with Basic Navigation | reproduced for no or Basic Navigation without worn sea artifacts |
| Hero owner table | entry -> FF | reproduced |
| Player record | hero count - 1; selected hero -> FF if it was this hero; slots compacted | reproduced |
| Map tile under the hero | actionable flag cleared; object class 34 -> 0; properties -> hero's "object under" properties | reproduced, land and water |
| Boat record | `exists` 1 -> 0 for a boarded boat | reproduced |
| Replay event log | one event appended per dismissal, one more for a boat; count updated | out of scope: previous-turn playback data, resizes the payload |

Findings from the second pair that corrected the first implementation:

- The visited block is not cleared as a whole. Moandor keeps bytes 53, 81 and 85; only bytes 109-111 are cleared now.
- The Pink town byte and the older replay event byte that changed in the first pair did not change in the second. Both are incidental. The town byte is one bit (spell 9) of a per-spell Mage Guild mask and carries no hero reference. Neither is ever written.

## Implementation

- `crates/h3forge-save/src/dismiss.rs`: `SaveDocument::hero_dismissal_blocker(id)` and `plan_hero_dismissals(ids)`. All structures are re-derived from the immutable payload and cross-checked before any patch is planned. Output is ordinary checked same-length `BytePatch` values; the payload is never resized and no hero bytes are removed. Heroes are planned jointly because heroes of one player share roster bytes.
- Desktop: `set_hero_dismissal` command, a pending dismissal set in the session, an entry in the unsaved-changes summary, and `dismissalPending` / `dismissalBlockedReason` in the hero edit state. Revert (per hero), Discard all, Save and Save As handle dismissals like other pending edits.
- A dismissal resets army, spell points and movement, so it cannot be combined with pending army or spell point edits of the same hero; both orders are refused with an explanation. Other edits of that hero still combine.
- UI: "Dismiss hero" in the hero header opens an inline confirmation that explains the effect and the differences from the game. A pending dismissal shows a notice with "Keep hero". Unsupported heroes get a disabled action with the backend's explanation. Four languages.

## Supported case and refusals

Supported: exact HotA 44/11, a player-owned hero listed in the owner's adventuring roster, visible on the map, standing on a tile with no object underneath, on land or aboard a boat.

Refused with an explanation (no evidence, action disabled):

- hero garrisoned in or visiting a town;
- hero standing on a map object ("object under" fields set);
- hero on water without a boarded boat (Fly, Water Walk), or a boat record that does not match;
- last hero of a player without towns (would eliminate the player);
- unowned heroes, low-confidence records, saves where the roster, hero owner table, player table, boat table, map size or the hero's tile cannot be verified or is ambiguous;
- any other save layout.

## Remaining differences against the real pairs

Land pair (Moandor): H3Forge's output equals the game's after-save except for

| Offset | Size | Explanation |
|---|---:|---|
| 913 | 5 | embedded save file name |
| 4,922,520 | 1 | replay event count |
| 4,945,063 | +8 | one appended replay event |

Boat pair (Galthran): additionally

| Offset | Size | Explanation |
|---|---:|---|
| hero + 31, + 35, + 1076 | 2 each | land movement 2200: the hero wears Equestrian's Gloves, whose bonus is not verified, so the stored values are kept |
| 4,335,985 | 1 | town 13 spell mask, incidental |
| 4,924,201 | 1 | byte inside an older replay event, incidental |
| 4,945,037 | +26 | two appended replay events (hero and boat) |

## Limitations

- **Kept creature.** H3Forge keeps the first occupied stack's creature. In the older pair hero 150 went from creature 65 in slot 0 to creature 56, so the game probably uses the hero type's first starting creature. Both clean pairs already had the starting creature in slot 0 and cannot tell the rules apart. H3Forge has no per-hero starting army table.
- **Bonuses not verified, values left unchanged:** land movement with Logistics or worn Equestrian's Gloves, Boots of Speed or Wayfarer's Boots; sea movement with Advanced/Expert Navigation or worn Necklace of Ocean Guidance, Sea Captain's Hat or Admiral's Hat; spell points with Advanced/Expert Intelligence.
- **Replay log** is not extended.
- Garrisoned and visiting heroes and heroes on map objects have no evidence and stay refused.

## Verification

- `H3FORGE_FIXTURE_ROOT=<abs path to games> cargo test -p h3forge-save --test sample_dismissal`: 3 passed. Both pairs: every patched byte equals the byte the game wrote, and the remaining differences are exactly the tables above. Gates, a joint two-hero plan, save and reopen are covered.
- `H3FORGE_FIXTURE_ROOT=<abs path to games> cargo test -p h3forge-desktop --test dismissal_flow`: passed. Mark, unmark, per-hero discard, Discard all, refusal of conflicting spell point edits in both orders, Save As with a combined attribute edit, re-base, direct Save, source untouched.
- `npm.cmd --prefix web run check` and `test:i18n-ui` passed.
- The obsolete research probe `tests/probe_dismissal.rs` was removed.

## Open items

- **In-game validation is required**: load a dismissed-hero save in HotA, end the turn, and recruit the hero from a tavern. Not done.
- Optional evidence that would lift limitations: a pair for a hero whose first stack is not its starting creature; a pair for a hero with Logistics or Boots of Speed; a pair for Advanced or Expert Intelligence.
