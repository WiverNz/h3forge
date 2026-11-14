# H3F-017 - Prepare Windows packaging and an evidence-backed release

Release sequencing: [H3F-024](024-internationalization.md) must meet its four-language interface acceptance before final Windows release. Packaging preparation can proceed independently.

Status: portable release automation implemented and locally artifact-verified; final release acceptance remains open for manual checks below. Milestone: M5. Priority: P1.

## Portable release implementation (2026-11-11)

- [release.ps1](../../scripts/release.ps1) supports Windows PowerShell 5.1,
  defaults to a patch bump and accepts patch/minor/major or an explicit version,
  validates stable versions, clean trees and tag collisions, updates workspace,
  Tauri and npm metadata plus lockfiles without a local build, then creates a
  commit/annotated tag after preparation confirmation. As in FlightTrace, `--push`
  opts into pushing; otherwise everything remains local. An exact `push vX.Y.Z`
  confirmation precedes the atomic origin push. `--dry-run` previews without
  mutations. No commit, tag, push or release was performed during implementation.
- [portable-release.yml](../../.github/workflows/portable-release.yml) runs for pushed `v*`
  tags on `windows-latest`, pins Rust 1.99.1 and Node 24.13.0, uses npm ci and
  locked Cargo, checks versions against the tag, builds via the installed Tauri 3
  CLI with `--no-bundle`, and publishes only the EXE and SHA-256 checksum.
- [build-portable.ps1](../../scripts/build-portable.ps1) produces the same local
  build. It remaps checkout, Cargo and Rust sysroot paths, strips symbols and
  selects static CRT linkage. The fixture guard locates a checkout relative to
  the running EXE, retaining protection in local release builds without embedding
  a development path; normal safe-save checks remain active.
- [verify-portable.cjs](../../scripts/verify-portable.cjs) checks the exact five
  frontend asset inputs, their Brotli payloads in the EXE, all six ICO image
  resources, x64 PE architecture, checksum, runtime DLL imports and common
  absolute development paths. CI runs this before publishing. No game assets,
  archives, fixtures, screenshots or private configuration are packaging inputs.
- Project/adapted-code MIT texts are embedded as `LICENSES.txt`. A full transitive
  dependency license inventory remains a public-release gate. User prerequisites,
  backup recovery, commands and limitations are in [RELEASE.md](../RELEASE.md)
  and the workflow's [release notes](../RELEASE_NOTES.md).

Verification: PowerShell 5.1 AST parsing, current metadata, valid/invalid version
checks and dirty-tree rejection passed; workflow YAML trigger/toolchain/permission
checks and PowerShell step parsing passed. A version-bump test in an ignored
fixture, with all Git calls mocked and a hard stop before staging, updated all
metadata to 0.2.0 and preserved every external Cargo lock entry. TypeScript, Vite and the native Windows
release build passed. The initial sandbox blocked esbuild; the approved retry
built successfully. Artifact inspection found Rust sysroot paths, so one corrective
rebuild added sysroot remapping. A final incremental build retained fixture
protection through runtime checkout discovery; artifact inspection and its
focused read-only guard test passed. The changed Rust file passes rustfmt;
workspace `cargo fmt --all -- --check` reports pre-existing differences in
unrelated files (including map_link.rs, town_flow.rs, domain.rs and towns.rs),
which were left untouched. No unrelated test suites were run.
GitHub-hosted execution/publishing has not been exercised.

CLI parity follow-up: [release.test.ps1](../../scripts/tests/release.test.ps1)
passes under Windows PowerShell 5.1, covering default patch, explicit version,
bump kinds, both option spellings, local-only operation, confirmed/declined push,
preparation cancellation, dry runs, invalid arguments and dirty-tree rejection.
All Git calls are mocked; real metadata updates occur only in ignored fixtures.
No application rebuild was needed for this script/documentation-only follow-up.

VS Code task follow-up: `npm: desktop:build (release)` now invokes the shared
portable build and artifact verification used by CI. The release launch
configuration points to `.local/release/H3Forge.exe`. Task/launch JSONC parsing,
PowerShell command parsing and diff checks passed; no application rebuild was
needed for this configuration-only change. Final release gates remain open.

Current-version artifact (0.1.0, no version bump performed):
`target/x86_64-pc-windows-msvc/release/H3Forge.exe`, copied to
`.local/release/H3Forge.exe` with `.local/release/H3Forge.exe.sha256`.
Size: 10,489,344 bytes. SHA-256:
`9996fc3013960b0669ae1a1bb349de7fe6980b38e65b6c14874602bf196167ce`.
Only Windows system/UCRT DLL imports were found; no separate MSVC redistributable,
WebView2Loader DLL, Python or Node DLL is imported. The WebView2 runtime remains
required. Native window/icon appearance, both placement modes, clean Windows
10/11 and missing-WebView2 behavior remain manual checks. The later
[user-reported HotA results](../MANUAL_COMPATIBILITY.md) verify specific hero/tent
operations, but do not identify or validate this release artifact.

## Application icon follow-up (2026-11-11)

The root `icon.png` is the application branding source. Following FlightTrace's
configuration pattern, Tauri now registers `icons/icon.png` and `icons/icon.ico`
under `apps/h3forge-desktop/src-tauri/`. These assets replace the placeholder icon
and supply the native window and Windows executable icon. Bundling remains disabled;
this follow-up does not complete the release acceptance criteria below.

Generate replacements from the repository root with the installed Tauri 3 CLI:

```powershell
& web/node_modules/.bin/tauri.cmd icon icon.png --output .local/app-icons
Copy-Item .local/app-icons/icon.png apps/h3forge-desktop/src-tauri/icons/icon.png
Copy-Item .local/app-icons/icon.ico apps/h3forge-desktop/src-tauri/icons/icon.ico
```

Only the PNG and Windows ICO are retained; other generated platform assets stay
in ignored `.local/`. FlightTrace was inspected read-only. Its Tauri 2 dependency
and single-size conversion script were not imported.

Verification: `npm.cmd run desktop:build:debug --cache ../.local/npm-cache`
from `web/` passed TypeScript checking, the Vite production build, and the native
Tauri build, producing `target/debug/H3Forge.exe`. The first sandboxed attempt
blocked esbuild process creation (`EPERM`); the approved retry passed. Native
window/taskbar appearance has not been visually checked.

## Workflow registration recovery (2026-11-12)

GitHub reported workflow 414703120 as `deleted` despite the file being present
on main and v0.1.3. The v0.1.3 SSH push succeeded; Actions was enabled, but no
run was created. Enabling the deleted workflow returned HTTP 403. The reason
for GitHub's deleted state remains unknown.

Renamed the workflow to `portable-release.yml` to request fresh registration
on the next push. Added `workflow_dispatch` with a tag-only job guard. Manual
runs require the new workflow at the selected tag and on the default branch.
Existing tags remain unchanged. Remote registration and execution remain
pending the maintainer's push; no release was published by this recovery.

## Goal

Produce reviewable Windows release artifacts and concise recovery/compatibility documentation.

## Scope

Configure a single portable H3Forge.exe, app-owned branding, documented WebView2 prerequisite and embedded compiled frontend. Installers are explicitly excluded by the assigned scope. Include license notices and exact support limitations. Add user instructions for installation selection, backups, restoring saves and unsupported HotA resources. Package only application-owned assets. Advanced M4 features may remain disabled; report their actual status. Preparing artifacts does not authorize publishing, pushing or signing with unavailable credentials.

## Relevant files or components

Tauri bundle/config; release scripts/workflow as needed; README/user docs; THIRD_PARTY_NOTICES.md; licenses/.

## Dependencies

H3F-006, H3F-009, H3F-016 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md). Implementation paths are planned until created.

## Acceptance criteria

- [ ] Clean Windows 10/11 smoke demonstrates launch and copied-save edit/save/reopen with no Python/Delphi/Node/mmarch runtime.
- [ ] Portable placement beside a game and installation elsewhere both work; missing WebView2 is handled/documented.
- [x] Artifact inspection finds no games/Data content, decoded proprietary graphics, private screenshots, test outputs or absolute developer paths.
- [x] Release notes separate verified supported edits from readable/unsupported layouts; notices accompany adapted code.

Automation is complete for this request. The two unchecked clean-machine/placement
criteria, H3F-016 review, H3F-024 native/translation review and transitive license
inventory remain final public-release gates. Do not infer game compatibility from
the build or sample parser round trips.

## Minimal verification

Build the release once, inspect package contents, and execute one clean-machine Windows smoke. Record any untested Windows version rather than imply coverage. Do not publish or push as part of this issue unless separately requested.

