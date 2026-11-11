# H3F-019 - Read-only hero and town coordinates, gated map-position edits

Status: planned. Milestone: M4. Priority: P1.

## Goal

Show where heroes and towns sit on the map, and only then consider moving them - coordinate edits are a separate high-risk feature that may stay unimplemented.

## Scope

Display hero and town X/Y/Z coordinates read-only. Evidence: h3sed `hero/profile.py` reads hero location from bytes before the fixed record block (with an optional biography caveat requiring bounded validation), and the h3sed `TOWN_REGEX` (version/hota.py) captures town x/y/z at record offsets 4–6; the h3svg specification documents coordinate fields for both. Hero coordinates may additionally exist in the map object index - h3sed itself treats profile location as heuristic. Writes (moving a hero) are out of scope until every linked structure is understood: the map object entry, any visitable/garrison references in towns, path/passability state, and the hero's own record must stay consistent, or the game may crash or duplicate the hero.

## Relevant files or components

crates/h3forge-save/src (profile/coordinate parsing); hero detail and future town UI.

## Dependencies

H3F-013 for town records. See the [issue index](README.md) and [architecture](../ARCHITECTURE.md).

## Acceptance criteria

- [ ] Coordinates parse read-only for verified heroes and located towns, with the biography-layout caveat surfaced as a note when applicable.
- [ ] No coordinate write operation exists until linked structures are identified and tested; until then the fields stay visibly read-only.
- [ ] If evidence stays ambiguous, the issue closes with coordinates read-only and the blocked-write rationale recorded.

## Minimal verification

Cross-check parsed coordinates against known map facts for a few sample heroes/towns (town positions also appear in the h3m-adjacent map data). No writes, no new pipelines.
