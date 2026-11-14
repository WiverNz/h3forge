# H3Forge product requirements

Status: requirements with implemented features and remaining scope; updated 2026-11-14. Hero armies, visited tent states, artifact addition, secondary skill replacement/addition, mana, known spells and all four primary attributes have user-reported in-game HotA verification. See the [manual compatibility checklist](MANUAL_COMPATIBILITY.md) for exact scope and untested operations. Technical evidence is in [RESEARCH.md](RESEARCH.md); delivery order is in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md).

## Purpose and users

Make editing a personal Heroes of Might and Magic III / Horn of the Abyss save understandable, visual, and recoverable on Windows 10/11. Primary users are players adjusting a campaign or cooperative game and scenario authors checking hero setups. Users should not need binary-format knowledge or Python tools.

The first significant outcome is opening a **copy** of `SaAndVla.GM2`, showing its actual heroes, changing a learned spell, saving a new valid file, reopening it, and successfully loading it in the matching game. Reopening in H3Forge alone does not establish game compatibility. The user has now confirmed known-spell editing in HotA; the manual report does not identify its save filename, exact game version or specific spell, so it is not attributed to this historical sample.

## Product scope

| Capability | First delivery | Requirement |
|---|---|---|
| Save loading | M1 | Native picker; content-based format detection; map and compatibility summary; understandable errors |
| Hero overview | M1 | Actual parsed names, ownership/status where established, level, attributes, spells and armies; searchable list |
| Game graphics | M2 | Discover installation, load verified portraits and icon frames on demand, retain useful placeholders |
| Attributes and spells | M3 | Validated changes, learned versus equipment-granted spells, clear dirty state, discard/reset |
| Safe persistence | M3 | Save As and guarded overwrite, automatic backups, changed-source detection, reopen verification |
| Armies | Early M4 | Seven slots, version-aware creature choices, validated counts, original creature images when available |
| Secondary skills | M4 | Add/remove/reorder skills and mastery within supported layout constraints |
| Artifacts | M4 | Inventory first, equipment later; slot restrictions, scrolls and coupled bonuses |
| Towns | M4 | Read-only overview first, proven garrison edits only; garrison-hero ownership handled explicitly |
| Save browser and polish | M5 | Discover saves under selected installation, recent files, drag-and-drop, filters, keyboard and DPI support |

**MVP release scope:** M0–M3 plus issue H3F-009 (hero armies). The M3 spell-edit workflow is the earlier product checkpoint. A polished public package follows M5; the intervening advanced editors are incremental, not prerequisites for proving save safety.

## User experience

Use a modern desktop layout: save/installation navigation on the left, a searchable portrait list, a spacious selected-hero panel, and a persistent save/compatibility bar. Hero sections cover overview, spellbook, army, skills, and artifacts as they become supported. Use a dark neutral theme with restrained gold accents, readable typography, consistent spacing, and original artwork as focal points. Avoid reproducing the old game's entire interface or placing text over busy artwork.

Show spell icons in a searchable grid with text labels and school filters. Distinguish learned, artifact-granted, unavailable, and unsupported spells without relying on color alone. Army slots show image, creature name, and count. Keyboard focus, numeric input validation, visible loading/error/empty states, and 100–200% Windows scaling are required. Labels remain useful if every image is missing.

Changes are in memory until Save. Show per-field validation and a concise change summary; support resetting a field/hero and discarding all changes. Opening another save or closing with unsaved changes offers Save, Discard, and Cancel. Failed saves retain the edits and provide a recovery path. Do not show a success toast until persistence and reread checks complete.

## Compatibility policy

- Initial write target is the sample's HotA layout `44/11`, enabled only after operation-specific validation. H3SED heuristic recognition is evidence for investigation, not a blanket support promise.
- Other HotA minors, Restoration of Erathia, Armageddon's Blade, Shadow of Death/Complete, and Heroes Chronicles are planned individually. Recognized but unvalidated files remain read-only. Unknown formats produce an explicit unsupported result.
- Display raw format identifiers alongside the inferred game family. Do not infer an exact retail HotA version from a filename or claim that minor 11 means a particular release without evidence.
- Unsupported fields stay visible when safely readable, with a reason for disabled editing. Unknown IDs are displayed as unknown, never coerced into a known creature/artifact.
- Town buildings, ownership changes, economy, quests, map geometry, and arbitrary binary editing are outside the initial scope. Town garrisons with an occupying hero are edited through the hero, not independently.

## Assets and distribution

H3Forge loads graphics from the user's installation; it bundles no Heroes III/HotA artwork. Putting `H3Forge.exe` beside `Heroes3.exe` or `HotA.exe` should locate `Data/` and `Games/`. A folder picker supports installation elsewhere. The chosen installation and missing/incompatible archives are visible in settings.

Development can use this repository's `Data/` explicitly, without including it in application resources. Startup indexes readable archives, rather than extracting them. Decoding and caches belong to Rust. Missing, unsupported, or incorrectly mapped graphics use generic original placeholders. Standard LOD support must not be presented as support for the supplied nonstandard `HotA.lod` directory.

Runtime is native Rust plus the Windows webview; frontend tooling is build-time only. Offline editing is supported. No account, telemetry, server, game executable modification, archive writing, cloud saves, multiplayer synchronization, auto-update service, or general mod manager is planned.

## Acceptance criteria

1. A copied sample shows map `- The Mysterious Island - COOP`, raw format `44/11`, and reproducible hero records with renamed/Cyrillic names preserved. The research baseline is 215 locator candidates, not 215 proven active heroes.
2. A supported hero with a spellbook can gain/lose an allowed learned spell; affected availability bytes obey equipment rules. Reload shows the change and the matching game accepts the saved copy.
3. Decompressed output differs only within reviewed patch ranges. A no-change Save does not recompress or rewrite the file. Unknown fields and unedited blank encodings remain intact.
4. Backups preserve original compressed bytes. Permission errors, failed validation, locked destinations, and changed source files cannot silently discard edits or destroy the only original.
5. Hero attributes and army counts reject invalid input in Rust even when UI validation is bypassed. Operation restrictions are per format and record.
6. At least one verified portrait mapping and spell icon mapping load from local archives; unresolved heroes use placeholders rather than guessed portraits. Assets absent or unreadable never prevent text-based editing.
7. The release contains no reference saves, original archives, extracted art, or absolute development paths. A clean Windows machine needs no Python, Delphi, Node runtime, or mmarch executable.

## Deferred features

Expand exact-version coverage using a few representative user-provided fixtures, support more HotA resource variants, improve portrait/custom-hero identity, and add advanced town fields only when their representation and coupling are understood. Animation, batch edits, save comparison and undo history beyond reset/discard remain optional later work. Four-language UI localization is implemented (H3F-024), with native visual/relaunch and human translation review still pending.
