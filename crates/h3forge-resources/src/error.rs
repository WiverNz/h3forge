//! Typed errors for LOD archive reading and PCX decoding.
//!
//! All failures are data-driven; no error path panics. There is deliberately
//! no writer/edit error family: this crate offers no write API in H3F-004.

use std::fmt;

/// Failure while opening an archive, reading an entry or decoding an image.
#[derive(Debug)]
#[non_exhaustive]
pub enum ResourceError {
    UnsupportedCompression {
        index: usize,
        method: u8,
    },
    DefInvalid {
        reason: &'static str,
    },
    DefCompression {
        format: u32,
    },
    /// A HotA D32F image container is malformed or uses an unrecognized
    /// layout variant; decoding stops instead of guessing.
    Hota32Invalid {
        reason: &'static str,
    },
    /// Underlying file I/O failed (open, seek, read).
    Io(std::io::Error),
    /// The archive file is zero bytes; not a valid LOD.
    EmptyArchive,
    /// The archive does not start with the `LOD\0` signature.
    BadMagic,
    /// The version word does not identify a Heroes III-format LOD
    /// (MM6/7/8-format archives are out of scope for this crate).
    UnsupportedFormat {
        version: u32,
    },
    /// The header claims more entries than the configured limit.
    EntryCountLimit {
        limit: usize,
        claimed: u32,
    },
    /// The directory table runs past the end of the archive file.
    DirectoryTruncated {
        needed: u64,
        actual: u64,
    },
    /// The first directory record fails validation: the header decodes but
    /// its directory cannot be trusted. The archive is rejected, never half-enumerated.
    UnsupportedDirectory {
        reason: &'static str,
    },
    /// A directory record other than the first fails validation.
    InvalidDirectory {
        index: usize,
        reason: &'static str,
    },
    /// A name lookup matched more than one entry; the match is rejected as
    /// ambiguous instead of silently picking one.
    AmbiguousName {
        name: String,
    },
    /// No entry with the requested name exists (case-insensitive).
    EntryNotFound {
        name: String,
    },
    /// The entry ID is outside the directory.
    BadIndex {
        index: usize,
        entries: usize,
    },
    /// The stored entry data lies outside the archive file.
    EntryRange {
        index: usize,
        offset: u64,
        size: u64,
        file_len: u64,
    },
    /// The entry declares more stored or unpacked bytes than the limit.
    EntrySizeLimit {
        limit: usize,
        claimed: u32,
    },
    /// The declared unpacked size is larger than the packed data could ever
    /// produce (deflate tops out near 1032:1), so the header is lying.
    ImpossibleSize {
        index: usize,
        claimed_unpacked: u32,
        packed: u32,
    },
    /// The zlib stream ended before producing the declared unpacked size.
    DecompressTruncated {
        index: usize,
        got: usize,
        expected: usize,
    },
    /// The stored entry data is not a usable zlib stream.
    CorruptCompression {
        index: usize,
    },
    /// PCX input is shorter than the 12-byte header.
    PcxTruncated {
        needed: usize,
        actual: usize,
    },
    /// PCX width/height/image size fields are zero or negative.
    PcxBadDimensions {
        image_size: i32,
        width: i32,
        height: i32,
    },
    /// The PCX pixel data length is neither 8-bit nor 24-bit for the given
    /// dimensions.
    PcxBadPixelLength {
        image_size: usize,
        width: usize,
        height: usize,
    },
    /// An 8-bit PCX has no 768-byte palette behind its pixels.
    PcxMissingPalette {
        needed: usize,
        actual: usize,
    },
    /// The image exceeds the configured pixel limit.
    PixelLimit {
        limit: usize,
        claimed: usize,
    },
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceError::UnsupportedCompression { index, method } => write!(
                f,
                "entry {index} uses unsupported HotA compression method {method}"
            ),
            ResourceError::DefInvalid { reason } => write!(f, "invalid/unsupported DEF: {reason}"),
            ResourceError::Hota32Invalid { reason } => {
                write!(f, "invalid/unsupported HotA D32F image: {reason}")
            }
            ResourceError::DefCompression { format } => {
                write!(f, "unsupported DEF compression {format}")
            }
            ResourceError::Io(e) => write!(f, "archive I/O failed: {e}"),
            ResourceError::EmptyArchive => write!(f, "archive is zero bytes, not a valid LOD"),
            ResourceError::BadMagic => write!(f, "archive does not start with the LOD signature"),
            ResourceError::UnsupportedFormat { version } => write!(
                f,
                "archive version word 0x{version:08X} is not a Heroes III-format LOD"
            ),
            ResourceError::EntryCountLimit { limit, claimed } => write!(
                f,
                "directory claims {claimed} entries, over the {limit}-entry limit"
            ),
            ResourceError::DirectoryTruncated { needed, actual } => write!(
                f,
                "directory needs {needed} bytes but the archive is only {actual} bytes"
            ),
            ResourceError::UnsupportedDirectory { reason } => write!(
                f,
                "directory cannot be trusted and is not a supported layout: {reason}"
            ),
            ResourceError::InvalidDirectory { index, reason } => {
                write!(f, "directory record {index} is invalid: {reason}")
            }
            ResourceError::AmbiguousName { name } => {
                write!(
                    f,
                    "entry name {name:?} matches more than one directory record"
                )
            }
            ResourceError::EntryNotFound { name } => {
                write!(f, "no entry named {name:?} exists in the archive")
            }
            ResourceError::BadIndex { index, entries } => {
                write!(
                    f,
                    "entry index {index} is outside the {entries}-entry directory"
                )
            }
            ResourceError::EntryRange {
                index,
                offset,
                size,
                file_len,
            } => write!(
                f,
                "entry {index} claims {size} bytes at offset {offset}, past the end of the {file_len}-byte archive"
            ),
            ResourceError::EntrySizeLimit { limit, claimed } => write!(
                f,
                "entry declares {claimed} bytes, over the {limit}-byte limit"
            ),
            ResourceError::ImpossibleSize {
                index,
                claimed_unpacked,
                packed,
            } => write!(
                f,
                "entry {index} declares {claimed_unpacked} unpacked bytes, more than its {packed} packed bytes could produce"
            ),
            ResourceError::DecompressTruncated {
                index,
                got,
                expected,
            } => write!(
                f,
                "entry {index} zlib stream ended after {got} of the declared {expected} bytes"
            ),
            ResourceError::CorruptCompression { index } => {
                write!(f, "entry {index} is not a usable zlib stream")
            }
            ResourceError::PcxTruncated { needed, actual } => write!(
                f,
                "PCX data is {actual} bytes, shorter than the {needed}-byte header"
            ),
            ResourceError::PcxBadDimensions {
                image_size,
                width,
                height,
            } => write!(
                f,
                "PCX declares image size {image_size}, width {width}, height {height}; all must be positive"
            ),
            ResourceError::PcxBadPixelLength {
                image_size,
                width,
                height,
            } => write!(
                f,
                "PCX image size {image_size} is neither 8-bit nor 24-bit for {width}x{height}"
            ),
            ResourceError::PcxMissingPalette { needed, actual } => write!(
                f,
                "8-bit PCX needs a {needed}-byte palette but only {actual} bytes follow the pixels"
            ),
            ResourceError::PixelLimit { limit, claimed } => {
                write!(
                    f,
                    "image claims {claimed} pixels, over the {limit}-pixel limit"
                )
            }
        }
    }
}

impl std::error::Error for ResourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ResourceError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ResourceError {
    fn from(e: std::io::Error) -> Self {
        ResourceError::Io(e)
    }
}
