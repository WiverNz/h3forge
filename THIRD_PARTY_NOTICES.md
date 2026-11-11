# Third-party notices

H3Forge's new source and geometric foundation icon are MIT licensed; see [LICENSE](LICENSE).

H3Forge uses Tauri/Wry, Serde, rfd, React and frontend build dependencies. Their upstream licenses remain applicable. Dependencies are resolved in the retained lockfiles. This document is not a complete release-license inventory: before a public release, retain the notices supplied by all resolved transitive dependencies. H3F-017 embeds the project license and the four retained adapted-code MIT notices in `LICENSES.txt` within the compiled frontend.

| Project | Declared upstream license | Role |
|---|---|---|
| Tauri and its JavaScript API/CLI | MIT OR Apache-2.0 | Native application and IPC/build tooling |
| Serde | MIT OR Apache-2.0 | Command response serialization |
| Rusty File Dialogs (rfd) | MIT | Selected native dialog dependency |
| React / React DOM | MIT | Frontend |
| i18next / react-i18next | MIT | Bundled interface localization (25.6.3 / 16.3.5) |
| sys-locale | MIT OR Apache-2.0 | Windows preferred UI-language lookup (0.3.2) |
| Vite / React Vite plugin | MIT | Build tooling |
| TypeScript | Apache-2.0 | Build tooling |
| DefinitelyTyped React definitions | MIT | Build tooling |

FlightTrace was consulted for project organization. H3F-025 adapts its warm light/dark palettes and theme preference approach from `web/src/styles/tokens.css` and `web/src/theme/theme.ts` (local reference inspected 2026-11-11). Copyright (c) 2026 FlightTrace contributors, MIT; full notice in [licenses/FlightTrace-MIT.txt](licenses/FlightTrace-MIT.txt). Adaptations include H3Forge tokens, accessible light-mode text colors, localized Settings selection, storage-failure feedback and an external CSP-compatible startup script. No FlightTrace artwork was copied. No mmarch implementation code has been adopted yet. When mmarch algorithms/modules are adopted in later issues, include the full mmarch MIT notice (Tom Chen), source paths and snapshot provenance under `licenses/` and here.

H3F-004 adopted mmarch's MIT-licensed LOD and Heroes PCX handling: the H3 header/directory layout and entry record structure, per-entry bounded zlib semantics (`stop at the declared unpacked size, tolerate trailing bytes`), the 12-byte-header PCX model (indexed + 768-byte R,G,B palette, or 24-bit pixels whose triplets are B,G,R), and the impossible-unpacked-size ceiling, all transcribed from `D:/Projects/mmarch/mmarch-rust/src/lod.rs` and `image.rs` (snapshot commit `fc104f2a7d8b170ed67bc05b9bfac5d0bb9a60b8`; full notice in [licenses/mmarch-MIT.txt](licenses/mmarch-MIT.txt)) into `crates/h3forge-resources` (`lod.rs`, `pcx.rs`). Adaptations: whole-file loading replaced by header/table-only opening with per-entry seeks, short/invalid directories and duplicate names rejected instead of silently tolerated, allocation limits enforced before reads and decompression, automatic PCX-to-BMP conversion replaced by explicit structured decoding (indexed data preserved, RGBA conversion explicit), the nonstandard supplied `HotA.lod` directory rejected as unsupported instead of trusted, and the zlib bounded decompression reimplemented over the workspace's existing `miniz_oxide` core API. No writer, CLI, MM6–8 archive kinds, sprite machinery or fixtures were ported.

H3SED parsing logic was adopted in H3F-002: the gzip-integrity policy contrast, version byte-range detection order, map text header parsing, hero record locator checks (HotA 1121/1122-byte record structure, name strip, equipment grammar), hero field offsets and the experience-level table, transcribed into `crates/h3forge-save` (`container.rs`, `detect.rs`, `parse.rs`, `locate.rs`, `definitions.rs`). Source: `D:/Projects/h3sed_4.1/src/h3sed/` (supplied 4.1 tree, MIT, copyright 2020 Erki Suurjaak; full notice in [licenses/h3sed-MIT.txt](licenses/h3sed-MIT.txt)). Adaptations: native Rust with checked bounds instead of regex/byte-regex heuristics, strict gzip CRC/ISIZE validation instead of h3sed's patched permissive reader, confidence-graded records instead of silent drops, and owner-range verification added; no wx widgets, global stores or embedded artwork were ported.

H3F-003 adopted the H3SED display-identification tables (skill names including HotA Interference/Runes, skill mastery names, spell names, player faction labels, equipment slot labels) and the spellbook-presence check (non-blank word at the spellbook equipment slot) from the same source and license, transcribed into `crates/h3forge-save/src/definitions.rs` (`names` module) and `domain.rs`.

Heroes III and Horn of the Abyss names identify the games supported by the planned editor. No original game artwork, archive, savegame or extracted asset is included. The application does not claim ownership of those games or their assets.
