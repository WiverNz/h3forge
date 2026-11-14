//! Narrow native checksum compatibility, without weakening output verification.
#[allow(dead_code)]
mod common;

use h3forge_save::{DiagnosticCode, Limits, SaveDocument, SaveError};

fn native_member(major: u32, minor: u32) -> (Vec<u8>, Vec<u8>) {
    let payload = common::save_payload(major, minor, b"CRC fixture", b"", &[]);
    let mut member = common::gzip_stored(&payload);
    let at = member.len() - 8;
    member[at] ^= 0x80;
    (member, payload)
}

#[test]
fn native_import_preserves_bytes_and_strict_output_rejects_mismatch() {
    let (member, payload) = native_member(44, 11);
    let doc = SaveDocument::open(member.clone()).unwrap();
    assert_eq!(doc.original_bytes(), member);
    assert_eq!(doc.payload(), payload);
    assert!(
        doc.diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::NativeGzipCrcMismatch)
    );
    assert!(matches!(
        SaveDocument::open_strict(member),
        Err(SaveError::CrcMismatch { .. })
    ));
    let output = h3forge_save::compress_gzip(doc.payload());
    let reopened = SaveDocument::open_strict(output).unwrap();
    assert_eq!(reopened.payload(), payload);
    assert!(
        !reopened
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::NativeGzipCrcMismatch)
    );
}

#[test]
fn exception_is_not_applied_to_other_versions_or_magic() {
    for (major, minor) in [(42, 2), (44, 10), (44, 12), (45, 11)] {
        assert!(matches!(
            SaveDocument::open(native_member(major, minor).0),
            Err(SaveError::CrcMismatch { .. })
        ));
    }
    for magic in [b"H3SVC", b"other"] {
        let (_, mut payload) = native_member(44, 11);
        payload[..5].copy_from_slice(magic);
        let mut member = common::gzip_stored(&payload);
        let at = member.len() - 8;
        member[at] ^= 1;
        assert!(matches!(
            SaveDocument::open(member),
            Err(SaveError::CrcMismatch { .. })
        ));
    }
}

#[test]
fn native_exception_still_rejects_bad_container_and_limits() {
    let (member, _) = native_member(44, 11);
    for cut in [7, 15, member.len() - 1, member.len() - 8] {
        assert!(SaveDocument::open(member[..cut].to_vec()).is_err());
    }
    let mut bad_size = member.clone();
    let end = bad_size.len();
    bad_size[end - 1] ^= 1;
    assert!(matches!(
        SaveDocument::open(bad_size),
        Err(SaveError::IsizeMismatch { .. })
    ));
    let mut multiple = member.clone();
    multiple.extend_from_slice(&member);
    assert!(matches!(
        SaveDocument::open(multiple),
        Err(SaveError::MultiMemberGzip)
    ));
    let mut trailing = member.clone();
    trailing.push(0);
    assert!(matches!(
        SaveDocument::open(trailing),
        Err(SaveError::TrailingData { .. })
    ));
    let mut corrupt = member.clone();
    corrupt[10] = 7; // Reserved DEFLATE block type.
    assert!(matches!(
        SaveDocument::open(corrupt),
        Err(SaveError::CorruptDeflate)
    ));
    let mut bad_header = member.clone();
    bad_header[3] = 0xE0;
    assert!(matches!(
        SaveDocument::open(bad_header),
        Err(SaveError::GzipUnsupportedFlags { .. })
    ));
    assert!(matches!(
        SaveDocument::open_with_limits(
            member.clone(),
            Limits {
                max_compressed: 1,
                max_payload: 100_000
            }
        ),
        Err(SaveError::InputTooLarge { .. })
    ));
    assert!(matches!(
        SaveDocument::open_with_limits(
            member,
            Limits {
                max_compressed: 100_000,
                max_payload: 32
            }
        ),
        Err(SaveError::PayloadTooLarge { .. })
    ));
}
