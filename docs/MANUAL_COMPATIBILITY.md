# Manual HotA compatibility testing

This checklist separates actual game results from automated parsing, byte-diff
and save/reopen tests. Implementation details and exact write gates remain in
the [issue index](issues/README.md) and [save specification](formats/SAVEGAME_FORMAT.md).
Release environment checks remain in [RELEASE.md](RELEASE.md#manual-release-checklist).

## Recorded user report

Recorded on 2026-11-14 from the user's direct report of manual testing in
Heroes of Might and Magic III: Horn of the Abyss. All checked operations below
worked correctly in the actual game, not merely in H3Forge's parser.
The execution date, exact HotA version, H3Forge build, test save filename and
hash, edited values, and Save versus Save As path were not supplied. Do not
infer them from earlier sample tests or the locally installed game version.
The report establishes the listed operations, not every variant or boundary
case, every supported format, or clean-machine release readiness.

## Implemented and verified in-game

| Result | Operation | Related issue | Scope of reported evidence |
|---|---|---|---|
| [x] Pass | Hero army composition | H3F-009 | Modified composition worked in HotA; no claim for extreme quantities or town/map garrisons |
| [x] Pass | Visited Keymaster's Tent states | H3F-028 | Modified visited states worked; player/color coverage and guard/gate interactions not individually specified |
| [x] Pass | Add artifacts to heroes | H3F-011 / H3F-012 | Artifact addition worked; artifact IDs and backpack/equipment destination not specified |
| [x] Pass | Replace an existing secondary skill | H3F-010 | One existing skill replaced successfully |
| [x] Pass | Add a secondary skill | H3F-010 | Another skill added successfully |
| [x] Pass | Current spell points (mana) | H3F-007 / H3F-008 | Current mana modified successfully |
| [x] Pass | Known hero spells | H3F-007 / H3F-008 | Known spells modified successfully; individual spells and equipment interactions not specified |
| [x] Pass | Attack | H3F-007 / H3F-008 | Attribute modified successfully |
| [x] Pass | Defense | H3F-007 / H3F-008 | Attribute modified successfully |
| [x] Pass | Spell Power | H3F-007 / H3F-008 | Attribute modified successfully |
| [x] Pass | Knowledge | H3F-007 / H3F-008 | Attribute modified successfully |

## Implemented, automated verification only for these operations

Each unchecked row means no recorded manual pass for that operation. It does
not negate the narrower passes above. See each issue for automated coverage.

| Pending | Operation | Related issue / remaining evidence |
|---|---|---|
| [ ] | Hero level and experience | H3F-007/008; not included in the user report |
| [ ] | Secondary skill removal, reordering and mastery changes | H3F-010; replacement/addition do not establish these cases |
| [ ] | Backpack removal/replacement, scroll editing and capacity cases | H3F-011; addition destination and artifact IDs remain unspecified |
| [ ] | Equipment swaps/unequips, combination artifact equip/remove, coupled attribute bonuses and granted spells | H3F-012; generic artifact addition does not establish these cases |
| [ ] | Town garrisons, including the garrisoned-hero UI path | H3F-013/022; hero army pass does not establish town editing |
| [ ] | Supported town buildings and Mage Guild spells | H3F-022; partial implementation, no manual pass |
| [ ] | Player resources | H3F-018; automated round trips only |
| [ ] | Wandering map creatures and map garrisons | H3F-027; native source pairs are format evidence, not edited-output validation |
| [ ] | Hero relocation | H3F-019; implemented constrained same-level/medium moves, no manual pass |
| [ ] | Hero dismissal, turn transition and tavern recruitment | H3F-026; before/after source pairs do not establish edited-output gameplay |
| [ ] | Keymaster per-player/color coverage and Border Guard/Gate interactions | H3F-028; visited-state pass does not claim all 64 combinations or access interactions |
| [ ] | Save/Save As separately, backup recovery, failures and source conflicts | H3F-008; automated coverage exists, manual persistence paths not identified |

Native UI/DPI/keyboard/relaunch checks, human translation review, clean Windows
10/11, installation placement and missing-WebView2 checks remain pending where
listed in H3F-016/017/024/025/029. None follows from save-edit compatibility.

## Not implemented or deliberately unsupported

- Additional save layouts without operation-specific evidence (H3F-015).
- Map revelation actions (H3F-023).
- Town relocation, cross-level/medium hero moves and object interactions (H3F-019).
- Unsupported town structures, including Fort, Citadel, Castle, Capitol and grail
  edits (H3F-022); supported building edits above remain separately unverified in-game.
- Automatic combination artifact assembly/disassembly and equipped-scroll coupling
  (H3F-012). Combination artifact equip/replace/remove is implemented but retains
  automated-only coverage. Unsupported operations are not pending test passes.

## Record future tests individually

Use a copied save and retain the original and backup. Record one operation per
row, the expected result, and what was actually observed after loading in HotA.
Test coupled gameplay behavior separately when relevant. Never fill missing
metadata by guessing. Keep private saves and screenshots out of tracked files.

| Recorded date / tester | Execution date | HotA version | H3Forge build | Private save reference / hash | Operation and before/after values | Save or Save As | Observed in-game result | Pass / Fail / Not tested | Evidence / limits |
|---|---|---|---|---|---|---|---|---|---|
| Pending | Not recorded | Not recorded | Not recorded | Not recorded | One operation | Not recorded | Not tested | Not tested | Record only observed behavior |

For each new result:

- [ ] Record available metadata and explicitly mark omissions.
- [ ] Load the modified copy in HotA and inspect the specific change.
- [ ] Record relevant gameplay follow-up separately, including failures.
- [ ] Update this checklist and the affected issue without expanding the pass
  to untested variants, formats or neighboring features.
