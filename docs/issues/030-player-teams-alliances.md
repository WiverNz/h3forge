# H3F-030 - Player Teams & Alliances

Status: joining an existing team is implemented and verified in HotA (see "Team reassignment delivered" at the end). Creating a team, emptying a team and saves without teams remain unsupported. Earlier sections are kept as history; where they say team writes are blocked, the final section supersedes them.
Milestone: M4. Priority: P1. Dependencies: H3F-008, H3F-018, H3F-023.

## Goal and delivered boundary

Change current player teams in existing HotA 44/11 GM2 saves, including joining
an existing team and splitting into a separate team. Neither operation is
currently supported. Initialized team assignments are now identified in the
supplied map/save comparison; current alliance authority and complete
consistency rules remain unverified.

Players -> General now has a compact disabled current-save Team selector with
an accessible explanation in English, Russian, German and Spanish. Its value
is Unknown even when a map is linked. Existing map team/allies/enemies remain
explicitly authored H3M facts. No speculative preview or writable command was
added. Existing resource/tent pending edits, Revert, Discard, Save and Save As
remain the only supported player workflows.

## Evidence reviewed, 2026-11-14

- [h3svg specification](https://github.com/alexanderbelous/h3mtxt/blob/h3svg/doc/h3svg_specification.MD):
  the classic 42/2 SavedGame embeds TeamsInfo after victory/loss settings.
  It describes a one-byte team count and eight zero-based player team bytes
  present only when the count is nonzero. This identifies a candidate structure,
  not authoritative ongoing HotA 44/11 alliance state. The zero-count case
  also raises a variable-length transition incompatible with blind same-length
  patching. Player, TileVisibility and Obelisk describe separate state.
- [H3save_reader compendium](https://github.com/AlexZ98765/H3save_reader/blob/master/02_format_docs/GM1_format_compendium.md)
  targets 42/2 examples. Its
  [header parser](https://github.com/AlexZ98765/H3save_reader/blob/master/01_tools/header_parser.py)
  uses filename searches and heuristic boundaries. These inspected sources do
  not establish a 44/11 team reassignment operation; their absolute offsets
  are not adopted.
- [H3API H3MapInfo](https://github.com/RoseKavalier/H3API/blob/master/include/h3api/H3GameData/H3MapInfo.hpp)
  names hasTeams and playerTeam[8], plus victory/loss structures.
  [H3Main](https://github.com/RoseKavalier/H3API/blob/master/include/h3api/H3GameData/H3Main.hpp)
  separately names isDead[8].
  [H3Player](https://github.com/RoseKavalier/H3API/blob/master/include/h3api/H3Players/H3Player.hpp)
  includes visitedObelisks and daysLeft; H3PlayersInfo separates starting and
  participation information. These are process-memory structures, not GM2
  offsets or a HotA serializer contract. No upstream implementation was copied.
- H3Forge's players.rs verifies eight 149-byte records anchored before towns,
  ownership links and resource words at +98. It contains no team field.
  PlayerEdit supports resources only; tents have their separate verified path.
  Empty ownership does not distinguish defeated from unused.
- Earlier H3F-023 research matched authored team bytes at payload offset 527
  in its historical sample. That remains an unwalked candidate, never a write
  address. Equality with H3M or paired obelisk visit bits cannot prove authority.

## Consistency gates

| State or rule | Evidence and remaining requirement |
|---|---|
| Current team IDs/count and alliance lookup | Locate structurally in 44/11; establish which serialized fields HotA actually uses, including duplicate/cached relations |
| Join/split, empty teams, no-team saves | Verify valid IDs, team-count semantics, renumbering and same-length feasibility; no assumed allocation policy |
| Shared vision | Identify full visibility state and load-time sharing; establish whether past exploration persists after a split and how a merge combines it |
| Obelisks/puzzle and visits | Existing visit masks are readable; team counters, merge/split behavior and slot associations remain incomplete |
| Victory and defeat | Verify ordinary allied victory and special conditions, AI applicability, target ownership and terminal-state behavior |
| Player status | Verify active/defeated/unused participation independently; team changes must not revive eliminated players |
| Other interactions | Validate allied hero meetings, town access, combat prevention, resource transfer, AI targeting and turn processing; determine whether persisted caches exist |

This is a dependency audit, not a claim that every dependency has been found.
No map or save parser was changed. No guessed GM2 offsets are exposed.

## Controlled comparison requested

The user confirmed that their game cannot change alliances mid-game. An
impossible native mid-game before/after request is withdrawn.

Use two copies of the same map with fixed starting settings. Start game A
with Red and Blue on separate teams and save teams-A.GM2 immediately on day 1,
before moving or ending a turn. Start game B with only Blue moved to Red's
team, then save teams-B.GM2 at the same point. Supply both copied H3M files,
all player/controller settings and exact HotA/HD versions. Preserve originals.
Avoid random factions/heroes/bonuses where possible.

This pair can locate initial team structures and supporting initialization
state. It cannot alone prove safe ongoing reassignment. New-game RNG, generated
objects and timestamps can differ; never attribute every difference to teams.
After identifying fields, evidence from HotA load/use behavior and disposable
edited-save reloads is still required before opening the write gate. A reverse
split case and later-game scenario with differing exploration/puzzle progress
must be validated as well.

## Acceptance and verification

### Supplied three-save comparison (2026-11-14)

Inspected same_teams.png and separate_teams.png plus private copies of all
three supplied GM2 files. Screenshots show the same Crimson and Clover
(Allies) scenario and the same color alliances: Red/Blue versus Tan/Orange.
Player2 occupies Blue in the same-team setup and Tan in the separate-team
setup. The Blue/Tan starting factions and chosen hero move with Player2.
This changes which color the human controls, not the displayed color teams.

| Save | Compressed bytes | Payload bytes | SHA-256 of original |
|---|---:|---:|---|
| same_teams.GM2 | 80031 | 580040 | 8d232da61946a960b0813defd67c19979ac7d3604aafd4bb5bb3d6b1d0caf25c |
| same_teams_restart.GM2 | 80057 | 580039 | 852640d9ffe55960b0356ef3d31b344555de294e4e2e91373eb0bfc78fe2134a |
| separate_teams.GM2 | 80036 | 580035 | ab5e3c3142337e1d10e38975bae3f195d5475a7ed01ab5bd9083cafa9a1aa493 |

All three payloads declare 44/11 and reference Crimson and CloverA.h3m.
Bytes [0,495) are identical in all three. The candidate team count at 474
is 2; candidate assignments at [475,483) are [0,0,1,0,1,0,0,0], unchanged.
These sample-specific candidates agree with the existing Rust map parser's
read-only result for the local Maps/Crimson and CloverA.h3m: playable Red/Blue
team 0, Tan/Orange team 1. Local map identity is corroborated by name, dimensions
and starting positions, not a supplied map hash.

Starting-faction candidates at [500,532) are
[2,2,5,-1,5,-1,-1,-1] in both same-team saves and
[2,5,2,-1,5,-1,-1,-1] in separate_teams. Starting-hero candidates at [904,912)
are [35,34,93,255,91,255,255,255] versus
[35,81,34,255,94,255,255,255]. This agrees with the screenshot's
Tower/Josephine moving from Blue to Tan; random AI heroes also differ.
These are research observations, not newly supported controller/team fields.

Even the same-team restart changes payload length and header bytes at
495, 497-499, 533, 902 and the save filename. Aligned differing byte totals
are 221003 (same/restart), 240728 (same/separate), 234214 (restart/separate).
Variable-length data shifts later offsets, so these totals are not counts of
changed semantic fields. Matching bytes across the two same-team saves do
not eliminate the controller/faction confounder.

All three gzip streams reach a complete DEFLATE end with exactly an eight-byte
trailer and matching ISIZE, but stored CRC32 differs from computed CRC32:
51023d2a/3c8830cd, e933111b/74ea2243, 131d5339/13a89bc7 respectively.
Research decompression on private copies explicitly bypassed CRC checking.
H3Forge's existing strict container reader would reject these originals.
No container policy was weakened, no originals repaired and no gameplay
corruption inferred solely from that checksum mismatch.

Result: these samples do not contain a demonstrated change to color-team
assignments, so team writes remain disabled. The next useful pair must keep
Player2 as Blue in both games and change Blue's team in a copied map's team
settings, with both map variants supplied. Do not merely move Player2 to Tan.
That pair would establish initialization evidence only; ongoing-save
consistency and HotA load/use validation would still be required.

Verification for this follow-up: bounded private-copy decompression and
three-way comparison; existing Rust player_details map example succeeded.
Research artifacts stay under ignored .local/fixtures/teams-030 and
.local/research/teams_comparison.py. No production code changed in this
follow-up, so no frontend build or unrelated test suite was repeated.

- [x] Research required references and existing player parsing.
- [x] Keep unverified operations disabled, without a backend team-write path.
- [x] Show current-save unknown separately from authored relationships.
- [x] Locate the 44/11 team table structurally (join scope only).
- [x] Implement checked BytePatch planning and pending/Revert/Discard/persistence.
- [x] Test joins, exact diffs, other-player isolation and save/reopen.
- [x] User-validated in HotA: alliances after load, after ending a turn, and
  after a native save and reload (two saves, both directions).
- [ ] Split into a new team, emptying a team, no-team saves: unverified, refused.

Verification:

- Four-language rendering suite: 2 tests passed. The team gate is disabled,
  accessible and Unknown both without a map and with authored team/allies/enemies.
- TypeScript check and one completed Vite production build passed. The first
  build invocation stopped before Vite on an unsupported assert.match typing in
  the new test; corrected to assert.ok with RegExp.test, then rerun successfully.
- Copy-only player_flow with the current games fixture ran without skipping,
  but stopped at its pre-existing Blue-status expectation: actual Active versus
  expected Unknown. No Rust code changed. Persistence and exact-byte regression
  verification were not reached; this is not reported as a passing round trip.
- No team round-trip or exact-diff tests are claimed while no safe operation
  exists. No original H3M, GM2 or game asset was modified.

## New map pair and CRC fix (2026-11-14)

The requested map pair is now supplied. Crimson and CloverA_blue_in_2_team.h3m
and separate_team_map_2.GM2 genuinely put Blue in team 2 (zero-based 1).
Do not repeat the request for that initial comparison. Rust map parsing and a
bounded day-1 GM2 header walk agree: old assignments [0,0,1,0,1,0,0,0], new
[0,1,1,0,1,0,0,0], count 2 in both. Old count/Blue offsets are 474/476; new
476/478. The map name grew by two bytes. The copied H3M also changed from RoE
format 14 to HotA format 32; random AI heroes differ. These are initialized
team fields, not a verified complete ongoing-game edit plan. Details and the
new sample hash are in SAVEGAME_FORMAT.md's H3F-030 section.

The native CRC open failure is fixed. Regular H3SVG 44/11 import accepts the
known native trailer-CRC mismatch with a typed diagnostic and visible notice
in all four languages. Other versions/campaigns remain strict. Bounded
DEFLATE completion, full trailer, matching ISIZE, header CRC and exact
single-member boundary remain mandatory. This cannot distinguish every
valid-stream corruption from a native checksum defect; the UI says so.

Original compressed bytes remain untouched and are used for backups. Save
and Save As write a correct CRC and reopen temporary output through the new
strict reader before replacement. No input files were repaired or modified.
This supersedes the previous notes that the strict importer rejected these
samples; output verification remains strict.

All four samples now open with 215 heroes. Existing town/player discovery
returns 2 towns / 0 verified players for same_teams and separate_teams, and
6 towns / 8 players for same_teams_restart and separate_team_map_2. The initial
integration test's assumption of eight players in every sample failed; the
final CRC regression uses an independently supported Red-hero mana edit.
No unrelated town/player discovery changes were made to force this test.

Verification:

- 12 existing container tests passed; 3 new native-CRC tests passed. They cover
  unchanged original/payload bytes, strict-output rejection, normalization,
  exact version/magic boundary, size failures, truncation, malformed DEFLATE,
  flags, limits, trailing data and multiple members. The first combined test
  invocation filtered out the new integration tests; they were explicitly
  rerun without the filter and passed.
- Native application flow passed without fixture skips for all four supplied
  saves: open, Revert, Discard, pending edit, backup, Save As, Save and strict
  reopen. Setting Red's mana to 99 changes exactly one payload byte at
  342097, 342096, 342092 and 342089 respectively; every other byte, including
  the initialized teams and all other players, remains identical. A subsequent
  direct Save to the generated copy sets mana to 100 and strictly reopens.
- Four-language rendering suite: 2 tests passed, including visible CRC notice,
  no notice for clean imports and disabled unknown-team selector provenance.
- TypeScript and one completed Vite production build passed. Initial Vite
  process creation was blocked by sandbox EPERM; approved retry succeeded.
- Desktop custom-protocol build passed with the rebuilt frontend embedded:
  target/debug/H3Forge.exe. No native interactive or HotA load test claimed.
- Documentation diff/whitespace review passed. Original games/ and Maps/ files
  remain unchanged; test copies and backups stay under ignored .local/fixtures/.

Still blocked: safe ongoing merge/split, current alliance authority and any
serialized caches, historical exploration/puzzle sharing, terminal victory/loss
and participation effects. Joining or creating a team is not exposed, including
through backend commands. The next evidence must concern HotA load/use and
later-game dependent-state behavior, not another request for the initial pair.

## Requested town/player discovery fix (2026-11-14)

Status: completed and sample-verified for all four supplied saves; ongoing team
reassignment remains blocked as described above.

Root cause: inherited town-name constraints rejected FF, which is CP1251 "я",
and limited names to 14 bytes. The third town in same_teams is the 14-byte
"Упавшая Звезда"; the third in separate_teams is the 16-byte "Забытая Крепость".
The scan stopped at that gap, leaving two towns instead of six. The player
parser then refused the incomplete anchor table as designed.

Implemented bounded 1-16-byte legacy town names, preserving FF. Tightened
record/name boundary checks so incomplete data cannot panic or become a
truncated verified span. No resource offsets or player safety gates changed.
All four saves now expose 6 verified towns and 8 editable player slots.
Existing pending/Revert/Discard/Save/Save As workflows work without UI changes.

Verification:

- 8 town locator unit tests passed, including both native name cases, following
  town preservation, unsupported names and truncated payload/section bounds.
- 5 synthetic town/edit tests passed, retaining unknown-creature and malformed
  garrison gates plus exact patch behavior.
- Native CRC/hero application regression passed for all four saves and now
  asserts the six-town/eight-player result.
- New native_player_discovery regression passed without skips: 224 independent
  resource edits (4 saves x 8 slots x 7 resources), exact four-byte field
  isolation and strict reopen; per-player Revert, Discard, Save As, Save,
  original-byte backups and external-source-change rejection. A deliberately
  incorrect town count still prevents resource edits.
- Larger SaAndVla/map cross-check passed without skipping: 48 towns agree on
  names, positions and declared factions. Initial run skipped without map-root
  configuration; configured run exposed the historical fixed expectation of
  37. Updated only this cross-check to require a nonempty map table and exact
  save/map count equality, retaining all per-town checks; current 48-town pair
  passes. Other historical fixture expectations were not changed.
- Desktop custom-protocol build passed; target/debug/H3Forge.exe rebuilt.
  No frontend source changed in this follow-up, so no frontend build repeated.
  Native visual and edited-save HotA load checks remain manual.

Original saves/maps/assets were not modified. No unrelated issue was started.

## Requested temporary experiment (2026-11-14)

Completed the requested temporary solution as a Rust example tool,
`crates/h3forge-save/examples/experimental_teams.rs`. This explicitly authorized
experiment supersedes the earlier blanket no-writer statement for the four
fingerprinted original samples only. The desktop team selector remains disabled.
This is not a claim of safe general ongoing-game alliance editing.

The tool accepts input, new output, color index (0 Red, 1 Blue, 2 Tan, 4 Orange)
and one-based existing team (1 or 2). SHA-256 pins the entire original file before
using sample-specific offsets. It refuses unknown/edited inputs, inactive colors,
unchanged assignments, new teams and emptying a team. Checked expected-old-byte
patching changes exactly one assignment byte. Count, caches, exploration, visits,
victory state and all other payload bytes remain unchanged by design; whether
HotA recomputes dependent state is the question this experiment tests.

Ready-to-load private outputs:

- `.local/fixtures/teams-030-experimental/Blue_join_Tan_Orange.GM2`, from
  `same_teams_restart.GM2`: Blue moves from team 1 to team 2.
- `.local/fixtures/teams-030-experimental/Blue_join_Red.GM2`, from
  `separate_team_map_2.GM2`: Blue moves from team 2 to team 1.

Reproduction (choose an unused output filename):

```powershell
cargo run -p h3forge-save --example experimental_teams -- games/same_teams_restart.GM2 .local/fixtures/teams-030-experimental/another_test.GM2 1 2
```

Output is compressed and strictly reopened before creation, opened exclusively
with `create_new`, synced, read back and strictly reopened from disk. Existing
destinations cannot be replaced. Originals are never written, so no overwrite
or backup workflow is introduced. A failed disk write may leave a partial new
output and reports an error; it never affects the source.

Verification: two focused tests passed (unknown/invalid input and existing-file
preservation); both real outputs passed exact one-byte payload checks, correct
CRC strict reopen and disk readback. Formatting passed. No in-game success is
claimed. Load each copy in HotA and inspect alliances, friendly/enemy interaction,
shared vision and turn advancement; save under a new name and reload to check
persistence. Record any unexpected victory or defeat. The new native save and
observed behavior will guide further implementation. No other issue was started.

## Team reassignment delivered (2026-11-14)

Supersedes the "blocked" statements above for one operation: moving a
participating player into another team that already exists.

Game evidence (user-run, HotA, both experimental saves): `Blue_join_Tan_Orange.GM2`
and `Blue_join_Red.GM2` loaded, showed the changed alliances, a turn was
ended, and the game was saved natively as `*_after.GM2` and reloaded. The user
reports that it works. Rechecked here: each edited save differs from its
original by exactly one assignment byte, and both native `_after` saves still
carry the edited table (count 2; `0,1,1,0,1` and `0,0,1,0,1`) with unchanged
participation. The game therefore reads and rewrites this table as the
alliance source; no second team field had to be changed.

Verified facts:

- The table is `count:u8` + `team[8]` directly after the loss condition in the
  embedded map header. A structural walk (name, description, eight player
  specs, victory, loss, teams, custom heroes) ends on three anchors: the
  `00..07` identity row, eight starting factions and the `.h3m` file name.
  It resolves all 12 reference saves in `games/`: offsets 475, 477, 431
  (special victory type 7, 3 custom heroes) and 528 (named starting heroes,
  5 teams, 214 custom heroes).
- A join is one same-length byte. Count and every other byte are untouched.

Design choices:

- Core: `h3forge_save::teams` with `SaveDocument::team_table()` and
  `plan_team_edits(&[TeamEdit])`. Regular HotA 44/11 only. All moves are
  planned jointly; each produces one checked `BytePatch` with the expected
  old byte. Refused: unknown victory/loss types or failed anchors, saves with
  fewer than two teams, colors not in the game (no starting faction, absent
  player type or not playable), teams that do not exist, duplicate moves, and
  any plan that leaves a team without a participating player.
- Desktop: `apply_team_edit(id, team)` (one-based team), pending moves shown
  in the pending list, cleared by Revert player and Discard, written by Save
  and Save As through the existing backup and strict reopen path. Revert of
  one player is refused if it would empty a team given other pending moves.
- UI: Players -> General has an enabled Team (current save) selector listing
  existing teams with their members. Targets that would empty a team are
  disabled. The list row shows the current save team. Map team, allies and
  enemies remain authored H3M facts. Four languages.

Unresolved, intentionally not done:

- Creating a team (count + 1) or removing one (renumbering) was never loaded
  in the game. A player who is alone in a team cannot be moved.
- Saves with team count 0 store no assignment bytes; adding them would change
  the payload length.
- Explored map, visited objects, obelisk/puzzle progress and tents are not
  merged or split. Whether HotA shares vision retroactively after a join was
  not inspected byte-wise; the UI states that only the assignment changes.
- "Participating" comes from starting settings. A defeated player still
  counts as a team member for the empty-team check.
- Special victory/loss conditions with a changed team were not played.

Verification:

- 1 synthetic unit test (variable header, no-team, unknown victory type,
  out-of-range team, truncation, other version).
- `sample_teams` passed without skips on six saves: offsets, counts, tables,
  every legal single move as an exact one-byte diff with strict reopen,
  refusal of every team-emptying move, joint swap, and byte-exact
  reproduction of both game-verified saves plus the `_after` tables.
- Desktop `team_flow` passed: options, refused requests, pending entry,
  Revert, Discard, Save As (single differing byte, source untouched), Save.
  `command_permissions`, `native_player_discovery` and `native_crc_flow` passed.
- TypeScript check, i18n render suite (2), list tests (31) and one Vite
  production build passed. fmt check passed; clippy shows no new warnings in
  non-test code.
- Not done: native window click-through of the new selector, and loading a
  save written by the app itself (as opposed to the byte-identical
  experimental files) in HotA.

`crates/h3forge-save/examples/experimental_teams.rs` is superseded by the
application feature; it stays SHA-pinned to the four samples and was left in
place. Originals in `games/` were only read. No other issue was started.
