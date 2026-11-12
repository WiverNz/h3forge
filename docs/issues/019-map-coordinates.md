# H3F-019 - Hero and Town Coordinates

Status: constrained arbitrary-distance relocation implemented (2026-11-12). Native game-load validation and unsupported transitions remain open.
Milestone: M4. Priority: P1. Dependencies: H3F-008, H3F-013, H3F-021.

## Goal and scope

- Town coordinates: read-only X/Y/Z.
- Hero coordinates: readable and editable X/Y/Z, allowing relocation to another
  valid map tile once every required linked structure is verified.
- Validate current save map bounds, terrain, passability, occupied tiles and
  surface/underground levels. Original H3M positions are reference data, never
  current occupancy or a write destination.
- Handle land and water safely, including boarded boats, underlying objects,
  movement/path state, town links and exploration. Unsupported movements stay
  disabled with an explanation.
- Coordinate edits must become checked same-length pending patches, integrated
  atomically with other edits, Revert, Discard, Save and Save As. Reject conflicts
  with dismissal, shared source/destination tiles and any linked pending changes.

This replaces the previous read-only plan. Coordinate editing is implemented
for the supported cases below, not limited to a one-tile step.

## Evidence and result

Compared all decompressed bytes of these private saves, copying them into ignored
`.local/fixtures/` first:

| Save | Observed current position | Payload bytes |
|---|---|---:|
| `SaAndVla_before_movement.GM2` | Moandor (231,52,0), Nimbus (76,41,0) | 5,012,342 |
| `SaAndVla_moandor_left_1.GM2` | Moandor (230,52,0), Nimbus unchanged | 5,012,397 |
| `SaAndVla_nimbus_water_step.GM2` | Nimbus (77,42,0), Moandor unchanged | 5,012,409 |

The after-saves are independent branches, not sequential moves. Exact changed
ranges, replay bytes, unchanged linked regions and interpretations are recorded
in [the save specification](../formats/SAVEGAME_FORMAT.md#h3f-019-coordinate-movement-evidence).

Both moves change the hero's two coordinate representations, facing, destination,
movement remaining, a movement-associated byte at hero +1069, and source/destination
tile occupancy. Object/template arrays, render lists, boat records, town records
and hero ownership tables do not change in either pair. The patch relationships generalize to arbitrary distance when the source and
destination satisfy the same validated passive-tile shape. Movement cost and
fog revelation are separate game actions, not necessary coordinate patches.

Moandor's underlying object class is 3, not the empty class 0; the move restores
class 3 on the vacated tile. His after-save also changes Nimbus's planned
destination without moving Nimbus. Nimbus's water step changes a visibility-like
byte `7E -> FF`, with a corresponding replay event. Boat 30 remains byte-identical,
including stale coordinates (70,52,0). Copying hero coordinates into that boat
would contradict the observed native step.

The embedded filename and player selection are incidental UI/save state. Replay
growth is isolated (55/67 bytes); all bytes after the aligned insertion are
identical. No RNG change is established. The other-hero destination is treated
as incidental to Moandor relocation, with its cause unproven. Exploration is movement-related, not incidental noise; editor relocation
explicitly preserves exploration instead of simulating revelation. It does not
block position editing under that policy.

## Implemented scope and policy

- Rust `SaveDocument::hero_coordinates` validates a unique bounded biography
  boundary, both coordinate encodings, map bounds and level count.
- `HeroEdit::SetCoordinates([x,y,z])` uses checked same-length patches and the
  existing pending-edit pipeline. Compact X/Y inputs submit one atomic position;
  Z is displayed read-only because cross-level relocation is currently blocked.
- Supported: arbitrary distance on the same level and medium, owned adventuring
  heroes, passive underlying class 0 or 3, and a destination with exactly the
  same underlying class/properties. Water requires one verified boarded boat.
- Validate current tile occupancy, owner roster, town links, terrain, accessibility
  bits and every current render reference's passability/action mask. Reject
  rock, occupied/actionable tiles, adjacent monsters, ambiguous records, unknown
  movement/spell state, patrol heroes, town visitors/garrisons, boarding/landing,
  mismatched underlying state and level changes.
- Update both hero coordinate forms and source/destination occupancy, clear X/Y
  route, and set the observed movement-associated byte +1069 to 1 (only values
  0/1 accepted). The byte's exact name remains unknown; its native 0 -> 1 change
  is reproduced, not interpreted as sleeping or another guessed field.
- **Editor policy:** retain movement points, facing, destination Z/unknown route
  bytes, fog/exploration, selection and replay history. No invented distance cost,
  free exploration, combat or automatic object visits. Boarded boat records remain
  unchanged exactly as in Nimbus's native move.
- Multiple pending relocations must have different destinations. Occupied source
  tiles cannot serve as another pending destination; swaps/chains stay refused.
  Dismissal and relocation of the same hero conflict in both orders.
- Revert, Discard, Save and Save As use the existing checked patch/backup/reopen
  path. Town coordinates remain read-only. Originals and H3M are never edited.

## What was ambiguous, and what was not

| Difference/relationship | Conclusion for relocation |
|---|---|
| Hero front matter and tile occupancy | Required; exact values and restoration rules are already established by the existing pairs. |
| Class 3 beneath Moandor | Known passive source/destination shape in this pair, not grounds for requesting the same action again. Its actionable variants remain excluded. |
| Old monster/resource object entries | Historical entries remain in the object array after removal. Use live tile/render references, not all objects' original masks. |
| +1069 | Exact semantic name unknown. Both native moves set 0 -> 1; reproduce that transition, reject other values. |
| Facing and remaining movement | Normal-step effects. Explicitly preserved for editor relocation, so no extra walking pair is needed to derive a distance cost. |
| Planned X/Y route | Clear to -1/-1 as in both pairs. Preserve other route bytes. |
| Nimbus route changed in Moandor save | Unrelated to Moandor position; preserve. No need to ask the user to eliminate it. |
| Selected hero and embedded filename | Incidental UI/save metadata; preserve. |
| Replay insertions | Historical playback, not current occupancy; preserve under the editor policy. |
| Nimbus fog byte 7E -> FF | Exploration side effect. Preserve fog; implementing automatic reveal would separately require its boundary/sharing rules. |
| Boarded boat stale coordinates | Verified unchanged. Do not move them to the hero's new tile. Boarding/landing is a different operation. |

The supplied [H3Hero definition](https://github.com/RoseKavalier/H3Plugins/blob/master/H3API/lib/h3api/H3Structures/H3Structures.hpp)
corroborates the position, underlying-object, route and movement concepts.
It is an in-memory classic-game structure, not a HotA serialization layout:
for example, `objectBelow` is at memory +0xB while the save stores class there
and the boolean at front +15. No memory offsets were copied into save writes.

**The previous request for another one-tile plain-land pair is withdrawn.**
It would not establish arbitrary relocation or resolve the important transition
rules. No additional pair is needed to use the supported editor operation.

If boarding/landing support is requested next, the specific new experiment is
one pair immediately before and after boarding an adjacent empty boat, with no
other action. Unlike another land step, this changes medium and would identify
boat existence/visibility, rider/owner, parked coordinates, both tiles and hero
state during attachment. Disembarking needs its own separate pair. Cross-level
moves and actionable-object visits remain independent unsupported transitions.
This is a description of missing evidence, not a request to repeat the supplied
experiment.

## Acceptance criteria

- [x] Town X/Y/Z read-only and validated current hero coordinates.
- [x] Compact coordinate inputs with pending state and refusal explanations.
- [x] Exhaustive byte comparison and incidental/required/policy separation.
- [x] Checked same-length hero relocation for the supported passive-tile shape.
- [x] Bounds, current terrain/accessibility/render masks, occupancy and levels
  validated; cross-level moves rejected.
- [x] Land-to-land and boarded water-to-water relocation; transitions disabled.
- [x] Pending edits, Revert, Discard, Save/Save As and collision/dismissal gates.
- [x] Native-byte projection, arbitrary-distance and return-move round trips.
- [ ] Native HotA load, movement and end-turn validation of edited copies.
- [ ] Expand to cross-level/medium transitions or different underlying states
  only with their own evidence and conflict rules.

## Verification

- Private `sample_coordinates` regression: 1 passed, no skipping. Every changed
  byte range, both replay insertions, unchanged arrays and all six observed
  hero positions checked; compression/reopen preserves every payload byte.
  Planner writes agree byte-for-byte with both native steps; the exact remaining
  differences are exclusively the documented policy/incidental fields. Long-distance
  land/water relocation and return restore all bytes except cleared routes and
  +1069. Corrupted destination/position/boat states are rejected.
- Focused coordinate parser unit test covers biography, full coordinates,
  duplicate agreement, bounds, level count and off-map refusal. It compiled,
  but Windows denied execution of the library-test binary (then reported a
  file lock), including outside the sandbox. A separate target directory was
  also denied. This unit test is not reported as passed; the private integration
  and desktop tests above/below executed successfully.
- Desktop `coordinates_flow`: 1 passed, no skipping. Coordinates and actual relocations survive
  pending attribute edits, per-hero Revert, Discard, Save As and Save. Both
  dismissal conflict orders and a real second-hero destination collision are
  tested. Original save stays byte-identical.
- Shared map-discovery regression `sample_neutral`: 1 passed, 291 groups and
  47 garrisons retained. Its optional H3M cross-check was not enabled.
- Frontend production build/typecheck and two localized rendering tests,
  now including enabled/blocked coordinate controls in four languages, passed.
  Sandbox child-process EPERM was resolved by approved execution outside the
  sandbox. Native visual inspection remains manual.

No original save or H3M was modified. No other issue was started. H3F-019 remains
open for manual native-game validation and the explicitly unsupported cases.
