# H3F-013 - Inspect towns and enable proven garrison edits

Status: completed (sample-verified; native window and in-game load pending). Milestone: M4. Priority: P1.

## Goal

Add town value while respecting the reference parser's heuristic limits.

## Scope

Port town candidate checks for names, types, army validity and adjacency before hero records. Bound every scan and record span. Show town names, type, owner, coordinates and garrison relationships read-only first. Enable garrison type/count patches only where exact spans and hero ownership are established. Other town fields remain read-only.

## Relevant files or components

save locate/parse town domain and garrison operations; app town DTO/commands; web town list/details.

## Dependencies

H3F-003, H3F-009 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are listed under Outcome.

## Acceptance criteria

- [x] Town overview identifies uncertainty rather than presenting every regex hit as a town.
- [x] A proven unoccupied town garrison can be edited and round-tripped without touching variable names or unknown tail bytes.
- [x] A town with a garrison hero directs editing to that hero and cannot independently overwrite its army.
- [x] No building/economy/ownership editor is added under this issue.

## Minimal verification

One synthetic false-positive/gap case and a private copy with established town spans. Test garrison-hero blocking and one valid garrison diff. If no reliable local town fixture exists, ship inspection only and leave write acceptance explicitly pending.


## Outcome (2026-11-10)

Garrison ownership follow-up (2026-11-10, under H3F-022): all 17 sample towns with a garrisoned hero store an empty garrison of their own, so the hero's army is the garrison and the town words stay closed. The Garrison tab now shows and edits that hero's stacks through the hero's own army edit and links to the hero's Army tab. Towns with only a visiting hero keep direct garrison editing.


Verified facts:

- **Discovery** (`crates/h3forge-save/src/towns.rs`): bounded scan of the region before the hero array, ported from h3sed `parse_towns` / `TOWN_REGEX` with attribution. Record length is learned from adjacency (`567 + name_len` on HotA 44/11). Records failing the secondary garrison checks stay visible as low-confidence candidates with explicit reasons and never accept writes.
- **Sample** (`SaAndVla.GM2`, 44/11): 37 towns, 37/37 spans exact by adjacency, type byte and coordinates equal to the source map's town objects, 17 garrisoned and 1 visiting hero resolved by `hero_type` with matching coordinates. Layout recorded in [SAVEGAME_FORMAT.md](../formats/SAVEGAME_FORMAT.md) section D.
- **Read-only view** (`domain.rs` `TownView`, app `TownListItemDto`/`TownDetailDto`, `web` `TownList`/`TownDetail`): name, type, owner, coordinates, mage guild level, garrison, garrisoned/visiting hero links, initial-map facts, diagnostics.
- **Write** (`edit.rs` `TownEdit::SetGarrisonSlot`, `plan_town_edits`): garrison creature ID/count words only, as checked same-length patches. Gate: exact 44/11, verified town, supported existing stack, no garrisoned hero. Pending town edits share the hero pending list, Save and Save As.
- **Garrison hero blocking**: a town with a garrisoned hero reports the owning hero, links to it in the UI and rejects town garrison edits in the planner.

Design choices: unverified candidates are listed (flagged) instead of dropped as h3sed does; no ownership, building, economy, spell or coordinate editing.

Follow-up fix (2026-11-10): the five town commands (`list_towns`, `get_town`, `apply_town_edit`, `get_town_edit_state`, `discard_town_edits`) were registered in the invoke handler but missing from the `build.rs` app manifest and `capabilities/main.json`, so the window refused them with "list_towns not allowed" as soon as a save was opened. Both lists are now complete, and `tests/command_permissions.rs` checks every registered command against both so this cannot recur silently. "Discard Changes" now also refreshes the town list and the selected town's garrison.

Verification: `towns.rs` unit tests and `tests/towns_synthetic.rs` (false positive, gap, candidate cases); `tests/sample_towns.rs` with `H3FORGE_FIXTURE_ROOT` and `H3FORGE_MAPS_ROOT` (discovery, map cross-check, edit validation, garrison-hero block, garrison round trip); desktop `tests/town_flow.rs` (pending override in list, garrison-hero block, Save As round trip and reopen) and `tests/command_permissions.rs`. Frontend typecheck passes.

Unresolved: the native window flow and loading an edited save in the matching game are not yet confirmed by the maintainer. Only the one 44/11 sample backs the layout; other HotA minors and SoD/RoE town layouts stay read-only or undiscovered (H3F-015).
