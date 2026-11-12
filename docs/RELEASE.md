# Portable Windows releases

## Maintainer commands

From a clean checkout on a branch, with Git, Rust 1.99.1 and Node 24.13.0 installed:

```powershell
.\scripts\release.ps1 --push        # Bump patch, prepare release, confirm push
.\scripts\release.ps1 0.2.0 --push  # Explicit version
.\scripts\release.ps1 minor --push  # Bump minor (major and patch also supported)
.\scripts\release.ps1              # Bump patch; keep commit/tag local
.\scripts\release.ps1 --dry-run    # Preview without changing files or Git
```

The version argument defaults to `patch`; `minor` and `major` reset lower
components to zero. Explicit stable `major.minor.patch` versions must be greater
than the current version (each component at most 65535). Options may precede or
follow the version; PowerShell-style `-Push` and `-DryRun` also work.

The script checks the clean tree and local tag collision, plus origin and remote
tag collision when `--push` is supplied. It confirms before updating metadata,
then updates Cargo workspace/Tauri/npm versions and both lockfiles and creates a
release commit and annotated tag. Without `--push`, it stops there and prints the
manual push command. With `--push`, it additionally asks for the exact text
`push v0.2.0` (using the selected version). Enter cancels the push, retaining the
local commit/tag. No confirmation bypass is provided. It does not build.
A confirmed atomic push sends only that branch and tag to origin. Review
all outgoing branch commits first: pushing the tag publishes a GitHub Release.
Failures stop immediately and leave changes for inspection; no reset, deletion,
force push or automatic retry occurs. A rejected push can be retried manually
using the printed command after resolving its cause.

If Windows PowerShell blocks scripts under its execution policy, use a per-process
invocation without changing the machine policy:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\release.ps1 --push
```

The focused CLI regression runs with
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/tests/release.test.ps1`.
It uses ignored fixtures and mocks every Git call, including commit/tag/push.

The release workflow checks metadata against the tag, installs pinned tools and
locked dependencies, runs the Tauri 3 release build (including the Vite hook), and
uploads exactly `H3Forge.exe` and `H3Forge.exe.sha256`. No MSI/NSIS/ZIP or directory
upload is configured. Only `contents: write` is granted to the release job.
The release title uses the pushed tag; the body comes from
[RELEASE_NOTES.md](RELEASE_NOTES.md). Update that file before preparing a version.

The workflow is `.github/workflows/portable-release.yml`. It supports pushed
`v*` tags and manual dispatch, with a guard that skips branch-based runs.
After this workflow exists on both main and a release tag, a manual run is:

```powershell
gh workflow run portable-release.yml --ref v0.1.4 --repo WiverNz/h3forge
```

Use the actual release tag. Older tags that lack this workflow cannot use this
command. Manual dispatch builds and publishes, so use it only for an unpublished
release. The workflow does not overwrite an existing GitHub Release.

Build locally without creating any Git objects or publishing:

In VS Code, run **Tasks: Run Build Task** and select
`npm: desktop:build (release)`. It runs the shared portable build script followed
by artifact verification, just like CI. Install locked frontend dependencies
with the `npm ci` command below before the first build or after lockfile changes.

```powershell
npm.cmd --prefix web ci --cache .local/npm-cache
.\scripts\build-portable.ps1
node scripts/verify-portable.cjs
Get-FileHash .local/release/H3Forge.exe -Algorithm SHA256
```

Native output: `target/x86_64-pc-windows-msvc/release/H3Forge.exe`.
Publishable copies: `.local/release/H3Forge.exe` and
`.local/release/H3Forge.exe.sha256`. These folders are ignored. The build uses a
static MSVC CRT, strips symbols and remaps source paths. The fixture guard finds
the checkout relative to the running EXE, protecting local release builds too
without embedding a development path. Ordinary source-change, backup and
safe-replacement checks remain active.

## Running and recovery

Install the Microsoft Edge WebView2 Evergreen Runtime before launching. The EXE
does not download or bundle it; missing-runtime launch failure requires installing
the runtime and retrying. Windows 10/11 x64 are intended targets, not a claim that
both have been clean-machine tested. No development tools or loose web files are
needed. Copy beside the game executable for automatic discovery; elsewhere use
the installation picker. Game data is read locally and is never distributed.

Use a copy of a save and prefer Save As initially. Direct Save uses source-change
detection, a backup, verified temporary output and safe replacement. Keep the
backup until the edited file loads in the matching game. To restore, close the
game/editor, keep a copy of the edited save, then copy the matching backup to the
original save name. Do not replace your only good copy. Unsupported artwork uses
placeholders; it does not establish save-edit compatibility.

The binary embeds `LICENSES.txt` containing the project license and all retained
adapted-code notices from `licenses/`; HTML identifies it with `rel="license"`.
The complete third-party dependency license inventory remains a final release
gate, as recorded in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

## Manual release checklist

- Review final release gates in [H3F-017](issues/017-windows-release.md), including
  H3F-016 and H3F-024, and update release notes honestly.
- On clean Windows 10 and 11 x64, test the EXE alone with WebView2, without a
  development server, Node, Python, Rust, mmarch or loose frontend files.
- Check native icon/taskbar, four languages, relaunch persistence, placement
  beside a disposable game installation and manual selection from elsewhere.
- On copied saves, edit/save/reopen, verify backups and load in the matching game.
- Check missing WebView2 behavior on a disposable machine. Do not uninstall it
  from the development machine just for this test.
- Compare SHA-256, inspect PE imports/resources, and confirm only the two named
  assets are published. Never include games, Data, Maps, saves, extracted graphics,
  screenshots, debug symbols, private configuration or development paths.
