//! Bounded map discovery and save-to-map matching.
//!
//! Discovery lists `.h3m` files in explicitly given `Maps` directories and
//! at most two folder levels below them (the game's own map browser shows
//! sub-folders); it never walks anywhere else. Each candidate contributes
//! only its header, read from a bounded decompressed prefix.
//!
//! Matching is by the map's **internal** name, never the file name, and is
//! narrowed by every other header field a savegame retains: edge length,
//! underground flag, description bytes and difficulty. One survivor is a
//! match; several are reported as ambiguous for the user to choose from.

use std::path::{Path, PathBuf};

use crate::container::{MapLimits, decompress_prefix};
use crate::error::MapError;
use crate::model::MapHeader;
use crate::parse::parse_header;

/// Bounds for one discovery pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanLimits {
    /// Sub-folder levels below each root that are listed.
    pub max_depth: usize,
    /// Total `.h3m` files considered across all roots.
    pub max_files: usize,
    /// Decompressed bytes read per file to reach the end of the header.
    pub header_prefix: usize,
}

impl Default for ScanLimits {
    fn default() -> Self {
        ScanLimits {
            max_depth: 2,
            max_files: 5_000,
            header_prefix: 256 * 1024,
        }
    }
}

/// File identity used to reuse cached headers between scans.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileStamp {
    pub path: PathBuf,
    pub len: u64,
    pub modified_ms: i64,
}

impl FileStamp {
    pub fn capture(path: &Path) -> Option<FileStamp> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0);
        Some(FileStamp {
            path: path.to_path_buf(),
            len: metadata.len(),
            modified_ms,
        })
    }
}

/// One discovered map file with its parsed header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapCandidate {
    pub stamp: FileStamp,
    pub header: MapHeader,
}

/// Lists `.h3m` files under `root`, breadth-first, bounded by `limits`.
/// Unreadable folders are skipped; the result order is deterministic.
pub fn list_map_files(root: &Path, limits: ScanLimits) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut level = vec![root.to_path_buf()];
    for depth in 0..=limits.max_depth {
        let mut next = Vec::new();
        for dir in &level {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
            paths.sort();
            for path in paths {
                if path.is_dir() {
                    if depth < limits.max_depth {
                        next.push(path);
                    }
                } else if path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("h3m"))
                {
                    if files.len() >= limits.max_files {
                        return files;
                    }
                    files.push(path);
                }
            }
        }
        level = next;
    }
    files
}

/// Reads only the header of one map file.
pub fn read_map_header(
    path: &Path,
    map_limits: MapLimits,
    scan_limits: ScanLimits,
) -> Result<MapHeader, MapError> {
    let bytes = read_file_prefix(path, map_limits.max_file, scan_limits.header_prefix)?;
    let prefix = decompress_prefix(&bytes, scan_limits.header_prefix)?;
    match parse_header(&prefix, map_limits) {
        // A header longer than the prefix (huge descriptions) gets one full pass.
        Err(MapError::Truncated { .. }) => {
            let bytes = read_bounded(path, map_limits.max_file)?;
            let payload = crate::container::decompress_map(&bytes, map_limits)?;
            parse_header(&payload, map_limits)
        }
        other => other,
    }
}

/// Reads at most `prefix` bytes from the start of a regular file.
fn read_file_prefix(path: &Path, max: usize, prefix: usize) -> Result<Vec<u8>, MapError> {
    use std::io::Read;
    let io = |error: std::io::Error| MapError::Io(error.to_string());
    let file = std::fs::File::open(path).map_err(io)?;
    let metadata = file.metadata().map_err(io)?;
    if !metadata.is_file() {
        return Err(MapError::Io("not a regular file".into()));
    }
    if metadata.len() > max as u64 {
        return Err(MapError::TooLarge {
            what: "map file",
            limit: max,
            actual: metadata.len() as usize,
        });
    }
    let mut bytes = Vec::with_capacity(prefix.min(metadata.len() as usize));
    file.take(prefix as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    Ok(bytes)
}

/// Reads and fully parses one map file.
pub fn read_map(path: &Path, limits: MapLimits) -> Result<crate::ParsedMap, MapError> {
    let bytes = read_bounded(path, limits.max_file)?;
    let payload = crate::container::decompress_map(&bytes, limits)?;
    crate::parse::parse_map(&payload, limits)
}

fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>, MapError> {
    let metadata = std::fs::metadata(path).map_err(|error| MapError::Io(error.to_string()))?;
    if !metadata.is_file() {
        return Err(MapError::Io("not a regular file".into()));
    }
    if metadata.len() > max as u64 {
        return Err(MapError::TooLarge {
            what: "map file",
            limit: max,
            actual: metadata.len() as usize,
        });
    }
    std::fs::read(path).map_err(|error| MapError::Io(error.to_string()))
}

/// What a savegame retains about the map it was started from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveMapKey<'a> {
    pub name_raw: &'a [u8],
    pub description_raw: Option<&'a [u8]>,
    pub size: Option<u32>,
    pub has_underground: Option<bool>,
    pub difficulty: Option<u8>,
}

/// How much of the key a candidate agreed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchStrength {
    /// Name only: the save offered nothing else to compare.
    NameOnly,
    /// Name and every dimension field the save retains.
    NameAndDimensions,
    /// Name, dimensions and identical description bytes.
    Full,
}

/// Result of matching one save against a candidate list. Indexes refer to
/// the slice passed to [`match_candidates`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchOutcome {
    /// No candidate has the save's map name with compatible metadata.
    NotFound,
    /// Exactly one candidate survived.
    Unique {
        index: usize,
        strength: MatchStrength,
    },
    /// Several candidates are indistinguishable by header data.
    Ambiguous { indexes: Vec<usize> },
}

/// Grades one header against the key; `None` when any field conflicts.
pub fn match_strength(key: &SaveMapKey<'_>, header: &MapHeader) -> Option<MatchStrength> {
    if key.name_raw.is_empty() || header.name_raw != key.name_raw {
        return None;
    }
    if key.size.is_some_and(|size| size != header.size)
        || key
            .has_underground
            .is_some_and(|flag| flag != header.has_underground)
        || key
            .difficulty
            .is_some_and(|difficulty| difficulty != header.difficulty)
    {
        return None;
    }
    let description = key.description_raw.unwrap_or(&[]);
    let description_known = key.description_raw.is_some() || header.description_raw.is_empty();
    if description_known && description != header.description_raw.as_slice() {
        return None;
    }
    Some(
        if description_known && key.size.is_some() && key.has_underground.is_some() {
            MatchStrength::Full
        } else if key.size.is_some() && key.has_underground.is_some() {
            MatchStrength::NameAndDimensions
        } else {
            MatchStrength::NameOnly
        },
    )
}

/// Matches a save key against discovered headers.
pub fn match_candidates<'a>(
    key: &SaveMapKey<'_>,
    headers: impl IntoIterator<Item = &'a MapHeader>,
) -> MatchOutcome {
    let survivors: Vec<(usize, MatchStrength)> = headers
        .into_iter()
        .enumerate()
        .filter_map(|(index, header)| match_strength(key, header).map(|s| (index, s)))
        .collect();
    match survivors.as_slice() {
        [] => MatchOutcome::NotFound,
        [(index, strength)] => MatchOutcome::Unique {
            index: *index,
            strength: *strength,
        },
        many => MatchOutcome::Ambiguous {
            indexes: many.iter().map(|(index, _)| *index).collect(),
        },
    }
}
