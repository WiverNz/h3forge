# Implementation issue index

All 27 issues are local artifacts, ready to transfer to GitHub later. None has been published. H3F-001 through H3F-005 and H3F-007/008/009 are completed (foundation; read-only save container + hero record parsing; read-only hero overview UI, sample-verified; bounded read-only LOD + PCX library, real-archive-verified; installation discovery + evidence-verified hero portraits; validated hero attribute/spell edit operations; Save As workflow with backups and reopen verification - sample-verified, in-game load pending maintainer confirmation); H3F-009 adds exact-44/11 hero army editing with a safe sample round trip (in-game verification outstanding); H3F-020 completes the requested frontend usability follow-up; H3F-006 (DEF decode, creature portraits, verified spell icons) and H3F-010 (secondary skill/mastery/order editing, sample-verified) are completed; H3F-011 is completed (conservative backpack artifact/scroll editing, sample round-trip verified; in-game loading pending); H3F-014 now supports the local opaque-ID stored/zlib archive layout and has all six Interference/Runes skill icons identified, verified, and served from HotA.lod (LZMA remains unsupported; section B of GAME_RESOURCES_FORMAT.md); other remaining issues are planned. IDs are stable; numbered order is the normal execution sequence. H3F-021 is completed (map discovery, bounded H3M reader, save-to-map hero enrichment; all 84 formerly missing sample hero portraits resolve through the portrait name tables - 42 exactly, 42 in ID range 187–244 by an inferred, not conclusively verified, file assignment). **H3F-009 is complete. H3F-008 also supports direct Save and permanent sample regressions; its manual game-load gate remains open. H3F-006 is complete. H3F-011 is complete. H3F-012 is complete (wear-slot equip/unequip with verified attribute-bonus and granted-spell coupling; combination/scroll/unknown occupants and reservation bytes block the section; sample round-trip verified, in-game load pending). H3F-012 visual-artifact follow-up is complete: compact equipment/backpack grids, searchable icon picker, 163 verified local icons and exact safety diagnostics; global equipment gates retained. H3F-013 is complete (bounded town discovery with low-confidence candidates, read-only town view, garrison slot editing for verified 44/11 towns without a garrisoned hero; sample round-trip verified, native window and in-game load pending). H3F-022 is partially complete (original town icons and Town Hall building pictures for all twelve town types; actual building names for every faction; building masks and Mage Guild state parsed for 44/11; Mage Guild spell editing and a verified subset of building editing in all sample towns including the Shipyard and most horde buildings, sample round-trip verified; a garrisoned hero's army is shown and edited as the town garrison; Fort, Citadel, Castle, Capitol, grail and structures with their own saved state stay read-only; in-game load pending). H3F-018 is complete (eight player slots parsed, four active sample players editable, all seven resources and original icons; copy-based round trips and session persistence verified; native UI and game-load gates remain manual). Further work requires a new request.**

Priorities: P0 proves/completes the MVP; P1 expands/polishes the supported product; P2 is incremental compatibility work. Dependencies are requirements, not a demand to wait for unrelated earlier numbers. Every issue inherits the source-data protections in [AGENTS.md](../../AGENTS.md). Functional savegame editing (M3) now precedes the optional artwork issue H3F-006, which moved to P1.

| ID | Issue | Milestone | Priority | Depends on |
|---|---|---|---|---|
| H3F-001 | [Create the pinned Rust 1.99.1 and Tauri 3 Alpha foundation](001-foundation.md) | M0 | P0 | - |
| H3F-002 | [Read and identify save containers safely](002-save-container.md) | M1 | P0 | 001 |
| H3F-003 | [Display actual heroes in a read-only save overview](003-hero-overview.md) | M1 | P0 | 002 |
| H3F-004 | [Adapt mmarch Rust into a bounded read-only LOD and PCX library](004-lod-pcx-library.md) | M2 | P0 | 001 |
| H3F-005 | [Discover the installation and display verified hero portraits](005-installation-portraits.md) | M2 | P0 | 003, 004 |
| H3F-006 | [Decode required DEF frames and show original spell icons](006-def-icons.md) | M2 | P1 | 004, 005 |
| H3F-007 | [Plan validated attribute and spell patches for HotA 44/11](007-hero-edit-operations.md) | M3 | P0 | 003 |
| H3F-008 | [Complete the sample spell-edit workflow with backups and safe saving](008-safe-save-workflow.md) | M3 | P0 | 007 |
| H3F-009 | [Edit hero armies and complete the MVP](009-hero-armies.md) | M4 | P0 | 008 |
| H3F-010 | [Edit secondary skills and mastery with version-aware ordering](010-secondary-skills.md) | M4 | P1 | 008 |
| H3F-011 | [Edit backpack artifacts and scrolls](011-artifact-inventory.md) | M4 | P1 | 008 |
| H3F-012 | [Edit validated equipment slots and coupled bonuses](012-equipment.md) | M4 | P1 | 007, 011 |
| H3F-013 | [Inspect towns and enable proven garrison edits](013-towns.md) | M4 | P1 | 003, 009 |
| H3F-014 | [Resolve or explicitly classify the supplied HotA LOD variant](014-hota-resource-variant.md) | M4 | P2 | 004, 005 |
| H3F-015 | [Add evidence-backed support for further save layouts](015-version-coverage.md) | M4 | P2 | 008, 009 |
| H3F-016 | [Polish save browsing and the desktop editing experience](016-desktop-ux.md) | M5 | P1 | 005, 008, 009 |
| H3F-017 | [Portable Windows EXE and tag-release CI (implemented; manual release gates open)](017-windows-release.md) | M5 | P1 | 009, 016; 024 before final release |
| H3F-018 | [Edit player resources and gold (complete)](018-player-resources.md) | M4 | P1 | 008 |
| H3F-019 | [Hero and Town Coordinates (constrained relocation implemented)](019-map-coordinates.md) | M4 | P1 | 008, 013, 021 |

| H3F-020 | [Make the existing desktop editor practical and compact](020-desktop-ui-usability.md) | UI follow-up | P0 | 005, 008 |
| H3F-021 | [Map discovery and H3M integration](021-map-discovery.md) | M4 | P1 | 003, 005 |
| H3F-022 | [Town buildings, Mage Guild and artwork](022-town-buildings-mage-guild.md) | M4 | P1 | 006, 013, 014 |
| H3F-023 | [Player details, alliances and map revelation (partial)](023-player-details-map-reveal.md) | M4 | P1 | 018, 021 |
| H3F-024 | [Multi-Language Support (i18n)](024-internationalization.md) | M5 | P1 | 009 |

H3F-024 is implemented with automated verification: English, Russian, German and Spanish catalogs, Settings, native language detection, persistent choice, typed frontend error translation and localized native picker labels. Native visual/relaunch smoke and human translation review remain pending before H3F-017's final release; original technical details/game catalogs are not claimed as fully translated. It remains independent of H3F-019 and other unfinished features. Save-name decoding and binary values are unchanged.

The earliest significant product checkpoint is H3F-008: actual sample hero → learned-spell edit → backup and safe save → parser reload → matching-game load. H3F-009 now supplies the army editor for the implemented MVP scope; matching-game verification remains outstanding. Resource work H3F-004–006 should improve that checkpoint but unresolved HotA artwork must not stall the save path.

H3F-005 has an explicit portrait-mapping evidence gate; placeholders are acceptable for unresolved individual heroes, not evidence that every portrait is mapped. H3F-014 can end with a documented unsupported resource variant; this does not count as implemented variant support. H3F-013/015 must leave unsafe or untested writes pending. H3F-018/019 gate every new field behind sample verification; coordinate writes stay closed until linked map structures are proven. M5 packaging may proceed for the supported MVP without pretending every optional M4 feature is finished.

Use [IMPLEMENTATION_PLAN.md](../IMPLEMENTATION_PLAN.md) for milestone definitions and [RESEARCH.md](../RESEARCH.md) for the inspection baseline. The self-contained binary-format references live in [SAVEGAME_FORMAT.md](../formats/SAVEGAME_FORMAT.md), [GAME_RESOURCES_FORMAT.md](../formats/GAME_RESOURCES_FORMAT.md) and [MAP_FORMAT.md](../formats/MAP_FORMAT.md). After an issue, record the focused verification result and any changed compatibility claim; do not rerun entire upstream suites.

H3F-018 is implemented and sample-verified. H3F-023 is partially complete: compact Resources/General/Exploration tabs, conservative current status, verified-slot resource editing, player details/artwork and read-only obelisk visits are implemented; both revelation actions remain disabled with documented structural blockers. H3F-019 implements arbitrary-distance hero relocation on the same level/medium to validated matching passive tiles, with compact inputs and pending/Revert/Discard/Save integration. Movement points, facing and exploration are preserved by editor policy. Towns remain read-only. Cross-level moves, boarding/landing and object interactions stay disabled; native game-load validation remains manual. The repeated plain-land pair request was withdrawn after deeper analysis of the existing saves.

H3F-025 implements FlightTrace-style light/dark/system themes in Settings with persistence and four-language labels. Production build, 25 frontend tests and localized rendering pass; native visual/relaunch smoke remains pending.

H3F-017 implements Windows PowerShell release preparation and tag-only GitHub
Actions publishing for a single portable EXE plus checksum. The
release CLI now follows FlightTrace: default patch bump, explicit versions or
patch/minor/major, opt-in `--push` with confirmation, and `--dry-run`; focused
PowerShell 5.1 tests pass with mocked Git. The local Tauri 3
release build and embedded frontend/icon/asset-boundary inspection passed. No Git
commit, tag, push or release was created in this task. Clean Windows 10/11,
placement/native UI, matching-game loading, translation review and the full
dependency license inventory remain manual final-release gates; see
[release instructions](../RELEASE.md). Installers are out of scope.

| ID | Issue | Milestone | Priority | Depends on |
|---|---|---|---|---|
| H3F-025 | [Light and dark themes in Settings](025-light-dark-themes.md) | M5 | P1 | 024 |
| H3F-026 | [Hero dismissal (verified case implemented; in-game validation open)](026-hero-dismissal.md) | M4 | P1 | 008, 018, 019 |
| H3F-027 | [Neutral Creatures and Map Garrisons (verified scope)](027-neutral-creatures.md) | M4 | P1 | 006, 008, 009, 013, 021 |

H3F-026 dismisses a player-owned hero from a HotA 44/11 save as checked same-length patches, derived from a real before/after pair of a native dismissal: hero record, hero owner table, player roster, map tile and boarded boat. Army, spell points and movement are reset as in the game in the verified cases; the replay log is not extended. A second (land hero) pair is reproduced completely apart from the replay log. Garrisoned or visiting heroes, heroes on map objects and ambiguous structures are refused with an explanation. Copy-based core and desktop regressions pass; loading the result in the game is still a manual gate.


H3F-027 adds a compact Creatures section: 291 current wandering groups with
verified current quantities and compatible-template type edits, plus 47
editable seven-slot map garrisons in SaAndVla.GM2. The supplied native
72-to-56 survivor pair establishes the monster quantity table. Initial H3M
data remains reference-only. Other guarded objects remain unsupported;
native edited-save loading and 1280x720 visual smoke remain manual checks.
See the issue for exact edit gates and focused verification.
