# H3Forge implementation plan

Status: M0 and M1 completed on 2026-11-08; M2 partially complete (H3F-004/005 done; H3F-006 deferred behind editing work at P1); M3 implemented on 2026-11-08 (H3F-007/008 - attribute/spell editing and Save As verified on a private sample copy; the matching-game load acceptance is pending maintainer confirmation); M4–M5 remain planned. Ordered tasks and priorities are in [issues/README.md](issues/README.md). Product scope is in [PRD.md](PRD.md); use [ARCHITECTURE.md](ARCHITECTURE.md) for boundaries and safety invariants.

## Delivery order

| Milestone | Deliverable | Dependencies | Definition of done |
|---|---|---|---|
| M0 - foundation | Rust 1.99.1 workspace, pinned Tauri 3 Alpha/Wry shell, React/TS/Vite, typed command round trip, notices and focused development commands | None; H3F-001 | Windows app launches, one IPC call works, lockfiles show compatible v3 packages; no private game data in build resources |
| M1 - read-only save browser | Strict gzip/content detection, sample profile, hero locator and readable/searchable overview | M0; H3F-002–003 | A copied sample opens with map/version and reproducible hero records; malformed input returns errors; no writing API exposed |
| M2 - useful original artwork | Adapted Rust LOD reader, PCX portraits, installation selection and first DEF icons | M1; H3F-004–006 | A verified portrait and spell frame display from readable local archives; missing/unsupported assets use placeholders; no startup extraction |
| M3 - first complete edit | Validated attributes/spells plus transactional persistence and full UI flow | M1; H3F-007–008; M2 enhances UI but asset gaps do not block editing | Copy of SaAndVla opens, actual hero selected, one learned spell changed, backup/output verified, unrelated payload bytes unchanged, saved copy reopens and loads in matching game |
| M4 - expanded editor | Armies first, then skills, inventory, equipment, towns, targeted resource/version coverage | M3; H3F-009–015 with dependencies in index | Each delivered operation has a narrow support gate and focused copy-based check; unsupported fields stay read-only |
| M5 - desktop release | Save discovery, drag/drop, accessible polished UX, four-language interface, Windows portable build/installer, user docs and notices | M3 + army MVP; H3F-016, H3F-024, then H3F-017 final release | Four-language UI checks and clean Windows smoke pass; no proprietary assets or development paths shipped; recovery and compatibility limitations documented |

**MVP completion** is M3 plus H3F-009 armies. Do not postpone the first spell-edit demonstration until equipment, all towns, complete HotA asset support or every H3 version is implemented. The normal issue order is numbered, but independent M2 work need not block H3F-007/008 if resource investigation stalls. M5 can package the completed MVP while optional M4 compatibility extensions remain explicitly unsupported.

## Milestone details

### M0: build the smallest working desktop foundation

Create the planned files only when implementing H3F-001. Resolve the candidate alpha set once; capture exact dependencies and Windows requirements in the developer README. Use a Rust-owned picker rather than guessing a v3 dialog-plugin pin. Establish a clear build-time/runtime distinction for Node and WebView2. Add license/provenance scaffolding and a synthetic command response, not a fake game editor.

Verification: one Windows development launch and command round trip, frontend typecheck/build, and package graph review. Do not install/build upstream applications.

### M1: show the real sample before edits

Implement bounded gzip and header parsing first, then port H3SED's locator checks into checked Rust logic. Keep exact raw format identifiers and diagnostics. Decode names without modifying bytes; use CP1251 as a selectable display encoding for the supplied Cyrillic sample. Distinguish owned, inactive, neutral and uncertain records as far as the data proves them. Do not advertise all locator candidates as active heroes.

Verification: synthetic truncation/CRC/unknown-header cases, one opt-in private sample baseline, and one UI smoke. No fixture output goes to original folders.

### M2: add enough artwork for a compelling editor

Adapt mmarch's relevant code with notices and a narrow reader API. Index only standard valid directories. Render PCX to PNG in Rust, establish portrait identity evidence, then add the required DEF icon frames. Validate mappings instead of treating hero list position or frame number as a universal game ID. The supplied HotA directory variant is a separate bounded issue; keep base assets usable.

Verification: generated archive fixture with one compressed and one raw entry, one real portrait, one spell icon on checkerboard, and missing/corrupt archive fallback. No full asset collection or animation system.

### M3: prove one safe editing workflow

Introduce semantic edit operations and a patch planner with source preconditions. Target the sample's exact `44/11` layout. Start with an owned hero with a verified spellbook, no unresolved artifact interaction and a spell whose availability is understood. Implement Save As, backups, source conflict detection, temporary-file verification and Windows replacement together with the UI save flow. Then enable conservative primary attributes and spell points; level/experience need explicit synchronization behavior.

Verification: one spell toggle with expected learned/available ranges, a no-op, invalid value, failed backup/locked target, changed source, and successful copied-file round trip. Perform one in-game load in the matching installation and record the result. If a record or operation cannot be established safely, leave it read-only and document why; M3 is not complete without its sample spell checkpoint.

Status: implemented 2026-11-08 (H3F-007/008). The copied-file round trip, byte-diff limits, failure paths and reopen verification pass against the real sample; scope widened slightly from "one spell toggle" to primary attributes, experience/level, spell points and learned spells behind the same patch pipeline. Only the in-game load checkpoint remains, explicitly pending the maintainer's manual test.

### M4: expand by feature, not by abstraction

H3F-009 armies completes MVP. Skills, backpack and equipment follow in increasing coupling complexity. Town inspection precedes garrison editing; block direct garrison edits when a hero owns the army. Do not add town economy/buildings without a later field-specific task. H3F-018 adds player resources behind sample-verified offsets, H3F-019 adds read-only coordinates and keeps map-position edits closed until linked structures are proven. H3F-014 investigates nonstandard HotA resources with a clear stopping point. H3F-015 widens support only for available representative fixtures; no demand for a giant version matrix.

Verification: targeted field/byte-diff checks per issue and one milestone smoke of the combined editor. Only add another game-load check when a new layout or coupled operation warrants it.

### M5: make the supported features comfortable to ship

H3F-024 implementation and automated checks are complete as of 2026-11-11. The native four-language visual/relaunch smoke and human translation review remain final-release gates; technical details and game catalogs retain their documented original-language fallback. See the issue for exact coverage and the computer-use launch restriction.

Implement [H3F-024 - Multi-Language Support](issues/024-internationalization.md) after the completed H3F-009 core editing baseline and before H3F-017's final Windows release. It is P1 and independent of H3F-019 and other unfinished features, including H3F-016. Localize the delivered interface in English, Russian, German and Spanish, with a persistent Settings choice, English fallback and structured frontend error translation. The issue owns the migration details; game-data decoding and binary values remain unchanged. Packaging preparation can continue independently, but final release includes H3F-024 acceptance.

Add save browsing, native drag/drop grants, recent files, search/filter refinements, accessible controls, unsaved-close handling and sensible scaling. Ship a portable executable using the installed WebView2 runtime and an installer that handles runtime prerequisites. Test beside a game directory and installed elsewhere; never embed development `Data/` via a bundler resource glob.

Verification: one release artifact inspection and clean Windows launch/open/edit/save/reopen smoke. Release notes list exact tested layouts, known gaps, third-party notices and backup recovery. Publishing/signing credentials are separate release operations, not part of this planning authorization.

## Rules for implementation sessions

- Implement one ordered issue or a coherent vertical slice; keep documents updated when evidence changes.
- Use local private copies for real fixtures and small synthetic files for CI. Never normalize or repair the supplied originals.
- Run focused checks for changed code, then one appropriate milestone check. No repeated full builds after trivial changes.
- Keep unresolved research bounded: record evidence, disable unsupported behavior and continue independent work. Do not invent offsets, asset mappings or dependency compatibility to close a task.
- Do not create more crates, frameworks, issue subdivisions or compatibility promises without a demonstrated need.
