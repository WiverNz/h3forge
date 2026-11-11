//! Typed errors for map reading. Malformed input never panics.

use std::fmt;

/// Everything that can go wrong while reading an `.h3m` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    /// File system failure (message only; maps are opened read-only).
    Io(String),
    /// The file or its decompressed payload exceeds a configured limit.
    TooLarge {
        what: &'static str,
        limit: usize,
        actual: usize,
    },
    /// The gzip wrapper is malformed or the DEFLATE stream is corrupt.
    Gzip(&'static str),
    /// The first word is not a known Heroes III map format.
    UnknownFormat { version: u32 },
    /// The format is recognized but this section is not implemented.
    Unsupported { what: String },
    /// A read ran past the end of the payload.
    Truncated { offset: usize, what: &'static str },
    /// A value is outside the range the format allows.
    Invalid { offset: usize, what: &'static str },
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapError::Io(message) => write!(f, "map file could not be read: {message}"),
            MapError::TooLarge {
                what,
                limit,
                actual,
            } => write!(f, "{what} is too large ({actual} > limit {limit})"),
            MapError::Gzip(reason) => write!(f, "map compression is invalid: {reason}"),
            MapError::UnknownFormat { version } => {
                write!(
                    f,
                    "not a known Heroes III map format (version word {version:#x})"
                )
            }
            MapError::Unsupported { what } => write!(f, "unsupported map content: {what}"),
            MapError::Truncated { offset, what } => {
                write!(
                    f,
                    "map data ends early at offset {offset} while reading {what}"
                )
            }
            MapError::Invalid { offset, what } => {
                write!(f, "invalid map data at offset {offset}: {what}")
            }
        }
    }
}

impl std::error::Error for MapError {}
