# H3F-023 - Player details, alliances and map revelation

Status: partially complete (verified read-only details implemented; both reveal actions safely blocked). Milestone: M4. Priority: P1. Dependencies: H3F-018, H3F-021.

## Players UX and status follow-up (2026-11-10)

- Resources is the default tab and resets on player selection. General and
  Exploration have separate compact tabs with keyboard navigation. The flag,
  color identity, status and Revert remain in the persistent header. Seven
  icon/input rows occupy a two-column grid, falling back to one column on
  narrow windows. Pending edits survive tab and player changes.
- Disabled fields previously came from a Rust policy requiring at least one
  hero or town, propagated through `readOnlyReason`; they were not evidence
  of invalid resource offsets. All eight sample records pass structural and
  ownership checks. Removed only that asset-presence gate. Ambiguous records,
  unsupported layouts and negative stored resources remain read-only with
  specific explanations. Bounds, typed patches and save protections remain.
- Status is Active only for structurally verified current ownership.
  Defeated / Eliminated and Unused / Never participated are explicit reserved
  statuses; neither is emitted without historical evidence. Empty or ambiguous
  records are Unknown. H3M eligibility never supplies current controller or
  participation status. Small indicators appear in both list and header.

| Sample player | Status | Verified heroes / towns | Confidence |
|---|---|---:|---|
| Red | Active | 5 / 0 | Verified current ownership |
| Blue | Unknown | 0 / 0 | Defeat versus unused unresolved |
| Tan | Unknown | 0 / 0 | Defeat versus unused unresolved |
| Green | Unknown | 0 / 0 | Defeat versus unused unresolved |
| Orange | Active | 5 / 19 | Verified current ownership |
| Purple | Unknown | 0 / 0 | Defeat versus unused unresolved |
| Teal | Active | 4 / 14 | Verified current ownership |
| Pink | Active | 5 / 0 | Verified current ownership |

The local h3svg Player specification names +23 `days_left`. Signed sample
values are `[6,2,7,5,-1,-1,-1,6]`; this is no-town countdown information,
not proof of defeat. No remaining opaque player flag was established as a
loss/history marker. All eight linked H3M slots are playable, which does not
prove participation in this saved game. Resource amounts and obelisk visits
are not used to manufacture historical status. Resources remain at record
+98 through +125, seven i32 values with supported inputs 0..2147483647.

Follow-up verification on private copies:

- `sample_players`: 1 focused test passed without skipping, including 56
  independent resource save/reopen round trips across all eight slots.
  Every single edit preserves all bytes outside its four-byte field and
  preserves player status. Ambiguous ownership remains Unknown/read-only;
  changing an empty slot's countdown never manufactures a defeat status.
  Invalid, overflowing, conflicting and stale patches remain rejected.
- `player_flow`: 1 focused test passed without skipping. Blue (Unknown) is
  editable. Revert, Discard, change-back suppression, Save As, direct Save,
  backup and source-change rejection pass. Blue Gold = 2147483647 and Teal
  Wood = 0 change exactly offsets `4635265,4635266,4635267,4635268,4635986,4635987`;
  all other 5,271,288 payload bytes are identical on reopen.
- One final workspace check, TypeScript/Vite build and desktop build passed.
  Rebuilt executable: `target/debug/H3Forge.exe`.
- Native visual verification could not be completed: the automation helper
  returned screenshots of a different foreground window and rejected input.
  Exact 1280x720 visual fit and actual game loading remain manual checks.
  No verified defeated/unused fixture exists, so those classifications are
  not claimed tested. Revelation remains disabled and unchanged.

## Scope

Extend the compact Players panel with original flags, map-defined eligibility,
teams and starting information. Keep current save controller and ownership
distinct from authored map facts. Preserve existing resource edits.

Investigate HotA 44/11 exploration and obelisk/puzzle state. Enable reveal
operations only when every required structure and sharing rule is verified.
Otherwise expose disabled controls explaining the exact blockers. Never use
VCMI serialization as a layout for original Heroes III saves.

## Acceptance and verification

- [x] Player details carry explicit map/save provenance and unknown states.
- [x] Original flag, town and identified hero artwork use Rust resource loaders.
- [x] H3M obelisk locations/count and Grail declarations are read-only.
- [x] Reveal controls are verified end to end or explicitly blocked.
- [x] Focused private-copy tests, workspace check and frontend build pass.
- [x] Record exact payload preservation and remaining manual game-load gates.

No screenshots were available as attachments in this session. Follow the
existing compact dark desktop layout. No unrelated issue is started.

## Delivered

- Compact General, Resources and Exploration sections in Players. The list
  shows original banners, color, authored eligibility and team number.
  Existing resource editing and pending state are unchanged; map relinking
  refreshes the authored facts without discarding edits.
- All 8 original banners resolve from `CREST58.def`, using the existing
  bounded Rust decoder, PNG cache and installation identity checks. The
  two new artwork commands have explicit build-manifest registrations,
  invoke handlers and local-main-window permissions (no wildcard grant).
- Current player identity is the owner color. Personal player names and
  current Human/Computer controller are unknown; eligibility is labelled
  as map-defined. No controller/team editing operations are added.
- Map eligibility: Red/Pink Computer; the other six Human or Computer.
  One-based teams: Red 1, Blue/Purple 2, Tan/Green 3, Orange/Teal 4, Pink 5.
  Allies/enemies are authored map relationships, including inactive slots,
  never inferred from current ownership. GM2 visit bits agree for the
  allied pairs, but current alliance fields are not structurally verified.
- All eight main-town entrance associations resolve uniquely in H3M. Red
  starts with Inferno, Pink with Cove; the other six factions are random.
  Both fixed faction images load. These facts are not replaced with current
  owned towns or the randomly selected factions from unverified save fields.
- Two named starting heroes: Night King (type 150, map portrait 161) and
  John Silver (type 165, default portrait). The first uses the authored
  portrait; the latter uses the uniquely matched save hero's current
  portrait and explicitly labels that provenance. Both images load. The
  other six starting hero identities remain random/generated and unknown.
- Read-only list of 29 H3M obelisk sprite anchors with H3M object ordinals
  (12 surface, 17 underground), and one authored Grail placement count.
  GM2 count + 48 visit masks at 4,634,945 immediately precedes Player[8].
  The count matches H3M and the unused 19 mask bytes are zero. Displayed
  visits by color: `[0,5,15,15,17,5,17,1]`. These are visit totals, not a
  claimed puzzle-completion counter.

## Exact blockers

**Reveal entire map is disabled.** The map has 252 x 252 tiles on two levels.
A classic two-byte visibility/adjacent-monster interpretation leaves 32
candidate alignments in the sample. Every candidate marks all 33 currently
owned town tiles visible, so that check cannot select a boundary. Exact
start/end, tile order, any additional visibility state, and original HotA
team-sharing rules are unverified. No guessed offsets or all-ones patches
are implemented. Coordinates for heroes were not reverse engineered as a
side task for H3F-019.

**Reveal Puzzle Map (Grail) is disabled.** Visit masks/count are readable,
but H3M object ordinals have not been mapped to GM2 obelisk slots. Separate
puzzle counters, complete team update rules and current buried Grail state
are unverified. An authored Grail does not establish current availability.
No single-counter or mask-only write is offered. No reveal command/operation
exists behind either disabled control, so bypassing UI cannot grant writes.

Sources and exact field evidence:
[save format](../formats/SAVEGAME_FORMAT.md#h3f-023---read-only-obelisk-visits-and-exploration-gates),
[map format](../formats/MAP_FORMAT.md#h3f-023---authored-player-details-and-puzzle-objects),
[resources](../formats/GAME_RESOURCES_FORMAT.md#h3f-023---original-player-banners).
VCMI was consulted only for conceptual team/visit/counter coupling; its
serialization and source code were not reused.

## Verification

- `cargo test -p h3forge-map --test synthetic`: **7 passed**, including
  playable-team ID/count validation, unused slot handling, surface/underground
  obelisk bounds, Grail radius extraction and truncation failures.
- `H3FORGE_FIXTURE_ROOT=D:\Projects\h3forge\games cargo test -p h3forge-desktop --test player_details_flow -- --nocapture`:
  **1 passed without skipping**. Private GM2 and H3M copies under ignored
  `.local/fixtures/`; all eight player mappings, no-map unknowns, 29 obelisk
  locations/visits, invalid mask count/tail rejection, 8 banners, 2 town
  icons, 2 portraits, unknown-artwork placeholders, Revert/Discard and
  resource Save As/reopen verified.
- H3F-023 itself changes **zero save payload bytes** and has no H3M writer.
  The unchanged resource pipeline regression sets Orange Gold to 1,234,567:
  exactly payload bytes **4,635,712, 4,635,713, 4,635,714** change inside the
  four-byte resource word. All other **5,271,291** bytes are identical,
  including every other player's resources, all visit masks and the entire
  candidate visibility region. H3M copies remain byte-identical.
- `cargo check --workspace`: passed.
- `npm.cmd --prefix web --cache .local/npm-cache run build`: TypeScript and
  Vite production build passed.
- `cargo build -p h3forge-desktop --features custom-protocol`: passed with
  the rebuilt frontend embedded. Executable: `target/debug/H3Forge.exe`.
- Native interactive inspection and actual game loading are not claimed.
  No game compatibility claim is made for either blocked reveal action.

No H3F-019 or other unrelated issue was started. Remaining revelation work
requires new evidence, ideally paired original-game saves before/after
exploration and obelisk visits with known players/teams.
