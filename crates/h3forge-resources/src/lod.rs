//! Read-only access to Heroes III-format LOD archives.
//!
//! Adapted from mmarch (`mmarch-rust/src/lod.rs`, MIT, copyright Tom Chen;
//! snapshot commit `fc104f2a7d8b170ed67bc05b9bfac5d0bb9a60b8`; full notice in
//! `licenses/mmarch-MIT.txt`). Adaptations for H3Forge:
//!
//! * mmarch `LodArchive::load()` slurped the whole archive with `fs::read`;
//!   this reader opens `Read + Seek` on the file and touches only the 92-byte
//!   header and the 32-byte directory records at open time. Entry payloads are
//!   read on demand by `read_entry`.
//! * mmarch stopped the directory loop on a short table and indexed
//!   `entries[index]` directly; this version rejects a short or invalid
//!   directory up front, bounds every stored/unpacked size against configured
//!   limits before allocating, and validates entry IDs.
//! * HotA opaque-ID directories are validated at offset 80 with XOR words;
//!   stored/zlib entries are supported. Other compression methods fail on read.
//!   Format reference: homm3data (MIT, licenses/homm3data-MIT.txt); no Python runtime.
//! * No writer, no MM6/7/8 archive kinds, no automatic PCX conversion:
//!   `read_entry` returns raw decompressed bytes; PCX decoding is explicit in
//!   `crate::pcx`.
//! * The bounded zlib semantics of mmarch `zlib_decompress_bounded()`
//!   (stop at the declared unpacked size, tolerate trailing data, reject a
//!   declared size beyond what the packed data could produce) are preserved,
//!   reimplemented over the workspace's existing `miniz_oxide` core API.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use miniz_oxide::inflate::TINFLStatus;
use miniz_oxide::inflate::core::inflate_flags::{
    TINFL_FLAG_PARSE_ZLIB_HEADER, TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
};
use miniz_oxide::inflate::core::{DecompressorOxide, decompress};

use crate::error::ResourceError;

// Heroes III LOD header: "LOD\0", version u32, entry count u32, then reserved
// bytes up to 92. Directory records are 32 bytes:
// name[16], absolute data offset u32, unpacked size u32, unused u32,
// packed size u32 (0 = entry is stored uncompressed).
const H3_HEADER_SIZE: u64 = 92;
const H3_ENTRY_SIZE: u64 = 32;
const H3_NAME_SIZE: usize = 16;

/// Deflate cannot expand more than about 1032:1; a declared unpacked size
/// beyond that is a corrupt or hostile header, not an allocation to attempt
/// (adapted from mmarch `zlib_decompress_bounded()`).
const MAX_DEFLATE_RATIO: usize = 1032;

/// Resource limits enforced before allocations and decompression
/// (research: ARCHITECTURE.md "Binary, domain, operations and DTOs").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLimits {
    /// Maximum accepted directory entry count.
    pub max_entries: usize,
    /// Maximum accepted stored or unpacked bytes for a single entry.
    pub max_entry_bytes: usize,
    /// Maximum accepted pixels for one decoded image.
    pub max_image_pixels: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        ResourceLimits {
            max_entries: 100_000,
            max_entry_bytes: 32 * 1024 * 1024,
            max_image_pixels: 16_000_000,
        }
    }
}

/// One directory record of a Heroes III LOD archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryInfo {
    /// Standard ASCII name, or a HotA opaque identifier as 32 lowercase hex digits.
    pub name: String,
    /// Absolute offset of the stored data in the archive file.
    pub offset: u64,
    /// Stored size in bytes: packed size when compressed, else unpacked size.
    pub stored_size: u32,
    /// Decompressed size in bytes; equals `stored_size` when not packed.
    pub unpacked_size: u32,
    /// Whether the entry is compressed; inspect `compression` for the method.
    pub packed: bool,
    /// HotA method byte (0 stored, 3 zlib); standard entries use the same values.
    pub compression: u8,
}

/// A read-only Heroes III LOD archive opened for metadata-only directory
/// access. Payloads are read on demand; nothing is cached.
#[derive(Debug)]
pub struct LodArchive {
    file: File,
    entries: Vec<EntryInfo>,
    opaque_ids: bool,
}

impl LodArchive {
    /// Opens an archive with default limits.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, ResourceError> {
        Self::open_with_limits(path, ResourceLimits::default())
    }

    /// Opens an archive, reading and validating only the header and the
    /// directory table. No entry payload bytes are touched.
    pub fn open_with_limits(
        path: impl AsRef<std::path::Path>,
        limits: ResourceLimits,
    ) -> Result<Self, ResourceError> {
        let mut file = File::open(path)?;
        let file_len = file.metadata()?.len();
        if file_len == 0 {
            return Err(ResourceError::EmptyArchive);
        }
        if file_len < H3_HEADER_SIZE {
            return Err(ResourceError::DirectoryTruncated {
                needed: H3_HEADER_SIZE,
                actual: file_len,
            });
        }

        let mut header = [0u8; H3_HEADER_SIZE as usize];
        file.read_exact(&mut header)?;
        if &header[0..4] != b"LOD\0" {
            return Err(ResourceError::BadMagic);
        }
        let version = u32::from_le_bytes(header[4..8].try_into().expect("slice length checked"));
        if version > 0xFFFF {
            // MM6/7/8-format LODs carry a version string past the signature;
            // they are out of scope for this crate.
            return Err(ResourceError::UnsupportedFormat { version });
        }
        let count = u32::from_le_bytes(header[8..12].try_into().expect("slice length checked"));
        if count as usize > limits.max_entries {
            return Err(ResourceError::EntryCountLimit {
                limit: limits.max_entries,
                claimed: count,
            });
        }

        // HotA 1.8-style directory, independently validated on local archives.
        // Format evidence: homm3data/lodfile.py (MIT; licenses/homm3data-MIT.txt).
        // Standard H3sprite may contain the historical 0x007e0213 junk word.
        let key = u32::from_le_bytes(header[12..16].try_into().expect("fixed header"));
        // Prefer a structurally valid standard first record even with nonzero
        // reserved bytes, avoiding a key-only detector on ordinary archives.
        let mut first_standard = [0u8; 32];
        let standard_first = count > 0
            && file.read_exact(&mut first_standard).is_ok()
            && parse_entry_name(&first_standard[..16]).is_ok()
            && {
                let word = |at| {
                    u32::from_le_bytes(first_standard[at..at + 4].try_into().expect("fixed record"))
                };
                let stored = if word(28) == 0 { word(20) } else { word(28) };
                validate_entry_range(
                    0,
                    word(16) as u64,
                    stored,
                    word(20),
                    H3_HEADER_SIZE + count as u64 * H3_ENTRY_SIZE,
                    file_len,
                )
                .is_ok()
            };
        let opaque_ids = !standard_first && version == 200 && key != 0 && key != 0x007e0213;
        let directory_start = if opaque_ids { 80 } else { H3_HEADER_SIZE };
        let table_bytes = (count as u64) * H3_ENTRY_SIZE;
        let data_start = directory_start + table_bytes;
        if data_start > file_len {
            return Err(ResourceError::DirectoryTruncated {
                needed: data_start,
                actual: file_len,
            });
        }

        let mut table = vec![0u8; table_bytes as usize];
        file.seek(SeekFrom::Start(directory_start))?;
        file.read_exact(&mut table)?;

        let mut entries = Vec::with_capacity(count as usize);
        for (index, record) in table
            .as_chunks::<{ H3_ENTRY_SIZE as usize }>()
            .0
            .iter()
            .enumerate()
        {
            let name = if opaque_ids {
                record[..16].iter().map(|b| format!("{b:02x}")).collect()
            } else {
                parse_entry_name(&record[..H3_NAME_SIZE]).map_err(|reason| {
                    if index == 0 {
                        // The header decoded but the very first record is garbage:
                        // the directory uses a layout this crate does not support
                        // (the supplied nonstandard HotA.lod lands here).
                        ResourceError::UnsupportedDirectory { reason }
                    } else {
                        ResourceError::InvalidDirectory { index, reason }
                    }
                })?
            };

            let read_u32 = |at: usize| {
                u32::from_le_bytes(record[at..at + 4].try_into().expect("slice length checked"))
            };
            let (offset, unpacked_size, packed_size, compression) = if opaque_ids {
                (
                    (read_u32(16) ^ key) as u64,
                    read_u32(20) ^ key,
                    read_u32(24) ^ key,
                    record[28],
                )
            } else {
                (
                    read_u32(16) as u64,
                    read_u32(20),
                    read_u32(28),
                    if read_u32(28) == 0 { 0 } else { 3 },
                )
            };
            // Recognize but do not pretend to decode unverified compression methods.
            if opaque_ids
                && ((compression == 0 && packed_size != 0)
                    || (compression != 0 && packed_size == 0))
            {
                return Err(ResourceError::InvalidDirectory {
                    index,
                    reason: "compression flag and size disagree",
                });
            }
            let packed = packed_size != 0;
            let stored_size = if packed { packed_size } else { unpacked_size };

            validate_entry_range(
                index,
                offset,
                stored_size,
                unpacked_size,
                data_start,
                file_len,
            )?;
            if stored_size as usize > limits.max_entry_bytes {
                return Err(ResourceError::EntrySizeLimit {
                    limit: limits.max_entry_bytes,
                    claimed: stored_size,
                });
            }
            if packed && unpacked_size as usize > limits.max_entry_bytes {
                return Err(ResourceError::EntrySizeLimit {
                    limit: limits.max_entry_bytes,
                    claimed: unpacked_size,
                });
            }

            entries.push(EntryInfo {
                name,
                offset,
                stored_size,
                unpacked_size,
                packed,
                compression,
            });
        }
        debug_assert_eq!(entries.len(), count as usize);

        Ok(LodArchive {
            file,
            entries,
            opaque_ids,
        })
    }

    /// True when entry names are opaque 16-byte identifiers rendered as hex.
    pub fn has_opaque_ids(&self) -> bool {
        self.opaque_ids
    }

    /// All directory records in archive order.
    pub fn entries(&self) -> &[EntryInfo] {
        &self.entries
    }

    /// Index of the entry called `name`, matched case-insensitively. More
    /// than one match is rejected as ambiguous rather than silently picking
    /// one (research: ARCHITECTURE.md "Resource integration and discovery").
    pub fn find_entry(&self, name: &str) -> Result<usize, ResourceError> {
        let mut matches = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.name.eq_ignore_ascii_case(name));
        let Some((index, _)) = matches.next() else {
            return Err(ResourceError::EntryNotFound {
                name: name.to_string(),
            });
        };
        if matches.next().is_some() {
            return Err(ResourceError::AmbiguousName {
                name: name.to_string(),
            });
        }
        Ok(index)
    }

    /// Raw decompressed bytes of one entry, looked up by index from
    /// `entries()` or `find_entry`. Packed entries are zlib-decompressed into
    /// exactly their declared unpacked size; trailing bytes after the stream
    /// are ignored (adapted from mmarch `read_entry_data()`).
    pub fn read_entry(&self, index: usize) -> Result<Vec<u8>, ResourceError> {
        let entry = self.entries.get(index).ok_or(ResourceError::BadIndex {
            index,
            entries: self.entries.len(),
        })?;

        if entry.packed && entry.compression != 3 {
            return Err(ResourceError::UnsupportedCompression {
                index,
                method: entry.compression,
            });
        }
        let mut file = &self.file;
        file.seek(SeekFrom::Start(entry.offset))?;
        let mut stored = vec![0u8; entry.stored_size as usize];
        file.read_exact(&mut stored)?;

        if !entry.packed {
            return Ok(stored);
        }
        zlib_decompress_entry(
            index,
            &stored,
            entry.unpacked_size as usize,
            self.opaque_ids,
        )
    }
}

/// Validates a 16-byte fixed directory name: non-empty, NUL-terminated or
/// full-length, printable ASCII before the NUL.
fn parse_entry_name(raw: &[u8]) -> Result<String, &'static str> {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    if end == 0 {
        return Err("empty entry name");
    }
    if raw[..end].iter().any(|&b| !(0x20..=0x7E).contains(&b)) {
        return Err("entry name is not printable ASCII");
    }
    Ok(String::from_utf8(raw[..end].to_vec()).expect("ASCII is valid UTF-8"))
}

/// Bounds one directory record's data region against the archive file and the
/// end of the directory table itself.
fn validate_entry_range(
    index: usize,
    offset: u64,
    stored_size: u32,
    unpacked_size: u32,
    data_start: u64,
    file_len: u64,
) -> Result<(), ResourceError> {
    if offset < data_start
        || offset > file_len
        || stored_size as u64 > file_len.saturating_sub(offset)
    {
        return Err(if index == 0 {
            ResourceError::UnsupportedDirectory {
                reason: "entry data lies outside the archive or inside the directory table",
            }
        } else {
            ResourceError::InvalidDirectory {
                index,
                reason: "data offset/size outside the archive",
            }
        });
    }
    if stored_size == 0 && unpacked_size != 0 {
        return Err(ResourceError::InvalidDirectory {
            index,
            reason: "entry claims unpacked data but stores zero bytes",
        });
    }
    Ok(())
}

/// Bounded zlib decompression with mmarch `zlib_decompress_bounded()`
/// semantics: the declared unpacked size is the contract - decompress until
/// exactly `expected` bytes are produced, never verify or consume past it.
fn zlib_decompress_entry(
    index: usize,
    data: &[u8],
    expected: usize,
    strict: bool,
) -> Result<Vec<u8>, ResourceError> {
    if expected == 0 {
        return Ok(Vec::new());
    }
    // Never reserve a buffer the input could not possibly fill (mmarch).
    let ceiling = data
        .len()
        .saturating_mul(MAX_DEFLATE_RATIO)
        .saturating_add(1024);
    if expected > ceiling {
        return Err(ResourceError::ImpossibleSize {
            index,
            claimed_unpacked: expected as u32,
            packed: data.len() as u32,
        });
    }

    let mut decompressor = Box::<DecompressorOxide>::default();
    let mut output = vec![0u8; expected];
    // Single pass: the output buffer is exactly `expected` bytes, so the call
    // either fills it, finishes the stream, or fails - there is no resume.
    let flags = TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
    let (status, _, out_pos) = decompress(&mut decompressor, data, &mut output, 0, flags);
    match status {
        // Buffer exactly filled: all expected bytes are here, whatever the
        // stream would still have produced (mmarch stops the same way, so
        // palettes or padding behind the stream survive).
        TINFLStatus::HasMoreOutput if out_pos == expected && !strict => Ok(output),
        TINFLStatus::Done => {
            if out_pos == expected {
                Ok(output)
            } else {
                Err(ResourceError::DecompressTruncated {
                    index,
                    got: out_pos,
                    expected,
                })
            }
        }
        // Input exhausted before the stream ended: truncated entry.
        TINFLStatus::NeedsMoreInput | TINFLStatus::FailedCannotMakeProgress => {
            Err(ResourceError::DecompressTruncated {
                index,
                got: out_pos,
                expected,
            })
        }
        // out_pos < expected with a full buffer cannot happen (the buffer is
        // exactly `expected` bytes), so treat it as corrupt.
        _ => Err(ResourceError::CorruptCompression { index }),
    }
}
