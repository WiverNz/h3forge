# H3F-015 - Add evidence-backed support for further save layouts

Status: planned. Milestone: M4. Priority: P2.

## Goal

Expand compatibility without extrapolating sample success to every Heroes III version.

## Scope

Choose the next available representative fixture (SoD/Complete, earlier HotA, RoE, AB or Chronicles). Implement its explicit detector/layout differences and operation capability table using H3SED as primary reference. Correct overlapping AB/SoD detection order. Include campaign/battle variants only when separately established. Add another fixture only when it exercises a materially different layout; no exhaustive matrix.

## Relevant files or components

save detect/definitions/layout-specific locators/edit gates; focused tests; compatibility section of user docs and RESEARCH.md.

## Dependencies

H3F-008, H3F-009 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [ ] Each newly enabled layout names its observed format identifiers, supported operations and remaining limitations.
- [ ] Unknown/newer versions remain read-only even if a broad family predicate matches.
- [ ] One representative copy passes a controlled edit/diff/reopen and matching-game load before write support is claimed.
- [ ] Unavailable fixture/game evidence is recorded as pending, not fabricated support.

## Minimal verification

Run only the new layout's focused tests plus the existing sample regression. Record one game-load result per newly enabled materially different layout.

