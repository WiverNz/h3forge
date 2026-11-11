//! Bounded gzip unwrapping for `.h3m` files.
//!
//! Maps are a single gzip member around the raw map payload. Two entry
//! points: a full bounded decompression for parsing, and a prefix
//! decompression that stops after a fixed number of output bytes so
//! discovery can read hundreds of headers without inflating every terrain
//! block. The trailer is not verified for prefixes (it cannot be) and is
//! treated as advisory for full reads: maps are reference data, never
//! written back, and several stock maps carry trailing bytes.

use miniz_oxide::inflate::TINFLStatus;
use miniz_oxide::inflate::core::inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
use miniz_oxide::inflate::core::{DecompressorOxide, decompress};

use crate::error::MapError;

/// Size limits for one map file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapLimits {
    /// Maximum accepted file size on disk.
    pub max_file: usize,
    /// Maximum accepted decompressed payload.
    pub max_payload: usize,
    /// Maximum accepted length of one text field.
    pub max_text: usize,
    /// Maximum accepted object template and object counts.
    pub max_objects: usize,
}

impl Default for MapLimits {
    fn default() -> Self {
        MapLimits {
            max_file: 32 * 1024 * 1024,
            max_payload: 64 * 1024 * 1024,
            max_text: 512 * 1024,
            max_objects: 500_000,
        }
    }
}

const GZIP_MAGIC: [u8; 2] = [0x1F, 0x8B];

/// Returns the offset of the DEFLATE stream inside a gzip member.
fn deflate_start(input: &[u8]) -> Result<usize, MapError> {
    if input.len() < 18 {
        return Err(MapError::Gzip("file is shorter than a gzip header"));
    }
    if input[2] != 8 {
        return Err(MapError::Gzip("unsupported compression method"));
    }
    let flags = input[3];
    let mut pos = 10usize;
    if flags & 0x04 != 0 {
        let len = u16::from_le_bytes([input[pos], input[pos + 1]]) as usize;
        pos = pos
            .checked_add(2 + len)
            .filter(|&end| end < input.len())
            .ok_or(MapError::Gzip("extra field runs past the file"))?;
    }
    for flag in [0x08u8, 0x10] {
        if flags & flag != 0 {
            let nul = input[pos..]
                .iter()
                .position(|&byte| byte == 0)
                .ok_or(MapError::Gzip("unterminated header text"))?;
            pos += nul + 1;
        }
    }
    if flags & 0x02 != 0 {
        pos += 2;
    }
    if pos >= input.len() {
        return Err(MapError::Gzip("header runs past the file"));
    }
    Ok(pos)
}

/// Whether the bytes start with the gzip magic; uncompressed maps (rare,
/// produced by some tools) are accepted as-is by the callers.
pub fn is_gzip(input: &[u8]) -> bool {
    input.len() >= 2 && input[..2] == GZIP_MAGIC
}

/// Fully decompresses a map file, bounded by `limits.max_payload`.
pub fn decompress_map(input: &[u8], limits: MapLimits) -> Result<Vec<u8>, MapError> {
    if input.len() > limits.max_file {
        return Err(MapError::TooLarge {
            what: "map file",
            limit: limits.max_file,
            actual: input.len(),
        });
    }
    if !is_gzip(input) {
        return Ok(input.to_vec());
    }
    let start = deflate_start(input)?;
    // The gzip ISIZE trailer is a hint for the allocation only.
    let hinted = u32::from_le_bytes(
        input[input.len() - 4..]
            .try_into()
            .expect("length checked by deflate_start"),
    ) as usize;
    let mut capacity = hinted.max(64 * 1024).min(limits.max_payload.max(1));
    loop {
        let (status, produced, out) = inflate_into(&input[start..], capacity);
        match status {
            TINFLStatus::Done => {
                let mut out = out;
                out.truncate(produced);
                return Ok(out);
            }
            TINFLStatus::HasMoreOutput if capacity < limits.max_payload => {
                capacity = capacity.saturating_mul(2).min(limits.max_payload);
            }
            TINFLStatus::HasMoreOutput => {
                return Err(MapError::TooLarge {
                    what: "decompressed map",
                    limit: limits.max_payload,
                    actual: capacity.saturating_add(1),
                });
            }
            _ => return Err(MapError::Gzip("corrupt or truncated DEFLATE stream")),
        }
    }
}

/// Decompresses at most `max_prefix` bytes from the start of a map file.
/// Returns whatever was produced, even when the stream continues.
pub fn decompress_prefix(input: &[u8], max_prefix: usize) -> Result<Vec<u8>, MapError> {
    if !is_gzip(input) {
        return Ok(input[..input.len().min(max_prefix)].to_vec());
    }
    let start = deflate_start(input)?;
    let (status, produced, mut out) = inflate_into(&input[start..], max_prefix);
    match status {
        TINFLStatus::Done | TINFLStatus::HasMoreOutput | TINFLStatus::NeedsMoreInput => {}
        // A truncated file still yields its valid prefix.
        TINFLStatus::FailedCannotMakeProgress if produced > 0 => {}
        _ => return Err(MapError::Gzip("corrupt DEFLATE stream")),
    }
    out.truncate(produced);
    Ok(out)
}

fn inflate_into(deflate: &[u8], capacity: usize) -> (TINFLStatus, usize, Vec<u8>) {
    let mut out = vec![0u8; capacity];
    let mut state = DecompressorOxide::new();
    let (status, _consumed, produced) = decompress(
        &mut state,
        deflate,
        &mut out,
        0,
        TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
    );
    (status, produced, out)
}
