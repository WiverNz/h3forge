//! Checked same-length binary patches and gzip recompression.
//!
//! Editing model (architecture: ARCHITECTURE.md "Save safety and
//! persistence"): the original payload is preserved verbatim, edits are
//! planned as [`BytePatch`] values carrying the expected old bytes, applied
//! together after overlap and preimage checks, and only the recompressed
//! result is ever a candidate for writing. Patches never resize the payload;
//! unknown and unrelated bytes survive by construction.

use crate::error::EditError;

/// A verified equal-length in-place modification of the decompressed payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytePatch {
    /// Absolute offset in the decompressed payload.
    pub offset: usize,
    /// Bytes expected at `offset` before applying; must match exactly.
    pub expected: Box<[u8]>,
    /// Replacement bytes; same length as `expected`.
    pub replacement: Box<[u8]>,
    /// Human-readable provenance, e.g. "set attack" or "learn spell 15".
    pub reason: String,
}

impl BytePatch {
    /// Creates a patch after structural checks; bounds and preimage are
    /// verified at apply time against the actual payload.
    pub fn new(
        offset: usize,
        expected: impl Into<Vec<u8>>,
        replacement: impl Into<Vec<u8>>,
        reason: impl Into<String>,
    ) -> Result<Self, EditError> {
        let expected = expected.into();
        let replacement = replacement.into();
        if expected.is_empty() {
            return Err(EditError::EmptyPatch { offset });
        }
        if expected.len() != replacement.len() {
            return Err(EditError::MismatchedPatchLength {
                offset,
                expected_len: expected.len(),
                replacement_len: replacement.len(),
            });
        }
        Ok(BytePatch {
            offset,
            expected: expected.into_boxed_slice(),
            replacement: replacement.into_boxed_slice(),
            reason: reason.into(),
        })
    }

    /// Exclusive end offset of the patched range.
    pub fn end(&self) -> usize {
        self.offset + self.expected.len()
    }
}

/// Applies every patch in one pass: structural checks, overlap detection,
/// bounds checks and preimage verification all happen before the first byte
/// is written, so a failed apply leaves `payload` untouched.
pub fn apply_patches(payload: &mut [u8], patches: &[BytePatch]) -> Result<(), EditError> {
    let mut ranges: Vec<&BytePatch> = patches.iter().collect();
    ranges.sort_by_key(|patch| patch.offset);
    let mut previous_end = 0usize;
    for patch in &ranges {
        if patch.end() > payload.len() {
            return Err(EditError::PatchOutOfBounds {
                offset: patch.offset,
                len: patch.expected.len(),
                payload_len: payload.len(),
            });
        }
        if patch.offset < previous_end {
            return Err(EditError::OverlappingPatches {
                offset: patch.offset,
            });
        }
        if payload[patch.offset..patch.end()] != patch.expected[..] {
            return Err(EditError::PreimageMismatch {
                offset: patch.offset,
                reason: patch.reason.clone(),
            });
        }
        previous_end = patch.end();
    }
    for patch in ranges {
        payload[patch.offset..patch.end()].copy_from_slice(&patch.replacement);
    }
    Ok(())
}

/// Compresses a payload into a standards-compliant single-member gzip save:
/// 10-byte header (no optional fields, MTIME 0, OS 0xFF), a DEFLATE stream
/// and a correct CRC-32 + ISIZE trailer. h3sed writes saves the same way
/// (full re-gzip of the payload through Python's gzip); the game reads any
/// valid member, so compressed bytes differ from the original while the
/// payload-level preservation guarantees still hold.
pub fn compress_gzip(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() / 2 + 64);
    out.extend_from_slice(&[0x1F, 0x8B, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xFF]);
    out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(payload, 6));
    out.extend_from_slice(&crc32fast::hash(payload).to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_requires_equal_nonzero_lengths() {
        assert!(matches!(
            BytePatch::new(4, Vec::<u8>::new(), vec![1], "x"),
            Err(EditError::EmptyPatch { offset: 4 })
        ));
        assert!(matches!(
            BytePatch::new(4, vec![1, 2], vec![1], "x"),
            Err(EditError::MismatchedPatchLength {
                offset: 4,
                expected_len: 2,
                replacement_len: 1
            })
        ));
    }

    #[test]
    fn applies_disjoint_patches() {
        let mut payload = vec![0u8; 16];
        let patches = [
            BytePatch::new(2, vec![0, 0], vec![9, 8], "first").unwrap(),
            BytePatch::new(8, vec![0], vec![7], "second").unwrap(),
        ];
        apply_patches(&mut payload, &patches).unwrap();
        assert_eq!(
            payload,
            vec![0, 0, 9, 8, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn rejects_overlaps_even_when_consistent() {
        let mut payload = vec![0u8; 16];
        let patches = [
            BytePatch::new(2, vec![0, 0], vec![9, 9], "first").unwrap(),
            BytePatch::new(3, vec![0], vec![8], "second").unwrap(),
        ];
        assert!(matches!(
            apply_patches(&mut payload, &patches),
            Err(EditError::OverlappingPatches { offset: 3 })
        ));
        // Failed apply wrote nothing.
        assert!(payload.iter().all(|&byte| byte == 0));
    }

    #[test]
    fn rejects_preimage_mismatch_without_writing() {
        let mut payload = vec![0u8; 16];
        let patches = [BytePatch::new(2, vec![5], vec![9], "wrong-old").unwrap()];
        assert!(matches!(
            apply_patches(&mut payload, &patches),
            Err(EditError::PreimageMismatch { offset: 2, .. })
        ));
        assert!(payload.iter().all(|&byte| byte == 0));
    }

    #[test]
    fn rejects_out_of_bounds() {
        let mut payload = vec![0u8; 4];
        let patches = [BytePatch::new(3, vec![0, 0], vec![1, 2], "past-end").unwrap()];
        assert!(matches!(
            apply_patches(&mut payload, &patches),
            Err(EditError::PatchOutOfBounds {
                offset: 3,
                len: 2,
                payload_len: 4
            })
        ));
    }

    #[test]
    fn gzip_roundtrip_through_reader() {
        let payload: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        let member = compress_gzip(&payload);
        let got = crate::container::decompress_single_member(&member, crate::Limits::default())
            .expect("own gzip member decodes");
        assert_eq!(got, payload);
        assert_eq!(&member[..4], &[0x1F, 0x8B, 0x08, 0x00]);
        let n = member.len();
        assert_eq!(
            u32::from_le_bytes(member[n - 8..n - 4].try_into().unwrap()),
            crc32fast::hash(&payload)
        );
        assert_eq!(
            u32::from_le_bytes(member[n - 4..].try_into().unwrap()),
            payload.len() as u32
        );
    }
}
