# H3F-005 - Discover the installation and display verified hero portraits

Status: completed (2026-11-08). Milestone: M2. Priority: P0.

## Goal

Connect the hero overview to local artwork without guessing hero identities.

## Scope

Implement stored folder selection, executable-parent discovery, debug-only development source configuration and diagnostic archive status. Add resource catalog, opaque asset keys, command image delivery and bounded cache. Investigate the sample's hero identity/portrait reference with targeted source/byte evidence; validate at least one mapping, including behavior for renamed/custom heroes. Do not derive portraits from display names or sorted list position. If identity cannot be proven, record that mapping as unresolved and use a placeholder; do not falsely close the verified-mapping acceptance item.

## Relevant files or components

crates/h3forge-resources/src/{discovery,catalog,png}.rs; save domain optional portrait identity; app commands/session/installation; web hero list and installation settings. (Implemented paths; the planned `cache.rs` became a bounded in-memory map inside the app session, which is all the issue's "simple caching" required.)

## Dependencies

H3F-003, H3F-004 See the [issue index](README.md) and [architecture](../ARCHITECTURE.md).

## Acceptance criteria

- [x] Portable executable-parent and manually selected installation paths work; current working directory is not assumed.
- [x] Development Data works only as explicit debug input and never a bundled resource.
- [x] At least one evidence-backed portrait mapping is displayed; uncertain identities use clearly generic placeholders.
- [x] Missing/corrupt archives leave the overview functional; cache invalidates after source change and writes outside the game folder.

## Implementation notes (2026-11-08)

- **Hero identity (the core investigation):** the runtime hero record carries `hero_type` (offset 10), `hero_class` (11) and `hero_portrait` (12) per the h3svg specification (alexanderbelous/h3mtxt `h3svg` branch `Hero` struct). Evidence for the HotA 44/11 sample: field alignment checks (destination `-1` words at 19–27, movement at 31/35, spell points at 47, level at 49); the class-byte histogram matching exactly the 24 known classes (16 SoD + 8 HotA/placeholder); unchanged CP1251 roster names at type-consistent classes (record 0 `ce f0 f0 e8 ed` = Оррин/Orrin, type 0 Knight; record 16 Мефала/Mephala, type 16 Ranger; record 8 Рион/Rion, type 8 Cleric); and renamed heroes ("AbsoluteZero", "J. Lannister", "Ben Kenoby", "Trichelieu", …) keeping their type bytes - names are never used as identity. `HeroView.portrait: Option<PortraitIdentity>` exposes this (h3forge-save, `domain.rs`, `definitions::hero::{HERO_TYPE,HERO_CLASS,HERO_PORTRAIT}`).
- **Portraits:** the displayed portrait is `hero_portrait` (equals `hero_type` unless customized). The game binary references `HPL<number><class>.PCX` files; the sample `H3bitmap.lod` carries portrait entries 000–136 in three families (`HPL###XX` large 58×64, `HPS###XX` and `PS###XX` small), with 130–136 stored under mixed-case names (`HPL130Kn.pcx`). `PortraitCatalog` (h3forge-resources `catalog.rs`) indexes the directory case-insensitively by ID and resolves without any class-name table. `png.rs` hand-encodes PNG (8-bit RGBA, zlib via miniz_oxide, CRC via crc32fast).
- **Installation discovery** (`discovery.rs`): candidates are the exe directory (portable layout), a debug-only `H3FORGE_DEV_DATA_ROOT`-configured root (`#[cfg(debug_assertions)]`, never bundled), and the stored selection - the working directory is never consulted, nothing is written under an installation, and there is no recursive search. `validate` accepts the root or its `Data` child, opens each known archive read-only (metadata only) and reports per-archive status (`Readable`/`Unsupported`/`Unreadable`); the supplied `HotA.lod` reports `Unsupported` while base archives stay usable. Resource sets are derived from what actually validated (SoD base, AB, HotA) without assuming every HotA archive exists.
- **App layer:** stored selection persists in the OS config dir (`installation.txt`, Rust-written only). Commands: `get_installation_status`, `choose_installation` (native picker, then validate+persist), `get_hero_portrait(id, size)`, batched `get_hero_portraits(ids, size)`; all registered in `main.rs`, `build.rs` and `capabilities/main.json`. Portraits serve as base64 PNG with the identity attached; unresolved results carry a human reason and the frontend renders a generic `?` placeholder - no artwork is invented. The in-memory cache (≤512 entries, keyed by archive entry) is invalidated by rebuilding the whole artwork state whenever the archive path/length/mtime identity changes; cache writes never leave the process.
- **Frontend:** hero list rows show small portraits, the detail header the large one; an installation bar shows resource sets and per-archive diagnostics with an "Installation…" picker. Search/filter/selection unchanged; no redesign.

## Verification (2026-11-08)

- Sample identity (opt-in `tests/sample_portraits.rs`): all 215 records carry identity; record 0 = (0,0,0); renamed record 1 keeps type 1; record 16 class 2 (Ranger); 147 default portraits (id == type), 68 customized, max ID 244 - all pinned.
- Real-archive pipeline (`tests/portraits.rs`): catalog ≥ 137 portraits (000–136, mixed-case 130–136 indexed); portrait 0 entry → PCX 58×64 → PNG with correct IHDR dims; small variants decode as separate artwork; ID 137/244 absent.
- Discovery (`tests/discovery.rs`, synthetic temp dirs): root and `Data`-child validation, empty/archiveless rejection, per-archive status kinds, unknown names never probed, `NotFound` without candidates, stored selection revalidated.
- End-to-end (`apps/h3forge-desktop` `tests/portrait_flow.rs`, no Tauri runtime): installation set → sample opened → record 0 resolves to PNG (cached repeat identical, identity attached) → renamed record 4 resolves → custom portrait 213 returns explicit unresolved.
- `cargo test -p h3forge-save` (6 suites incl. sample baseline), `-p h3forge-resources` (4 suites), `-p h3forge-desktop` (all) pass; `cargo clippy --workspace --all-targets` clean; `cargo fmt --check` clean; `cargo check --workspace --locked` passes; frontend `npm run build` (tsc + vite) passes.

## Resolved vs unresolved portraits (SaAndVla.GM2, 215 records)

- **131 resolve** from the readable `H3bitmap.lod` (portrait ID ≤ 136), including Orrin (0) and every renamed hero whose map kept the standard portrait.
- **84 are unavailable** (portrait IDs > 136): all 68 customized portraits and 16 default portraits of HotA roster heroes. These likely live in the unsupported `HotA.lod` (H3F-014) or are custom beyond the standard set; the UI shows explicit placeholders with the reason. Identity itself is known for all 215.

## Limitations

> Superseded in part by [H3F-021](021-map-discovery.md): portrait IDs index a name table (0–162 in `H3bitmap.lod`, 163–244 in `HotA.lod`), and all 215 sample portraits now resolve. The "84 unavailable" figures above are historical.

- `HotA.lod` portraits (and custom portraits beyond ID 136) stay unavailable until H3F-014.
- Hero class names are not displayed (only the class byte); a class table was unnecessary for portrait resolution.
- No disk cache (in-memory only, per session); the architecture's optional bounded disk cache remains future work.
- Discovery validates known archive names only; installations with renamed archives report no artwork.

## Minimal verification

Check one known mapping against the matching game's portrait, then a renamed/unknown hero and missing archive. Test executable-parent versus manual selection and one cache invalidation. No whole-archive hashing/extraction at startup.
