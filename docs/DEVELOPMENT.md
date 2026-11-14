# Developing H3Forge

Build setup, dependency pins and historical foundation verification. For downloads and everyday use, see the [main README](../README.md). For release preparation, see [RELEASE.md](RELEASE.md).

## Development requirements

- Windows 10/11 x64, Visual Studio Build Tools with Desktop development with C++, MSVC and Windows SDK.
- Rustup with Rust **1.99.1**, pinned by `rust-toolchain.toml`.
- Node **20.19+ in the 20.x line, or 22.12+**, and npm. This session used Node 24.13.0 / npm 11.21.2.
- Microsoft Edge WebView2 Runtime. Node, npm and Rust are build-time tools; the desktop app has no Python, Delphi, mmarch or Node sidecar.

## Bootstrap and run

From the repository root in PowerShell, in an environment with dependency access:

```powershell
# Install the retained dependency versions.
npm --prefix web ci
cargo fetch --locked

# Start the desktop shell and its local Vite server.
npm --prefix web run desktop:dev
```

Use `npm --prefix web ci` and Cargo `--locked` for reproducible installs/checks. Do not hand-write or fabricate dependency locks. The development server binds only to `127.0.0.1:1420`. Tauri's hook starts/stops it; the commands do not assume the shell's working directory beyond running from the repository root.

Use the native app to open a copied save and select a game installation. `npm --prefix web run dev` is a browser-only layout preview; actual save and archive access requires the native Rust commands.

## Focused verification

Automated tests establish byte preservation and parser/session behavior, not
in-game compatibility. Record actual HotA results in the
[manual compatibility checklist](MANUAL_COMPATIBILITY.md), including individual
passes, failures and missing metadata. The 2026-11-14 recorded user report covers
specific hero edits and tent states; it does not validate all implemented editors.
Documentation-only status updates need link/completeness/diff checks, not builds.


```powershell
# Rust syntax/style and core-only checks for ordinary library changes.
cargo fmt --all -- --check
cargo check --locked

# Desktop milestone: build frontend and executable once (no installer).
.\scripts\build-portable.ps1
node scripts/verify-portable.cjs
```

The desktop build invokes frontend typechecking/Vite through the Tauri hook. For frontend-only changes use `npm --prefix web run check`. For a desktop compile check use `cargo check -p h3forge-desktop --locked` after frontend assets are built. The portable script produces `.local/release/H3Forge.exe` and its SHA-256 checksum. Installers are out of scope.

When intentionally changing the dependency set, inspect `cargo tree --workspace --locked` and `npm --prefix web ls --depth=0` once. Confirm all Tauri runtime/build/plugin packages resolve to a compatible v3 graph, then run the native smoke check. Do not mark M0 complete from a browser preview or manifest review alone.

## Exact direct dependency selections

| Component | Version |
|---|---|
| Rust | 1.99.1 |
| tauri / tauri-runtime-wry | 3.0.0-alpha.5 / 3.0.0-alpha.5 |
| tauri-build | 3.0.0-alpha.4 |
| tauri-runtime (explicit compatibility constraint) | 3.0.0-alpha.4 |
| @tauri-apps/api / @tauri-apps/cli | 3.0.0-alpha.3 / 3.0.0-alpha.4 |
| serde | 1.0.229 |
| rfd (Rust-owned Windows picker dependency) | 0.15.4 |
| react / react-dom | 19.2.0 / 19.2.0 |
| @types/react / @types/react-dom | 19.2.2 / 19.2.2 |
| TypeScript | 5.9.3 |
| Vite / @vitejs/plugin-react | 7.2.2 / 5.1.0 |

The Tauri selection follows the [official alpha.5 source snapshot](https://raw.githubusercontent.com/tauri-apps/tauri/tauri-v3.0.0-alpha.5/Cargo.toml), its [API manifest](https://raw.githubusercontent.com/tauri-apps/tauri/tauri-v3.0.0-alpha.5/packages/api/package.json), and the [CLI alpha.4 release](https://github.com/tauri-apps/tauri/releases/tag/@tauri-apps%2Fcli-v3.0.0-alpha.4). Wry is selected explicitly using the v3 API. [React plugin 5.1.0](https://raw.githubusercontent.com/vitejs/vite-plugin-react/plugin-react@5.1.0/packages/plugin-react/package.json) accepts Vite 7 and the stated Node versions. [rfd 0.15.4](https://docs.rs/rfd/0.15.4/rfd/) provides native Windows dialogs without a Tauri plugin. The resolved lockfiles and Windows compile/runtime checks have now verified this set, including the explicit runtime alpha.4 constraint.

## Workspace and security

- `crates/h3forge-save`: Tauri-independent save parsing, validated patches and safe saving.
- `crates/h3forge-resources`: Tauri-independent installation discovery, archive access and image decoding.
- `crates/h3forge-map`: Tauri-independent read-only `.h3m` reader, bounded map discovery and save-to-map matching (H3F-021).
- `apps/h3forge-desktop/src-tauri`: native shell, typed commands and editing sessions.
- `web`: React frontend, typed IPC wrappers, localization and light/dark themes.

Only the local main window receives the enumerated application command permissions. There is no arbitrary frontend filesystem API or game-data resource glob. CSP permits the IPC transport; development additionally permits Vite's loopback HMR and injected styles/scripts. Release assets use the stricter policy.

`games/`, `Data/` and `Maps/` remain ignored, private and untouched. Never package them. The application uses the PNG/ICO derived from the root `icon.png` under `src-tauri/icons/`. Adapted-code notices are embedded in the portable frontend. See [third-party notices](../THIRD_PARTY_NOTICES.md).

## Verification recorded for H3F-001

Passed agent checks: Rust 1.99.1 selection, formatting, workspace metadata, configuration paths, capability scope, resource exclusions and ignore rules. The resolved Cargo lockfile contains the expected v3 alpha set and no Tauri 2 packages.

User-run checks passed on 2026-11-08: TypeScript/Vite production build; `cargo check --workspace --locked`; and the Windows development build/launch through `npm --prefix web run desktop:dev`. The user confirmed **Connected to H3Forge** in the native window, verifying React's `get_app_info` invocation and response display.

The initial registry restriction was resolved through external installation. Both lockfiles are retained. The actual compile mismatch was corrected by explicitly pinning `tauri-runtime = "=3.0.0-alpha.4"`: runtime alpha.5 removed methods still used by tauri/Wry alpha.5 despite being allowed by their upstream dependency ranges. Retain this constraint until deliberately upgrading and verifying the stack together. Tauri CLI workspace-inheritance notices are informational.

This section records historical H3F-001 verification only. Current feature and release acceptance status is maintained in the [issue index](issues/README.md).
