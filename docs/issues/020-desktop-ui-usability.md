# H3F-020 - Make the existing desktop editor practical and compact

Status: completed (implementation and automated frontend verification). Requested UI follow-up, 2026-11-09. Dependencies: H3F-005 and H3F-008. Native window/manual interaction verification remains outstanding.

## Delivered

- Permanent Save As toolbar, disabled explanation, unsaved count, Discard Changes, and persistent save result/error messages. Native commands and Rust safety checks are unchanged. Opening another file is disabled until pending edits are saved or discarded.
- Viewport-sized workspace with independent hero-list/editor scrolling, full available width, bounded numeric fields, wrapping controls, and a stacked layout below 480px. Main desktop targets are 1280×720, 1600×900 and larger; live window verification is still needed.
- Compact portrait/identity and grouped General fields. Searchable spells use the editor scrollbar; read-only available spells are collapsed.
- Army slots show Empty or Creature #ID with counts. Equipment shows Empty or Artifact not identified. Backpack occupancy is not asserted; raw equipment/backpack bytes and diagnostics are collapsed.
- Movement, secondary skills/mastery, army, artifacts and available spells remain read-only. Towns and Players are explicitly disabled/planned navigation entries, with no fabricated data.
- CP1251 decoding, hero filtering, portrait loading, edit and native save APIs remain in place. No Rust changes, new writing support, town parsing, or work on H3F-006/009.

## Evidence and boundaries

`crates/h3forge-save/src/domain.rs` documents zero and 0xFFFFFFFF army blanks; presentation preserves creature ID 0 when its count is nonzero. `locate.rs::equipment_valid` recognizes a first word of four FF bytes as a blank equipment slot regardless of the second word. The frontend only presents these existing conventions; no artifact ID decoding or inventory occupancy interpretation was added. The existing DTOs expose no creature/artifact name definitions. All original DTO bytes remain unchanged.

## Verification

- `npm.cmd --prefix web --cache .local/npm-cache test`: 6 passed, including CP1251 and focused blank-slot regressions.
- `npm.cmd --prefix web --cache .local/npm-cache run build`: TypeScript and Vite passed.
- `git diff --check`: passed.
- Initial sandbox runs failed to spawn Node/esbuild (EPERM); approved reruns succeeded. No dependencies installed and no Rust suites run.
- Native dialogs, real-save interaction, and visual/DPI checks at desktop sizes were not run in this session; no new native safety or compatibility claims are made.


## Compact list army preview follow-up (2026-11-09)

Hero rows now show the first three occupied army slots as 16×16 portraits on the existing name line, followed by +N for additional stacks. The read-only list projection preserves slot order and overlays pending army edits; apply/discard refresh the preview. Missing artwork uses compact placeholders. The Army editor and list share a frontend request cache, invalidated on installation selection, plus the existing Rust PNG cache. No extra row or vertical padding was added; long names use ellipsis with a full-name tooltip.

Verification: frontend TypeScript/Vite build, one focused stack-order/duplicate/overflow test, and `cargo check -p h3forge-desktop` passed. Initial frontend commands hit sandbox child-process EPERM; approved reruns passed. No full Rust test suites or new editing features.


Owner-color follow-up completed: subtle owner-ID stripes, selected-row tint/border and matching detail-header accent. Mapping tests and frontend build pass; see H3F-010 follow-up notes for the accompanying atomic skill replacement.

## Follow-up: dialog crash, Save button, map commands (2026-11-10)

- **Crash on Open save (verified location, design fix):** the reported `0x80000003` at `0x7ffe09675a0d` resolves to `EmbeddedBrowserWebView.dll` (WebView2 157.0.4341.42, RVA `0x5e5a0d`), not H3Forge or the save parser; the session open flow runs clean on the same saves in tests. Native pickers ran synchronously on the main thread inside the WebView2 IPC callback, so their modal message loop re-entered WebView2. `choose_save`, `choose_installation`, `save_save_as` and `choose_map_file` are now `#[tauri::command(async)]` (thread pool), parented to the main window, and never hold the state lock while a dialog is open. **Not yet re-run in the real window** - needs one manual open/Save As check.
- **Save button:** enabled whenever there are pending edits. A source that cannot be overwritten (protected project `games/`/`Data/`/`Maps/`, read-only file or folder) no longer disables it silently; clicking shows the reason and points to Save As. Backend protection is unchanged.
- **Map link commands:** `get_map_link`, `rescan_maps`, `select_map_candidate` and `choose_map_file` were registered handlers but missing from the app manifest and `main` capability, so the packaged app rejected them. Added to both.
- **Artifact icon commit review:** the `HrArt###` → artifact ID table in `artifact_icons.rs` agrees with the nearest-pixel probe for all names 000–140; only the format note was stale and is corrected.
