//! Opt-in verification against the real local game archives in `Data/`.
//!
//! These archives are private, copyrighted reference material: they are only
//! ever opened read-only and are never copied, modified or used as a write
//! destination. Set `H3FORGE_FIXTURE_ROOT` to override the archive directory;
//! without a resolvable directory the tests skip.

use std::path::{Path, PathBuf};

use h3forge_resources::{LodArchive, PcxImage, ResourceError, decode_pcx};

fn fixture_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") {
        let root = PathBuf::from(root);
        if root.is_dir() {
            return Some(root);
        }
    }
    // Default: the workspace's own read-only reference archives.
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Data");
    match data.canonicalize() {
        Ok(path) if path.is_dir() => Some(path),
        _ => None,
    }
}

fn open_archive(root: &Path, name: &str) -> Option<LodArchive> {
    let path = root.join(name);
    if !path.is_file() {
        eprintln!("{name} not present; skipping");
        return None;
    }
    Some(LodArchive::open(&path).unwrap_or_else(|e| panic!("{name} opens: {e}")))
}

#[test]
fn h3bitmap_enumerates_and_decodes_indexed_portrait() {
    let Some(root) = fixture_root() else {
        eprintln!("no fixture root; skipping real-archive test");
        return;
    };
    let Some(archive) = open_archive(&root, "H3bitmap.lod") else {
        return;
    };
    assert_eq!(archive.entries().len(), 4912, "H3bitmap.lod entry count");

    // Portrait entry researched in RESEARCH.md "Actual local resources".
    let index = archive
        .find_entry("HPL000KN.PCX")
        .expect("portrait entry present");
    let entry = &archive.entries()[index];
    assert_eq!(entry.name, "HPL000KN.PCX");
    assert!(entry.packed);
    assert_eq!(entry.stored_size, 4303);
    assert_eq!(entry.unpacked_size, 4492);

    let bytes = archive.read_entry(index).expect("portrait reads");
    assert_eq!(bytes.len(), 4492, "decompressed to the declared size");

    // 58x64 indexed image: 12-byte header + 3712 pixels + 768-byte palette.
    let image = decode_pcx(&bytes, Default::default()).expect("portrait PCX decodes");
    let PcxImage::Indexed {
        width,
        height,
        indices,
        palette,
    } = &image
    else {
        panic!("portrait is an indexed PCX");
    };
    assert_eq!(*width, 58);
    assert_eq!(*height, 64);
    assert_eq!(indices.len(), 58 * 64);
    assert_eq!(palette.len(), 768);
    // First palette entry is the cyan (0,255,255); palette bytes are R,G,B.
    assert_eq!(&palette[..6], &[0, 255, 255, 255, 150, 255]);

    let rgba = image.to_rgba();
    assert_eq!(rgba.data.len(), 58 * 64 * 4);
    assert_eq!(&rgba.data[..4], &[96, 97, 37, 255], "top-left pixel");
}

#[test]
fn hota_lng_reads_24bit_pcx() {
    let Some(root) = fixture_root() else {
        eprintln!("no fixture root; skipping real-archive test");
        return;
    };
    let Some(archive) = open_archive(&root, "HotA_lng.lod") else {
        return;
    };
    assert_eq!(archive.entries().len(), 225, "HotA_lng.lod entry count");

    // 800x600 campaign background, stored as 24-bit B,G,R triplets.
    let index = archive
        .find_entry("CAMPBACK.pcx")
        .expect("background entry present");
    let entry = &archive.entries()[index];
    assert!(entry.packed);
    assert_eq!(entry.unpacked_size, 1_440_012, "12 + 800*600*3");
    let bytes = archive.read_entry(index).expect("background reads");

    let image = decode_pcx(&bytes, Default::default()).expect("background PCX decodes");
    let PcxImage::Bgr24 { width, height, bgr } = &image else {
        panic!("background is a 24-bit PCX");
    };
    assert_eq!(*width, 800);
    assert_eq!(*height, 600);
    assert_eq!(bgr.len(), 800 * 600 * 3);

    let rgba = image.to_rgba();
    assert_eq!(&rgba.data[..4], &[47, 47, 47, 255], "top-left pixel");
    let center = (300 * 800 + 400) * 4;
    assert_eq!(
        &rgba.data[center..center + 4],
        &[164, 165, 168, 255],
        "center pixel"
    );
}

#[test]
fn hota_lod_directory_and_one_asset_decode() {
    let Some(root) = fixture_root() else {
        eprintln!("no fixture root; skipping real-archive test");
        return;
    };
    let path = root.join("HotA.lod");
    if !path.is_file() {
        eprintln!("HotA.lod not present; skipping");
        return;
    }
    let archive = LodArchive::open(&path).unwrap();
    assert!(archive.has_opaque_ids());
    assert_eq!(archive.entries().len(), 5232);
    assert_eq!(
        archive
            .entries()
            .iter()
            .filter(|e| e.compression == 3)
            .count(),
        3869
    );
    // Verified interface DEF token; no semantic skill identity is claimed.
    let id = archive
        .find_entry("7aa5a6b612d85711b32e3e287b850e66")
        .unwrap();
    let data = archive.read_entry(id).unwrap();
    let def = h3forge_resources::DefFile::parse(data, Default::default()).unwrap();
    assert_eq!(def.frames().len(), 4);
    assert_eq!(def.frames()[0].name, "iam000A000");
    let image = def.decode_index(0).unwrap();
    assert_eq!((image.width, image.height), (32, 32));
    assert!(h3forge_resources::encode_png(&image.to_rgba()).starts_with(b"\x89PNG"));
}

#[test]
fn empty_and_zero_byte_archives_report_clean_errors() {
    let Some(root) = fixture_root() else {
        eprintln!("no fixture root; skipping real-archive test");
        return;
    };
    // 92-byte header, zero entries: valid, empty.
    if let Some(archive) = open_archive(&root, "HotA_ext.lod") {
        assert!(archive.entries().is_empty());
    }
    // Zero bytes: rejected.
    let path = root.join("h3ab_bmp.lod");
    if path.is_file() {
        let error = LodArchive::open(&path).unwrap_err();
        assert!(matches!(error, ResourceError::EmptyArchive));
    }
}
