//! Portrait catalog and real-archive pipeline verification (opt-in against
//! the local `Data/` directory, overridable with `H3FORGE_FIXTURE_ROOT`).
//! Everything is read-only; no archive content is written anywhere.

use std::path::{Path, PathBuf};

use h3forge_resources::{LodArchive, PortraitCatalog, ResourceLimits, decode_pcx, encode_png};

fn data_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") {
        let root = PathBuf::from(root);
        if root.is_dir() {
            return Some(root);
        }
    }
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Data");
    match data.canonicalize() {
        Ok(path) if path.is_dir() => Some(path),
        _ => None,
    }
}

#[test]
fn real_archive_catalog_and_png_pipeline() {
    let Some(root) = data_root() else {
        eprintln!("no data root; skipping real portrait test");
        return;
    };
    let bitmaps_path = root.join("H3bitmap.lod");
    if !bitmaps_path.is_file() {
        eprintln!("H3bitmap.lod absent; skipping real portrait test");
        return;
    }
    let bitmaps = LodArchive::open(&bitmaps_path).expect("bitmaps open");
    let catalog = PortraitCatalog::scan(bitmaps.entries());

    // The sample archives carry portraits 000-129 in three size families.
    assert!(
        catalog.ids().count() >= 137,
        "at least the 137 known portraits"
    );
    let asset = catalog.resolve(0).expect("portrait 0 indexed");
    let large = asset.large.expect("HPL000KN.PCX present");
    assert!(asset.small.len() >= 2, "HPS + PS small variants indexed");

    // Full pipeline: entry -> PCX -> RGBA -> PNG, with preserved dimensions.
    let bytes = bitmaps.read_entry(large).expect("portrait entry reads");
    let image = decode_pcx(&bytes, ResourceLimits::default()).expect("PCX decodes");
    assert_eq!((image.width(), image.height()), (58, 64));
    let png = encode_png(&image.to_rgba());
    assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    assert_eq!(&png[12..16], b"IHDR");
    assert_eq!(&png[16..20], &58u32.to_be_bytes());
    assert_eq!(&png[20..24], &64u32.to_be_bytes());

    // Small variants decode too; they are separate artwork (different
    // proportions than the 58x64 large portrait), never stretched.
    for small in &asset.small {
        let bytes = bitmaps.read_entry(*small).expect("small entry reads");
        let small_image = decode_pcx(&bytes, ResourceLimits::default()).expect("small decodes");
        assert!(small_image.width() > 0 && small_image.height() > 0);
    }

    // Portraits 130-136 exist under mixed-case names (`HPL130Kn.pcx`); beyond
    // 136 no standard portrait exists in this archive (custom/HotA portraits
    // may live in the unsupported HotA.lod and must stay unresolved).
    assert!(catalog.resolve(136).is_some(), "mixed-case 130-136 indexed");
    assert!(catalog.resolve(137).is_none());
    assert!(catalog.resolve(244).is_none());
}
