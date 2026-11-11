//! Game installation discovery and validation.
//!
//! Read-only and strictly bounded: candidate directories come from the
//! executable's directory (portable layout), an explicitly configured
//! development root (debug builds only), or a previously stored selection;
//! the user can additionally point the picker at any directory. There is no
//! recursive drive search and nothing under an installation is ever written.
//!
//! Validation opens each known LOD archive through the regular bounded
//! reader (metadata only) and reports per-archive status, so a missing or
//! nonstandard archive never blocks the readable ones (research:
//! ARCHITECTURE.md "Resource integration and discovery").

use std::fmt;
use std::path::{Path, PathBuf};

use crate::ResourceError;
use crate::lod::LodArchive;

/// LOD archives probed inside an installation, in priority order. Only
/// archives actually present on disk are reported.
pub const KNOWN_ARCHIVES: &[&str] = &[
    "H3bitmap.lod",
    "H3sprite.lod",
    "h3ab_bmp.lod",
    "h3ab_spr.lod",
    "HotA.lod",
    "HotA_lng.lod",
    "HotA_ext.lod",
    "HotA_l_ext.lod",
];

/// Failure to find or validate an installation.
#[derive(Debug)]
#[non_exhaustive]
pub enum DiscoveryError {
    /// No candidate directory contained a readable game archive.
    NotFound { searched: Vec<PathBuf> },
    /// The directory exists but holds no readable LOD archive.
    NoArchives { path: PathBuf },
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoveryError::NotFound { searched } => write!(
                f,
                "no Heroes III installation found; searched {}",
                searched
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            DiscoveryError::NoArchives { path } => write!(
                f,
                "{} does not contain any readable LOD archive",
                path.display()
            ),
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// How one archive fared during validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveState {
    /// Opened read-only; carries this many directory entries.
    Readable { entries: usize },
    /// Present but rejected by directory validation (the supplied nonstandard
    /// `HotA.lod` lands here; see RESEARCH.md "HotA directory gap").
    Unsupported { reason: String },
    /// Present but unreadable (zero bytes, I/O failure, unknown format).
    Unreadable { reason: String },
}

/// Per-archive diagnostic for the frontend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveStatus {
    pub name: String,
    pub state: ArchiveState,
}

/// A validated read-only installation: the directory that directly contains
/// the LOD archives (`<root>` or `<root>/Data`).
#[derive(Debug, Clone)]
pub struct Installation {
    /// Directory whose archives were validated (either the selected root or
    /// its `Data` child - callers should display `root` for persistence).
    pub root: PathBuf,
    /// Archive diagnostics, in [`KNOWN_ARCHIVES`] order, missing files excluded.
    pub archives: Vec<ArchiveStatus>,
}

impl Installation {
    /// Detected resource sets, derived from which archives validated. A set
    /// is listed only when its evidence archive is readable.
    pub fn resource_sets(&self) -> Vec<&'static str> {
        let readable = |name: &str| {
            self.archives.iter().any(|status| {
                status.name == name && matches!(status.state, ArchiveState::Readable { .. })
            })
        };
        let mut sets = Vec::new();
        if readable("H3bitmap.lod") && readable("H3sprite.lod") {
            sets.push("Heroes III: Complete (SoD) artwork");
        }
        if readable("h3ab_bmp.lod") || readable("h3ab_spr.lod") {
            sets.push("Armageddon's Blade artwork");
        }
        if readable("HotA.lod") || readable("HotA_lng.lod") {
            sets.push("HotA artwork");
        }
        sets
    }

    /// True when at least the base bitmap archive is readable, i.e. hero
    /// portraits can be served.
    pub fn has_base_artwork(&self) -> bool {
        self.archives.iter().any(|status| {
            status.name == "H3bitmap.lod" && matches!(status.state, ArchiveState::Readable { .. })
        })
    }

    /// Full path of one archive inside the installation, when present.
    pub fn archive_path(&self, name: &str) -> Option<PathBuf> {
        let present = self.archives.iter().any(|status| status.name == name);
        present.then(|| self.root.join(name))
    }

    /// Opens the base bitmap archive read-only for the asset pipeline.
    pub fn open_bitmaps(&self) -> Result<LodArchive, ResourceError> {
        let path = self.archive_path("H3bitmap.lod").ok_or_else(|| {
            ResourceError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "H3bitmap.lod not present in the installation",
            ))
        })?;
        LodArchive::open(path)
    }
}

/// Validates a candidate directory (or its `Data` child) and, on success,
/// returns the installation description. This is the entry point for manual
/// selection and for re-validating a stored selection.
pub fn validate(root: impl AsRef<Path>) -> Result<Installation, DiscoveryError> {
    let root = root.as_ref();
    let candidates = [root.to_path_buf(), root.join("Data")];
    for dir in &candidates {
        if let Some(installation) = scan_dir(dir) {
            return Ok(installation);
        }
    }
    Err(DiscoveryError::NoArchives {
        path: root.to_path_buf(),
    })
}

/// Automatic discovery: portable executable directory, then the debug-only
/// configured development root, then the stored selection. The current
/// working directory is never consulted.
pub fn discover(stored: Option<&Path>) -> Result<Installation, DiscoveryError> {
    let mut searched = Vec::new();
    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1. Portable layout: the directory containing H3Forge.exe.
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.to_path_buf());
    }
    // 2. Explicitly configured development root (debug builds only, and never
    //    bundled: the path comes from the environment at runtime).
    #[cfg(debug_assertions)]
    if let Some(dev) = std::env::var_os("H3FORGE_DEV_DATA_ROOT") {
        candidates.push(PathBuf::from(dev));
    }
    // 3. Previously selected installation, re-validated before use.
    if let Some(path) = stored {
        candidates.push(path.to_path_buf());
    }

    for candidate in &candidates {
        searched.push(candidate.clone());
        if let Ok(installation) = validate(candidate) {
            return Ok(installation);
        }
    }
    Err(DiscoveryError::NotFound { searched })
}

/// Opens every known archive present directly in `dir`, read-only.
fn scan_dir(dir: &Path) -> Option<Installation> {
    if !dir.is_dir() {
        return None;
    }
    let mut archives = Vec::new();
    for name in KNOWN_ARCHIVES {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        let state = match LodArchive::open(&path) {
            Ok(archive) => ArchiveState::Readable {
                entries: archive.entries().len(),
            },
            Err(ResourceError::UnsupportedDirectory { reason }) => ArchiveState::Unsupported {
                reason: reason.to_string(),
            },
            Err(error) => ArchiveState::Unreadable {
                reason: error.to_string(),
            },
        };
        archives.push(ArchiveStatus {
            name: name.to_string(),
            state,
        });
    }
    if archives.is_empty() {
        None
    } else {
        Some(Installation {
            root: dir.to_path_buf(),
            archives,
        })
    }
}
