//! gzip save container: bounded, integrity-checked single-member decoding.
//!
//! Unlike h3sed (`metadata.patch_gzip_for_partial`), which monkey-patches
//! Python gzip to tolerate CRC problems while writing, H3Forge validates the
//! CRC-32 and ISIZE trailer up front: any integrity failure blocks the open,
//! and multi-member or trailing-data containers are rejected explicitly
//! rather than skipped silently (research: RESEARCH.md "H3SED architecture").

use miniz_oxide::inflate::TINFLStatus;
use miniz_oxide::inflate::core::inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
use miniz_oxide::inflate::core::{DecompressorOxide, decompress};

use crate::error::SaveError;

/// Resource limits enforced before and during decompression
/// (research: ARCHITECTURE.md "Binary, domain, operations and DTOs").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum accepted compressed input size.
    pub max_compressed: usize,
    /// Maximum accepted decompressed payload size.
    pub max_payload: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_compressed: 64 * 1024 * 1024,
            max_payload: 256 * 1024 * 1024,
        }
    }
}

const GZIP_MAGIC: [u8; 2] = [0x1F, 0x8B];
const GZIP_HEADER_MIN: usize = 10;
const GZIP_TRAILER_LEN: usize = 8;
const FHCRC: u8 = 0x02;
const FEXTRA: u8 = 0x04;
const FNAME: u8 = 0x08;
const FCOMMENT: u8 = 0x10;
const RESERVED_FLAGS: u8 = 0xE0;

/// Decompresses a single-member gzip save, validating header, CRC-32 and
/// ISIZE trailer, and rejecting multi-member or trailing-data input.
pub fn decompress_single_member(input: &[u8], limits: Limits) -> Result<Vec<u8>, SaveError> {
    if input.len() > limits.max_compressed {
        return Err(SaveError::InputTooLarge {
            limit: limits.max_compressed,
            actual: input.len(),
        });
    }
    if input.len() < GZIP_HEADER_MIN {
        return Err(SaveError::GzipTruncated {
            needed: GZIP_HEADER_MIN,
            actual: input.len(),
        });
    }
    if input[..2] != GZIP_MAGIC {
        return Err(SaveError::NotGzip);
    }
    let method = input[2];
    if method != 8 {
        return Err(SaveError::GzipUnsupportedMethod { method });
    }
    let flags = input[3];
    if flags & RESERVED_FLAGS != 0 {
        return Err(SaveError::GzipUnsupportedFlags { flags });
    }
    let data_start = skip_header_optional_fields(input, flags)?;
    let (payload, consumed) = inflate_bounded(&input[data_start..], limits.max_payload)?;
    let trailer_at = data_start + consumed;
    validate_trailer(input, trailer_at, &payload)?;
    let member_end = trailer_at + GZIP_TRAILER_LEN;
    if member_end < input.len() {
        let rest = &input[member_end..];
        if rest.starts_with(&GZIP_MAGIC) {
            return Err(SaveError::MultiMemberGzip);
        }
        return Err(SaveError::TrailingData { offset: rest.len() });
    }
    Ok(payload)
}

/// Parses optional gzip header fields and returns the offset of the DEFLATE stream.
fn skip_header_optional_fields(input: &[u8], flags: u8) -> Result<usize, SaveError> {
    let mut pos = GZIP_HEADER_MIN;
    let read_u16 = |input: &[u8], pos: usize| -> Result<u16, SaveError> {
        let bytes = input
            .get(pos..pos + 2)
            .ok_or(SaveError::GzipHeaderTruncated)?;
        Ok(u16::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    };
    if flags & FEXTRA != 0 {
        let extra_len = read_u16(input, pos)? as usize;
        pos = pos
            .checked_add(2 + extra_len)
            .filter(|&end| end <= input.len())
            .ok_or(SaveError::GzipHeaderTruncated)?;
    }
    for flag in [FNAME, FCOMMENT] {
        if flags & flag != 0 {
            let nul = input[pos..]
                .iter()
                .position(|&byte| byte == 0)
                .ok_or(SaveError::GzipHeaderTruncated)?;
            pos += nul + 1;
        }
    }
    if flags & FHCRC != 0 {
        let stored = read_u16(input, pos)?;
        // gzip header CRC16 is the low 16 bits of the CRC-32 of the header so far.
        let actual = (crc32fast::hash(&input[..pos]) & 0xFFFF) as u16;
        if stored != actual {
            return Err(SaveError::GzipHeaderChecksumMismatch);
        }
        pos += 2;
    }
    Ok(pos)
}

/// Validates the 8-byte gzip trailer (CRC-32 + ISIZE) at `trailer_at`.
fn validate_trailer(input: &[u8], trailer_at: usize, payload: &[u8]) -> Result<(), SaveError> {
    let trailer = input
        .get(trailer_at..trailer_at + GZIP_TRAILER_LEN)
        .ok_or(SaveError::DeflateTruncated)?;
    let expected_crc = u32::from_le_bytes(trailer[..4].try_into().expect("slice length checked"));
    let actual_crc = crc32fast::hash(payload);
    if expected_crc != actual_crc {
        return Err(SaveError::CrcMismatch {
            expected: expected_crc,
            actual: actual_crc,
        });
    }
    let expected_size = u32::from_le_bytes(trailer[4..8].try_into().expect("slice length checked"));
    let actual_size = payload.len() as u32;
    if expected_size != actual_size {
        return Err(SaveError::IsizeMismatch {
            expected: expected_size,
            actual: actual_size,
        });
    }
    Ok(())
}

/// Inflates a raw DEFLATE stream with a hard output cap, returning the
/// payload and the number of compressed bytes consumed (excluding trailer).
fn inflate_bounded(deflate: &[u8], max_output: usize) -> Result<(Vec<u8>, usize), SaveError> {
    let mut decompressor = Box::<DecompressorOxide>::default();
    let mut output = vec![0u8; initial_capacity(deflate.len(), max_output)];
    let mut in_pos = 0usize;
    let mut out_pos = 0usize;
    loop {
        let (status, in_consumed, out_consumed) = decompress(
            &mut decompressor,
            &deflate[in_pos..],
            &mut output,
            out_pos,
            TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
        );
        in_pos += in_consumed;
        out_pos += out_consumed;
        match status {
            TINFLStatus::Done => {
                output.truncate(out_pos);
                return Ok((output, in_pos));
            }
            TINFLStatus::HasMoreOutput => {
                if out_pos >= max_output {
                    return Err(SaveError::PayloadTooLarge { limit: max_output });
                }
                let new_len = output
                    .len()
                    .saturating_mul(2)
                    .max(out_pos + 1)
                    .min(max_output);
                output.resize(new_len, 0);
            }
            TINFLStatus::NeedsMoreInput => return Err(SaveError::DeflateTruncated),
            _ => return Err(SaveError::CorruptDeflate),
        }
    }
}

/// Initial output capacity: generous for small streams, never above the cap.
fn initial_capacity(input_len: usize, max_output: usize) -> usize {
    input_len
        .saturating_mul(2)
        .max(64 * 1024)
        .min(max_output)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a gzip member with stored (uncompressed) DEFLATE blocks.
    fn gzip_stored(payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0x1F, 0x8B, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xFF];
        let chunks: Vec<&[u8]> = payload.chunks(u16::MAX as usize).collect();
        if chunks.is_empty() {
            out.push(1); // Single final empty stored block.
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&(!0u16).to_le_bytes());
        }
        for (index, chunk) in chunks.iter().enumerate() {
            out.push((index == chunks.len() - 1) as u8); // BFINAL, BTYPE = stored.
            let len = chunk.len() as u16;
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&(!len).to_le_bytes());
            out.extend_from_slice(chunk);
        }
        out.extend_from_slice(&(crc32fast::hash(payload)).to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out
    }

    fn tiny_limits() -> Limits {
        Limits {
            max_compressed: 1024,
            max_payload: 4096,
        }
    }

    #[test]
    fn roundtrips_stored_member() {
        let payload = b"hello h3forge".to_vec();
        let member = gzip_stored(&payload);
        let got = decompress_single_member(&member, tiny_limits()).unwrap();
        assert_eq!(got, payload);
    }

    #[test]
    fn roundtrips_empty_member() {
        let member = gzip_stored(b"");
        let got = decompress_single_member(&member, tiny_limits()).unwrap();
        assert!(got.is_empty());
    }

    #[test]
    fn rejects_truncated_input() {
        let member = gzip_stored(b"hello h3forge");
        let error = decompress_single_member(&member[..7], tiny_limits()).unwrap_err();
        assert!(matches!(error, SaveError::GzipTruncated { .. }));
    }

    #[test]
    fn rejects_deflate_truncated_before_trailer() {
        let member = gzip_stored(b"hello h3forge");
        // Keep the header and stored-block header, drop the payload body.
        let cut = &member[..15];
        let error = decompress_single_member(cut, tiny_limits()).unwrap_err();
        assert!(matches!(
            error,
            SaveError::DeflateTruncated | SaveError::CorruptDeflate
        ));
    }

    #[test]
    fn rejects_bad_crc() {
        let mut member = gzip_stored(b"hello h3forge");
        let n = member.len();
        member[n - 6] ^= 0xFF; // Corrupt stored CRC.
        let error = decompress_single_member(&member, tiny_limits()).unwrap_err();
        assert!(matches!(error, SaveError::CrcMismatch { .. }));
    }

    #[test]
    fn rejects_bad_isize() {
        let mut member = gzip_stored(b"hello h3forge");
        let n = member.len();
        member[n - 1] ^= 0xFF; // Corrupt ISIZE.
        let error = decompress_single_member(&member, tiny_limits()).unwrap_err();
        assert!(matches!(error, SaveError::IsizeMismatch { .. }));
    }

    #[test]
    fn rejects_multi_member() {
        let mut member = gzip_stored(b"first");
        member.extend_from_slice(&gzip_stored(b"second"));
        let error = decompress_single_member(&member, tiny_limits()).unwrap_err();
        assert!(matches!(error, SaveError::MultiMemberGzip));
    }

    #[test]
    fn rejects_trailing_garbage() {
        let mut member = gzip_stored(b"first");
        member.extend_from_slice(b"garbage");
        let error = decompress_single_member(&member, tiny_limits()).unwrap_err();
        assert!(matches!(error, SaveError::TrailingData { .. }));
    }

    #[test]
    fn rejects_over_limit_compressed() {
        // Stored blocks do not compress: a 2000-byte payload yields a member
        // larger than the 1024-byte compressed limit.
        let member = gzip_stored(&vec![0u8; 2000]);
        let error = decompress_single_member(&member, tiny_limits()).unwrap_err();
        assert!(matches!(error, SaveError::InputTooLarge { .. }));
    }

    #[test]
    fn rejects_over_limit_payload() {
        // Member fits the compressed limit, payload exceeds the payload limit.
        let member = gzip_stored(&vec![7u8; 8192]);
        let limits = Limits {
            max_compressed: 16384,
            max_payload: 4096,
        };
        let error = decompress_single_member(&member, limits).unwrap_err();
        assert!(matches!(error, SaveError::PayloadTooLarge { .. }));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut member = gzip_stored(b"hello");
        member[0] = 0x1E;
        let error = decompress_single_member(&member, tiny_limits()).unwrap_err();
        assert!(matches!(error, SaveError::NotGzip));
    }

    #[test]
    fn honors_fhcrc() {
        // Hand-build a member with FNAME + FHCRC set.
        let payload = b"fhcrc payload".to_vec();
        let mut out = vec![0x1F, 0x8B, 0x08, FHCRC | FNAME, 0, 0, 0, 0, 0x00, 0xFF];
        out.extend_from_slice(b"name.txt\0");
        let crc16 = (crc32fast::hash(&out) & 0xFFFF) as u16;
        out.extend_from_slice(&crc16.to_le_bytes());
        for (index, chunk) in payload.chunks(u16::MAX as usize).enumerate() {
            out.push((index == payload.chunks(u16::MAX as usize).len() - 1) as u8);
            let len = chunk.len() as u16;
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&(!len).to_le_bytes());
            out.extend_from_slice(chunk);
        }
        out.extend_from_slice(&(crc32fast::hash(&payload)).to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        let got = decompress_single_member(&out, tiny_limits()).unwrap();
        assert_eq!(got, payload);
        // Corrupt the stored header CRC.
        let mut bad = out.clone();
        bad[19] ^= 0xFF;
        let error = decompress_single_member(&bad, tiny_limits()).unwrap_err();
        assert!(matches!(error, SaveError::GzipHeaderChecksumMismatch));
    }
}
