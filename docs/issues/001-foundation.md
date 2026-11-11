# H3F-001 - Create the pinned Rust 1.99.1 and Tauri 3 Alpha foundation

Status: completed. Milestone: M0. Priority: P0. Updated: 2026-11-08.

## Goal

Launch the smallest genuine Windows desktop shell with a typed Rust command round trip.

## Scope

Create the two-library Cargo workspace and desktop member described in ARCHITECTURE.md. Add rust-toolchain.toml (1.99.1), exact direct dependencies and committed lockfiles. Resolve the researched v3 candidate set; review the resolved graph for Tauri 2 contamination. Configure Wry explicitly, React/TypeScript/Vite, CSS tokens and a typed invoke wrapper. Select a compatible Rust-owned native picker dependency. Document Windows/WebView2 and build-time Node requirements. Add MIT project license and THIRD_PARTY_NOTICES scaffolding. Keep private data out of resources.

## Relevant files or components

Root Cargo.toml, Cargo.lock, rust-toolchain.toml; crates/h3forge-save/; crates/h3forge-resources/; apps/h3forge-desktop/src-tauri/; web/; README.md; LICENSE; THIRD_PARTY_NOTICES.md.

## Dependencies

None. See the [issue index](README.md) and [architecture](../ARCHITECTURE.md).

## Acceptance criteria

- [x] Windows shell launches and returns a real Rust command result; no game editing is implemented here.
- [x] Resolved packages are pinned and mutually compatible Tauri 3 Alpha. If unavailable, record the blocker rather than downgrade to v2.
- [x] Core libraries have no Tauri dependencies; frontend has no unrestricted file or shell permissions.
- [x] One documented development command and focused verification workflow work; no proprietary assets are included.

## Minimal verification

Resolve once and inspect the dependency graph/lockfiles. Run frontend typecheck/build and one Windows development launch with IPC round trip. Inspect resource configuration for accidental games/Data inclusion.

## Implementation and verification result

Completed on 2026-11-08. Implemented the three-member Cargo workspace with two dependency-free library boundaries, Rust 1.99.1 pin, explicit Tauri 3 Wry runtime, one `get_app_info` command/DTO, restricted main-window capability/CSP, and React/TypeScript/Vite shell with connection/error/retry states. Development scripts, original minimal icon, MIT license, notices and README are present. No save parsing, archive access, editing or picker command was added. Private-data protections remain intact; no game data is bundled.

Exact direct pins: tauri and tauri-runtime-wry 3.0.0-alpha.5; tauri-runtime and tauri-build 3.0.0-alpha.4; JS API 3.0.0-alpha.3; CLI 3.0.0-alpha.4; serde 1.0.229; rfd 0.15.4; React/React DOM 19.2.0; React/React DOM types 19.2.2; TypeScript 5.9.3; Vite 7.2.2; React Vite plugin 5.1.0. Cargo.lock and web/package-lock.json are generated and retained.

Verification evidence:

- Agent checks passed: Rust 1.99.1 selection, Rust formatting, workspace metadata/default members, configuration paths, restricted capabilities, empty bundle resources, ignore rules and whitespace checks.
- The resolved Cargo lockfile was inspected: runtime is alpha.4; no Tauri 2 packages are present.
- User-run frontend production build passed: TypeScript check and Vite 7.2.2, 34 modules transformed.
- User-run `cargo check --workspace --locked` passed after the runtime pin correction (3.16 seconds).
- User-run `npm.cmd --prefix D:/Projects/h3forge/web run desktop:dev` compiled and launched `target/debug/H3Forge.exe` (8.17-second native build). The user confirmed the native screen displays **Connected to H3Forge**. This screen sets connected status only after React's `get_app_info` invocation resolves with the Rust response.

The only blocking code correction was explicitly constraining tauri-runtime to alpha.4: upstream `~alpha.4` ranges had admitted alpha.5, which removed dispatcher methods still required by tauri/Wry alpha.5. Registry restrictions were resolved by the user's external installation; no registry source or global proxy settings were modified. The CLI's workspace-inheritance messages are informational, not build failures.

No remaining M0 acceptance blockers. Installer/release packaging and game features remain outside this issue. H3F-002 is next and has not been started. No builds were repeated merely to record completion.

## IDE launch follow-up (2026-11-11)

Status: configuration fix complete; native F5 retry pending. The reported CodeLLDB launch command began with a stray Cyrillic `с` before PowerShell's `&` invocation operator and failed parsing before the application started. Its origin is not established. Both application launch profiles now set `terminal: "console"`, bypassing the integrated-shell launcher and routing stdout/stderr to Debug Console. H3Forge's GUI needs no terminal stdin. The option was checked against the installed CodeLLDB 1.12.3 schema; launch configuration parsing and whitespace checks passed. No application rebuild is needed for this configuration change.
