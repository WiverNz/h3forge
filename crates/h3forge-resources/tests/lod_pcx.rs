//! Synthetic LOD archive and PCX fixture tests.
//!
//! All fixtures are built in memory and written to OS-unique temporary paths;
//! no game data is touched and nothing persists after the test run.

use std::fs;
use std::path::PathBuf;

use h3forge_resources::{LodArchive, PcxImage, ResourceError, ResourceLimits, decode_pcx};

// --------------------------------------------------------------------- LOD --

const HEADER_SIZE: usize = 92;
const ENTRY_SIZE: usize = 32;
const NAME_SIZE: usize = 16;

fn u32le(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// One synthetic directory record plus its payload.
struct FixtureEntry {
    name: &'static str,
    /// What the directory stores: `(data, packed)`. Packed entries are
    /// zlib-compressed here, mirroring what real archives store.
    data: Vec<u8>,
    packed: bool,
}

/// Builds a complete LOD archive image from entries, laying payloads out back
/// to back right after the directory table.
fn build_lod(entries: &[FixtureEntry]) -> Vec<u8> {
    let mut out = vec![0u8; HEADER_SIZE];
    out[0..4].copy_from_slice(b"LOD\0");
    out[4..8].copy_from_slice(&u32le(200)); // standard H3 LOD version
    out[8..12].copy_from_slice(&u32le(entries.len() as u32));

    let data_start = HEADER_SIZE + entries.len() * ENTRY_SIZE;
    let mut offset = data_start as u32;
    let mut payloads = Vec::new();
    for entry in entries {
        let mut record = [0u8; ENTRY_SIZE];
        record[..entry.name.len()].copy_from_slice(entry.name.as_bytes());
        let stored = if entry.packed {
            miniz_oxide::deflate::compress_to_vec_zlib(&entry.data, 6)
        } else {
            entry.data.clone()
        };
        record[NAME_SIZE..NAME_SIZE + 4].copy_from_slice(&u32le(offset));
        let (unpacked, packed_size) = if entry.packed {
            (entry.data.len() as u32, stored.len() as u32)
        } else {
            (stored.len() as u32, 0)
        };
        record[NAME_SIZE + 4..NAME_SIZE + 8].copy_from_slice(&u32le(unpacked));
        record[NAME_SIZE + 12..NAME_SIZE + 16].copy_from_slice(&u32le(packed_size));
        out.extend_from_slice(&record);
        offset += stored.len() as u32;
        payloads.extend_from_slice(&stored);
    }
    out.extend_from_slice(&payloads);
    out
}

fn temp_lod(bytes: &[u8]) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "h3forge-lod-test-{}-{}.lod",
        std::process::id(),
        bytes.len()
    ));
    fs::write(&path, bytes).expect("fixture archive writes");
    path
}

fn open(bytes: &[u8]) -> Result<LodArchive, ResourceError> {
    let path = temp_lod(bytes);
    let result = LodArchive::open(&path);
    fs::remove_file(&path).ok();
    result
}

#[test]
fn enumerates_and_reads_raw_and_packed_entries() {
    let raw = b"plain text payload".to_vec();
    let packed: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
    let lod = build_lod(&[
        FixtureEntry {
            name: "A.TXT",
            data: raw.clone(),
            packed: false,
        },
        FixtureEntry {
            name: "B.PCX",
            data: packed.clone(),
            packed: true,
        },
    ]);

    let archive = open(&lod).expect("archive opens");
    assert_eq!(archive.entries().len(), 2);
    assert_eq!(archive.entries()[0].name, "A.TXT");
    assert!(!archive.entries()[0].packed);
    assert_eq!(archive.entries()[0].stored_size, raw.len() as u32);
    assert_eq!(archive.entries()[1].name, "B.PCX");
    assert!(archive.entries()[1].packed);

    // Names resolve case-insensitively.
    let index = archive.find_entry("a.txt").expect("lookup resolves");
    assert_eq!(archive.read_entry(index).expect("raw entry reads"), raw);
    let index = archive.find_entry("B.pcx").expect("lookup resolves");
    assert_eq!(
        archive.read_entry(index).expect("packed entry reads"),
        packed
    );
}

#[test]
fn opening_does_not_decode_payloads() {
    // The directory range is in bounds but the stored payload is not a zlib
    // stream at all: opening must succeed (payloads are untouched), and the
    // failure surfaces only at read_entry.
    let lod = build_lod(&[FixtureEntry {
        name: "BAD.PCX",
        data: vec![42u8; 64],
        packed: true,
    }]);
    // Corrupt the stored payload region in place, keeping directory intact.
    let mut broken = lod;
    let data_at = HEADER_SIZE + ENTRY_SIZE;
    broken[data_at..].fill(b'A');

    let archive = open(&broken).expect("open never decodes payloads");
    assert_eq!(archive.entries().len(), 1);
    assert!(matches!(
        archive.read_entry(0).unwrap_err(),
        ResourceError::CorruptCompression { .. }
    ));
}

#[test]
fn rejects_zero_byte_archive() {
    let error = open(&[]).unwrap_err();
    assert!(matches!(error, ResourceError::EmptyArchive));
}

#[test]
fn rejects_bad_magic() {
    let mut lod = build_lod(&[]);
    lod[0] = b'X';
    assert!(matches!(open(&lod).unwrap_err(), ResourceError::BadMagic));
}

#[test]
fn rejects_short_header() {
    let mut lod = build_lod(&[]);
    lod.truncate(40);
    assert!(matches!(
        open(&lod).unwrap_err(),
        ResourceError::DirectoryTruncated { .. }
    ));
}

#[test]
fn rejects_huge_entry_count() {
    let mut lod = build_lod(&[]);
    lod[8..12].copy_from_slice(&u32le(1_000_000));
    assert!(matches!(
        open(&lod).unwrap_err(),
        ResourceError::EntryCountLimit { .. }
    ));
}

#[test]
fn rejects_short_directory_table() {
    let mut lod = build_lod(&[]);
    lod[8..12].copy_from_slice(&u32le(10));
    lod.resize(HEADER_SIZE + 2 * ENTRY_SIZE, 0);
    assert!(matches!(
        open(&lod).unwrap_err(),
        ResourceError::DirectoryTruncated { .. }
    ));
}

#[test]
fn rejects_nonstandard_first_directory_record() {
    // The supplied nonstandard HotA.lod shape: plausible header, garbage
    // first record that a standard layout cannot safely decode.
    let mut lod = build_lod(&[FixtureEntry {
        name: "A.TXT",
        data: b"x".to_vec(),
        packed: false,
    }]);
    let first = HEADER_SIZE;
    lod[first..first + 8].copy_from_slice(&[0x9F, 0x18, 0x54, 0x6E, 0x18, 0x59, 0xA6, 0xB5]);
    assert!(matches!(
        open(&lod).unwrap_err(),
        ResourceError::UnsupportedDirectory { .. }
    ));
}

#[test]
fn rejects_bad_later_directory_record() {
    let lod = build_lod(&[
        FixtureEntry {
            name: "OK.TXT",
            data: b"x".to_vec(),
            packed: false,
        },
        FixtureEntry {
            name: "OK2.TXT",
            data: b"y".to_vec(),
            packed: false,
        },
    ]);
    // Point the second record's data past the end of the file.
    let mut broken = lod.clone();
    let second = HEADER_SIZE + ENTRY_SIZE;
    broken[second + NAME_SIZE..second + NAME_SIZE + 4].copy_from_slice(&u32le(9_000_000));
    assert!(matches!(
        open(&broken).unwrap_err(),
        ResourceError::InvalidDirectory { index: 1, .. }
    ));
}

#[test]
fn rejects_entry_pointing_into_directory_table() {
    let lod = build_lod(&[FixtureEntry {
        name: "A.TXT",
        data: b"x".to_vec(),
        packed: false,
    }]);
    let mut broken = lod;
    broken[HEADER_SIZE + NAME_SIZE..HEADER_SIZE + NAME_SIZE + 4]
        .copy_from_slice(&u32le(HEADER_SIZE as u32 - 4));
    assert!(matches!(
        open(&broken).unwrap_err(),
        ResourceError::UnsupportedDirectory { .. }
    ));
}

#[test]
fn rejects_ambiguous_and_missing_names() {
    let lod = build_lod(&[
        FixtureEntry {
            name: "DUP.TXT",
            data: b"1".to_vec(),
            packed: false,
        },
        FixtureEntry {
            name: "dup.txt",
            data: b"2".to_vec(),
            packed: false,
        },
    ]);
    let path = temp_lod(&lod);
    let archive = LodArchive::open(&path).expect("archive opens");
    assert!(matches!(
        archive.find_entry("Dup.Txt").unwrap_err(),
        ResourceError::AmbiguousName { .. }
    ));
    assert!(matches!(
        archive.find_entry("absent.bin").unwrap_err(),
        ResourceError::EntryNotFound { .. }
    ));
    fs::remove_file(&path).ok();
}

#[test]
fn rejects_bad_entry_index() {
    let lod = build_lod(&[FixtureEntry {
        name: "A.TXT",
        data: b"x".to_vec(),
        packed: false,
    }]);
    let archive = open(&lod).expect("archive opens");
    assert!(matches!(
        archive.read_entry(7).unwrap_err(),
        ResourceError::BadIndex { .. }
    ));
}

#[test]
fn rejects_over_limit_entry_size() {
    let big = vec![0u8; 4096];
    let lod = build_lod(&[FixtureEntry {
        name: "BIG.TXT",
        data: big,
        packed: false,
    }]);
    let path = temp_lod(&lod);
    let error = LodArchive::open_with_limits(
        &path,
        ResourceLimits {
            max_entries: 100,
            max_entry_bytes: 1024,
            max_image_pixels: 1_000,
        },
    )
    .unwrap_err();
    fs::remove_file(&path).ok();
    assert!(matches!(error, ResourceError::EntrySizeLimit { .. }));
}

#[test]
fn rejects_impossible_unpacked_size() {
    // packed_size 10 bytes can never inflate to the claimed 1 MB.
    let mut lod = build_lod(&[FixtureEntry {
        name: "A.TXT",
        data: b"x".to_vec(),
        packed: true,
    }]);
    let record = HEADER_SIZE;
    lod[record + NAME_SIZE + 4..record + NAME_SIZE + 8].copy_from_slice(&u32le(1_000_000));
    let archive = open(&lod).expect("directory itself stays plausible");
    assert!(matches!(
        archive.read_entry(0).unwrap_err(),
        ResourceError::ImpossibleSize { .. }
    ));
}

#[test]
fn rejects_truncated_zlib_stream() {
    // Stored stream is valid zlib of 50 bytes, but the directory declares
    // 5000 unpacked: the stream ends long before the declared size.
    let lod = build_lod(&[FixtureEntry {
        name: "A.TXT",
        data: vec![7u8; 50],
        packed: true,
    }]);
    let mut broken = lod;
    let record = HEADER_SIZE;
    broken[record + NAME_SIZE + 4..record + NAME_SIZE + 8].copy_from_slice(&u32le(5000));
    let archive = open(&broken).expect("directory itself stays plausible");
    assert!(matches!(
        archive.read_entry(0).unwrap_err(),
        ResourceError::DecompressTruncated {
            got: 50,
            expected: 5000,
            ..
        }
    ));
}

#[test]
fn rejects_corrupt_zlib_stream() {
    let lod = build_lod(&[FixtureEntry {
        name: "A.TXT",
        data: b"whatever".to_vec(),
        packed: true,
    }]);
    // Overwrite the stored payload region with non-zlib bytes, same size.
    let mut broken = lod.clone();
    let data_at = HEADER_SIZE + ENTRY_SIZE;
    let stored = broken.len() - data_at;
    broken[data_at..].fill(b'A');
    assert!(stored > 0);
    let archive = open(&broken).expect("directory itself stays plausible");
    assert!(matches!(
        archive.read_entry(0).unwrap_err(),
        ResourceError::CorruptCompression { .. }
    ));
}

#[test]
fn accepts_empty_archive_with_zero_entries() {
    let lod = build_lod(&[]);
    let archive = open(&lod).expect("zero-entry archive opens");
    assert!(archive.entries().is_empty());
}

// -------------------------------------------------------------------- PCX --

fn indexed_pcx(width: i32, height: i32, indices: &[u8], palette: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&u32le((indices.len()) as u32));
    out.extend_from_slice(&u32le(width as u32));
    out.extend_from_slice(&u32le(height as u32));
    out.extend_from_slice(indices);
    out.extend_from_slice(palette);
    out
}

#[test]
fn decodes_indexed_pcx_with_explicit_palette() {
    // 2x2 image, palette entry i = (3i, 3i+1, 3i+2) modulo 256.
    let palette: Vec<u8> = (0..256u32)
        .flat_map(|i| [(i * 3) as u8, (i * 3 + 1) as u8, (i * 3 + 2) as u8])
        .collect();
    let data = indexed_pcx(2, 2, &[0, 1, 1, 0], &palette);
    let image = decode_pcx(&data, ResourceLimits::default()).expect("PCX decodes");
    let rgba = image.to_rgba();
    match &image {
        PcxImage::Indexed {
            width,
            height,
            indices,
            palette: decoded_palette,
        } => {
            assert_eq!(*width, 2);
            assert_eq!(*height, 2);
            assert_eq!(indices, &vec![0, 1, 1, 0]);
            assert_eq!(decoded_palette, &palette);
            // Palette bytes are R,G,B per entry: entry 1 starts at offset 3.
            assert_eq!(&decoded_palette[0..6], &[0, 1, 2, 3, 4, 5]);
        }
        other => panic!("expected an indexed image, got {other:?}"),
    }
    assert_eq!((rgba.width, rgba.height), (2, 2));
    assert_eq!(rgba.data.len(), 2 * 2 * 4);
    // Top-left pixel = index 0 -> (0,1,2,255); top-right = index 1 -> (3,4,5,255).
    assert_eq!(&rgba.data[0..4], &[0, 1, 2, 255]);
    assert_eq!(&rgba.data[4..8], &[3, 4, 5, 255]);
}

#[test]
fn decodes_24bit_pcx_swapping_bgr_to_rgba() {
    // 2x1 image; stored triplets are B,G,R per the format's proven handling.
    let mut data = Vec::new();
    data.extend_from_slice(&u32le(6)); // image size: 2 pixels * 3 bytes
    data.extend_from_slice(&u32le(2));
    data.extend_from_slice(&u32le(1));
    data.extend_from_slice(&[30, 20, 10, 60, 50, 40]); // B,G,R / B,G,R
    let image = decode_pcx(&data, ResourceLimits::default()).expect("PCX decodes");
    let rgba = image.to_rgba();
    match &image {
        PcxImage::Bgr24 { width, height, bgr } => {
            assert_eq!(*width, 2);
            assert_eq!(*height, 1);
            assert_eq!(bgr, &vec![30, 20, 10, 60, 50, 40]);
        }
        other => panic!("expected a 24-bit image, got {other:?}"),
    }
    assert_eq!(&rgba.data[0..4], &[10, 20, 30, 255]);
    assert_eq!(&rgba.data[4..8], &[40, 50, 60, 255]);
}

#[test]
fn rejects_short_pcx() {
    assert!(matches!(
        decode_pcx(&[0u8; 5], ResourceLimits::default()).unwrap_err(),
        ResourceError::PcxTruncated { .. }
    ));
}

#[test]
fn rejects_zero_sized_pcx() {
    let mut data = Vec::new();
    data.extend_from_slice(&u32le(4));
    data.extend_from_slice(&u32le(0)); // width 0
    data.extend_from_slice(&u32le(2));
    data.extend_from_slice(&[0u8; 4]);
    assert!(matches!(
        decode_pcx(&data, ResourceLimits::default()).unwrap_err(),
        ResourceError::PcxBadDimensions { .. }
    ));
}

#[test]
fn rejects_bad_pixel_length() {
    // image size neither 1 nor 3 bytes per pixel for 2x2.
    let data = indexed_pcx(2, 2, &[0u8; 8], &[0u8; 768]);
    assert!(matches!(
        decode_pcx(&data, ResourceLimits::default()).unwrap_err(),
        ResourceError::PcxBadPixelLength { .. }
    ));
}

#[test]
fn rejects_truncated_pixel_data() {
    let mut data = indexed_pcx(2, 2, &[0, 1, 1, 0], &vec![0u8; 768]);
    data.truncate(data.len() - 770); // cut into the pixel region
    assert!(matches!(
        decode_pcx(&data, ResourceLimits::default()).unwrap_err(),
        ResourceError::PcxTruncated { .. }
    ));
}

#[test]
fn rejects_missing_palette() {
    let data = indexed_pcx(2, 2, &[0, 1, 1, 0], &[0u8; 100]); // short palette
    assert!(matches!(
        decode_pcx(&data, ResourceLimits::default()).unwrap_err(),
        ResourceError::PcxMissingPalette { .. }
    ));
}

#[test]
fn rejects_pixel_limit() {
    let data = indexed_pcx(16, 16, &vec![0u8; 256], &vec![0u8; 768]);
    assert!(matches!(
        decode_pcx(
            &data,
            ResourceLimits {
                max_entries: 100,
                max_entry_bytes: 1024,
                max_image_pixels: 100,
            },
        )
        .unwrap_err(),
        ResourceError::PixelLimit { .. }
    ));
}

fn hota_fixture(method: u8, data: &[u8]) -> Vec<u8> {
    let key = 0xb5a4d744u32;
    let stored = if method == 0 {
        data.to_vec()
    } else {
        miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
    };
    let mut bytes = vec![0u8; 112];
    bytes[..4].copy_from_slice(b"LOD\0");
    bytes[4..8].copy_from_slice(&200u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
    bytes[12..16].copy_from_slice(&key.to_le_bytes());
    bytes[80..96].fill(0xab);
    for (at, value) in [
        (96, 112u32),
        (100, data.len() as u32),
        (104, if method == 0 { 0 } else { stored.len() as u32 }),
    ] {
        bytes[at..at + 4].copy_from_slice(&(value ^ key).to_le_bytes());
    }
    bytes[108] = method;
    bytes.extend_from_slice(&stored);
    bytes
}

#[test]
fn hota_stored_zlib_and_unsupported_method() {
    for method in [0, 3] {
        let archive = open(&hota_fixture(method, b"verified bounded entry")).unwrap();
        assert!(archive.has_opaque_ids());
        assert_eq!(
            archive
                .find_entry("abababababababababababababababab")
                .unwrap(),
            0
        );
        assert_eq!(archive.read_entry(0).unwrap(), b"verified bounded entry");
    }
    let archive = open(&hota_fixture(2, b"unsupported lzma test payload")).unwrap();
    assert!(matches!(
        archive.read_entry(0),
        Err(ResourceError::UnsupportedCompression { method: 2, .. })
    ));
}

#[test]
fn hota_rejects_bounds_limits_wrong_key_and_bad_compression() {
    let valid = hota_fixture(3, b"a bounded test payload");
    for (at, value) in [
        (96, 0u32 ^ 0xb5a4d744),
        (96, u32::MAX ^ 0xb5a4d744),
        (100, (33 * 1024 * 1024u32) ^ 0xb5a4d744),
        (12, 0x12345678),
        (8, 100001),
    ] {
        let mut bad = valid.clone();
        bad[at..at + 4].copy_from_slice(&value.to_le_bytes());
        assert!(open(&bad).is_err());
    }
    assert!(open(&valid[..100]).is_err());
    let mut corrupt = valid.clone();
    corrupt[112..].fill(0);
    assert!(open(&corrupt).unwrap().read_entry(0).is_err());
    let mut short = valid.clone();
    short[100..104].copy_from_slice(&(1u32 ^ 0xb5a4d744).to_le_bytes());
    assert!(open(&short).unwrap().read_entry(0).is_err());
    let mut mismatch = valid;
    mismatch[108] = 0;
    assert!(open(&mismatch).is_err());
}

#[test]
fn standard_lod_nonzero_reserved_word_keeps_standard_layout() {
    let mut bytes = build_lod(&[FixtureEntry {
        name: "sample.txt",
        data: b"standard data".to_vec(),
        packed: false,
    }]);
    bytes[12..16].copy_from_slice(&0x12345678u32.to_le_bytes());
    let archive = open(&bytes).unwrap();
    assert!(!archive.has_opaque_ids());
    assert_eq!(archive.read_entry(0).unwrap(), b"standard data");
}
