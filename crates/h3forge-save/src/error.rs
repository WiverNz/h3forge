//! Typed errors for save container reading, record parsing and edit planning.
//!
//! All failures are data-driven; no error path panics.

use std::fmt;

/// Failure while opening a save container or parsing its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SaveError {
    /// Compressed input exceeded the configured byte limit before parsing.
    InputTooLarge { limit: usize, actual: usize },
    /// Decompressed payload exceeded the configured byte limit.
    PayloadTooLarge { limit: usize },
    /// Input is too short to be a gzip member.
    GzipTruncated { needed: usize, actual: usize },
    /// gzip magic bytes `1F 8B` not present at the start of input.
    NotGzip,
    /// gzip header uses a compression method other than DEFLATE.
    GzipUnsupportedMethod { method: u8 },
    /// gzip header uses reserved flag bits.
    GzipUnsupportedFlags { flags: u8 },
    /// gzip header optional-field parsing ran past the input.
    GzipHeaderTruncated,
    /// FHCRC flag set but the stored header CRC16 does not match.
    GzipHeaderChecksumMismatch,
    /// DEFLATE stream ended before the gzip trailer.
    DeflateTruncated,
    /// DEFLATE payload is not decodable.
    CorruptDeflate,
    /// gzip trailer CRC-32 does not match the decompressed payload.
    CrcMismatch { expected: u32, actual: u32 },
    /// gzip trailer ISIZE does not match the decompressed payload length.
    IsizeMismatch { expected: u32, actual: u32 },
    /// More than one gzip member; not supported and never silently skipped.
    MultiMemberGzip,
    /// Non-gzip bytes follow the single gzip member.
    TrailingData { offset: usize },
    /// Decompressed payload does not carry a recognized save magic.
    UnknownFormat { found: Vec<u8> },
    /// Payload is too short to contain version identifiers.
    PayloadTooShort { needed: usize, actual: usize },
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::InputTooLarge { limit, actual } => {
                write!(
                    f,
                    "compressed input is {actual} bytes, over the {limit}-byte limit"
                )
            }
            SaveError::PayloadTooLarge { limit } => {
                write!(f, "decompressed payload exceeds the {limit}-byte limit")
            }
            SaveError::GzipTruncated { needed, actual } => {
                write!(
                    f,
                    "gzip input is {actual} bytes, shorter than the {needed}-byte header"
                )
            }
            SaveError::NotGzip => write!(f, "input does not start with the gzip magic 1F 8B"),
            SaveError::GzipUnsupportedMethod { method } => {
                write!(
                    f,
                    "gzip compression method {method} is not supported (expected 8 = DEFLATE)"
                )
            }
            SaveError::GzipUnsupportedFlags { flags } => {
                write!(f, "gzip header flag byte 0x{flags:02X} uses reserved bits")
            }
            SaveError::GzipHeaderTruncated => {
                write!(f, "gzip header optional fields run past the end of input")
            }
            SaveError::GzipHeaderChecksumMismatch => {
                write!(f, "gzip header CRC16 does not match the header contents")
            }
            SaveError::DeflateTruncated => {
                write!(
                    f,
                    "DEFLATE stream ends before producing the declared payload"
                )
            }
            SaveError::CorruptDeflate => write!(f, "DEFLATE stream is corrupt"),
            SaveError::CrcMismatch { expected, actual } => write!(
                f,
                "gzip CRC-32 mismatch: trailer has 0x{expected:08X}, payload computes to 0x{actual:08X}"
            ),
            SaveError::IsizeMismatch { expected, actual } => write!(
                f,
                "gzip ISIZE mismatch: trailer has {expected}, payload is {actual} bytes"
            ),
            SaveError::MultiMemberGzip => {
                write!(f, "multi-member gzip input is not supported")
            }
            SaveError::TrailingData { offset } => {
                write!(f, "{offset} unexpected bytes follow the gzip member")
            }
            SaveError::UnknownFormat { found } => write!(
                f,
                "payload magic {:?} is not a recognized Heroes III save format",
                String::from_utf8_lossy(found)
            ),
            SaveError::PayloadTooShort { needed, actual } => write!(
                f,
                "payload is {actual} bytes, shorter than the {needed}-byte version header"
            ),
        }
    }
}

impl std::error::Error for SaveError {}

/// Failure while planning or applying a save edit.
///
/// Every variant is a typed rejection before any write; the caller keeps the
/// original document, payload and pending edits untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditError {
    /// The container is recognized but this version/layout has no validated
    /// edit support; nothing may be patched.
    UnsupportedFormat {
        family: Option<&'static str>,
        major: u32,
        minor: u32,
    },
    /// No hero record with this ID exists in the document.
    HeroNotFound { id: usize },
    /// No town record with this ID exists in the document (H3F-013).
    TownNotFound { id: usize },
    /// The town record failed secondary verification, so it exposes no edit
    /// capability (ambiguous candidates stay read-only).
    TownNotEditable { id: usize },
    /// The record failed secondary verification, so it exposes no edit
    /// capability (ambiguous candidates stay read-only).
    RecordNotEditable { id: usize },
    /// A field value violates its documented game-version range.
    InvalidValue { field: &'static str, reason: String },
    /// A patch carried no bytes.
    EmptyPatch { offset: usize },
    /// A patch expected and replacement lengths differ.
    MismatchedPatchLength {
        offset: usize,
        expected_len: usize,
        replacement_len: usize,
    },
    /// Two patches cover overlapping byte ranges.
    OverlappingPatches { offset: usize },
    /// The payload no longer carries the bytes a patch was planned against
    /// (stale preimage); nothing was written.
    PreimageMismatch { offset: usize, reason: String },
    /// A patch range runs past the end of the payload.
    PatchOutOfBounds {
        offset: usize,
        len: usize,
        payload_len: usize,
    },
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EditError::UnsupportedFormat {
                family,
                major,
                minor,
            } => write!(
                f,
                "editing is not validated for {} {major}/{minor}",
                family.unwrap_or("unrecognized-family")
            ),
            EditError::HeroNotFound { id } => write!(f, "no hero record with ID {id}"),
            EditError::TownNotFound { id } => write!(f, "no town record with ID {id}"),
            EditError::TownNotEditable { id } => write!(
                f,
                "town record {id} failed structural verification and stays read-only"
            ),
            EditError::RecordNotEditable { id } => write!(
                f,
                "hero record {id} failed structural verification and stays read-only"
            ),
            EditError::InvalidValue { field, reason } => {
                write!(f, "invalid value for {field}: {reason}")
            }
            EditError::EmptyPatch { offset } => write!(f, "patch at {offset} carries no bytes"),
            EditError::MismatchedPatchLength {
                offset,
                expected_len,
                replacement_len,
            } => write!(
                f,
                "patch at {offset} has {expected_len} expected bytes but {replacement_len} replacement bytes"
            ),
            EditError::OverlappingPatches { offset } => {
                write!(f, "patches overlap at payload offset {offset}")
            }
            EditError::PreimageMismatch { offset, reason } => write!(
                f,
                "payload bytes at offset {offset} no longer match the planned preimage ({reason})"
            ),
            EditError::PatchOutOfBounds {
                offset,
                len,
                payload_len,
            } => write!(
                f,
                "patch at {offset} ({} bytes) runs past the {payload_len}-byte payload",
                len
            ),
        }
    }
}

impl std::error::Error for EditError {}
