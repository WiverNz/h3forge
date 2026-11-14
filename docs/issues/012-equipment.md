# H3F-012 - Edit validated equipment slots and coupled bonuses

Status: completed. Milestone: M4. Priority: P1.

Manual update recorded 2026-11-14: the user verified adding artifacts to heroes
in HotA. Placement and artifact IDs were not specified, so this does not establish
wear-slot swaps/unequips, attribute-bonus coupling or granted-spell coupling.
Those equipment-specific operations remain automated-only. Historical manual
limitations below apply to these untested cases, not to all artifact addition.
See the [manual checklist](../MANUAL_COMPATIBILITY.md).

## Goal

Equip supported artifacts without corrupting stats, granted spells or slot reservations.

## Scope

Port slot compatibility and conflict logic, primary bonus adjustments and equipment-granted spells. Handle war machines/spellbook through explicit actions only when validated. Support ordinary equipment first; combined artifacts or HotA reservation values with unknown meaning must stay blocked until a specific demonstrated case establishes safe behavior. No blanket rewrite of reservation bytes.

## Relevant files or components

save definitions/edit equipment rules and patch planner; web equipment panel; app change summary.

## Dependencies

H3F-007, H3F-011 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [x] A normal supported artifact swap updates only proven slot/bonus/spell fields and survives save/reopen.
- [x] Conflicting or unsupported combinations are rejected with actionable messages.
- [x] Unknown HotA reservation state is preserved; a restriction cannot silently become data loss.
- [x] Removing an artifact does not remove a separately learned spell.

## Implementation notes (H3F-012)

- `HeroEdit::SetEquipmentSlot { slot, item }` with `EquipmentEdit::{Empty, Artifact(id)}`
  plans the 8-byte slot word plus the verified couplings, nothing else.
- Slot compatibility (`artifacts::slot_kind`), stat bonuses (`artifacts::stat_bonus`,
  h3sed `ARTIFACT_STATS`) and spell grants (`artifacts::spell_grants`, Tomes 86-89 and
  Spellbinder's Hat 124) are ported from h3sed `metadata.py`, names crossed to binary IDs.
- Coupling semantics: stored attributes include equipped bonuses, so equip/unequip
  rewrites 238-241 by `(stored - old_bonus) + new_bonus` mod 256 (h3sed
  `update_primary_stats`); available flags 312-381 become
  `(learned && has_spellbook) || granted` with the bannable conditional
  (Summon Boat 0, Scuttle Boat 1, Water Walk 7; h3sed `hero/spells.py:serialize`).
  Learned flags are never written.
- Hard blockers (whole section, explicit reason): any wear slot occupied by a
  combination artifact, equipped scroll, or unknown entry; any non-zero byte in the
  1046-1055 combination reservations. War-machine slots 13-16 and the spellbook
  slot 17 are always read-only.
- Equipment<->backpack moves are one `SetEquipmentSlot` + one `SetBackpackSlot` edit
  (swap or clear) sent in a single request, so nothing is duplicated or lost.
- Identical patches computed by two edits in one batch collapse; divergent overlaps
  are rejected as an explicit conflict (`resolve_patch_conflicts`).
- Artifact icons are not bundled (no verified mapping exists); the editor uses the
  same placeholder diamonds as the backpack. Equipped scrolls stay read-only.
- Remains read-only: combination artifacts and their assembly/disassembly, war
  machines, the spellbook, equipped scrolls, Grail equipping.

## Minimal verification

Focused ordinary swap and spell-grant removal tests, plus slot conflict and opaque-reservation rejection. Inspect payload diff and load one changed copy in game for newly enabled coupling.

Verified: 7 unit tests in `edit.rs` (equip/unequip involution, replace delta, grant
add/withdraw, bannable conditional, slot/artifact/blocker rejections, batch conflict
dedupe/rejection) and `tests/sample_equipment_roundtrip.rs` against a copy of
SaAndVla.GM2 (equip + unequip restores the payload byte-for-byte; diff limited to the
patch union; source untouched). Equipment-specific swaps/unequips and coupled effects still require manual verification; generic artifact addition is covered by the report above.

## Visual artifact editor follow-up - completed (2026-11-10)

Supersedes the earlier placeholder-artwork and whole-spell-array coupling notes.
The requested H3F-012 usability follow-up is complete; H3F-013 is not started.

- Equipment uses a compact labeled 3×3 body-position arrangement, five miscellaneous
  squares, and a separate read-only war-machine/spellbook section. Full names,
  verified modeled effects and blocking reasons appear on hover/selection.
- Backpack retains all 64 fixed positions in an eight-column icon grid (four on
  narrow panels). Selection exposes replacement, removal, or move-to-empty-slot;
  no persistent remove buttons, sorting or compaction.
- One shared searchable image picker opens at a time, with arrow/Enter/Escape
  navigation, full wrapping names, effect/slot hints and lazy icon requests.
  Equipment choices are compatible supported backpack artifacts; replacement swaps
  the displaced item back into that same backpack position.
- **163 original icons** resolve from local resources: base IDs 0–140, HotA IDs
  141–143 and 147–165. Base 141–143 are unavailable tiles and explicitly excluded.
  See [resource evidence](../formats/GAME_RESOURCES_FORMAT.md#artifact-artwork-and-visual-editor-follow-up-2026-11-10).
- Global equipment blocking was **not narrowed**: unknown effects and reservation
  ownership are not proven independent. Exact item/slot and reservation offset/value
  diagnostics replace the generic reason. Unsupported combinations, equipped scrolls,
  special slots and incompatible placements remain blocked; original bytes survive.
- Pending primary attributes now reflect checked equipment bonus patches immediately.
  Save/Save As, backup/replacement, Revert and Discard mechanisms remain in use.

Verification: original-resource icon coverage/pinned pixels/fallback; focused
Rust equipment gates and reservation-preservation tests; real-sample session flow
covering backpack replacement/move, equipment replacement/swap, pending primary
bonus, Save As/reopen and both Discard variants. Final workspace check and frontend
build recorded with the completion result. Manual verification of this replacement/transfer/coupling flow remains open; the reported artifact-addition pass is narrower.

Final verification (2026-11-10): 3 local-resource tests passed (163 icons plus
fallbacks); 9 focused equipment/locator tests passed, with the precise blocker
and reservation-preservation case rerun successfully after its diagnostic fix;
1 real-sample core equipment round trip and 1 real-sample application replacement/
transfer/Save As/discard flow passed. `cargo check --workspace`, `cargo fmt --all`,
frontend TypeScript + Vite production build, and `git diff --check` passed.
Fixtures ran with absolute roots and did not skip. No native-UI or in-game manual
smoke test was performed in this follow-up. No commits, releases or original game/
save data modifications were made.

## Combination, HotA and in-place equipment follow-up - completed (2026-11-10)

Supersedes the "combination artifacts stay blocked" notes above; the scope
condition (a specific demonstrated case) is met by sample evidence recorded in
[the format spec](../formats/SAVEGAME_FORMAT.md#combination-artifacts-hota-artifacts-and-in-place-equipment-2026-11-10).

- **In-place change:** clicking a wear slot opens the picker with every
  artifact that fits it (compatible backpack items first, which still swap).
  Remove and Move to backpack remain explicit actions.
- **HotA and expansion artifacts** (127–143, 147–165) are writable and
  equippable; the "Unsupported item" state is gone for catalog artifacts.
- **Combination artifacts** can be equipped, replaced and removed. Slots they
  take are shown as taken (positions follow the h3sed display convention; the
  save stores counters only). Equipping needs those slots free and is rejected
  with the items to unequip otherwise. No auto-assembly or disassembly.
- **Planner:** all equipment edits of a batch are planned as one desired
  state, so two stat-changing slot edits no longer conflict. The session
  validates equipment edits against the merged pending set.
- **UI bug:** the equipment and backpack panels shared one React key, which
  left the previous hero's panels mounted when another hero was selected.

Verification: 68 save-crate unit tests (3 new combination cases, table
consistency), the real-sample reservation test over all four saves, and a
real-sample session flow (`combination_and_in_place_equipment_flow`) covering
display, refusal, in-place replacement, Save As and reopen. Frontend typecheck
passes. Not verified: the native window and this combination/in-place equipment flow in-game. The generic artifact-addition report does not specify combination artifacts.

### Combination picker follow-up (2026-11-10)

- A combination artifact is offered from **any slot it takes**, not only its
  primary slot (Cloak of the Undead King appears under Necklace and Boots).
  Choosing it there equips it in its primary slot and clears the clicked slot
  in the same request (`equipmentSecond`); its other slots must already be free.
- Components carry "Part of <combination>" in their description (hover, picker,
  backpack), from `artifacts::combination_of`.

Verification: table test (every component maps back to its combination) and
the real-sample session test `combination_equipped_through_secondary_slot`.
Native window and this secondary-slot combination workflow remain unverified in-game.
