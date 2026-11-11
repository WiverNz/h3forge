//! Save format and version detection.
//!
//! Ported from h3sed `version/__init__.py` and the per-version `detect()`
//! functions (MIT, copyright 2020 Erki Suurjaak; see licenses/h3sed-MIT.txt).
//! h3sed iterates RoE, AB, SoD, HC, HotA in release order and takes the first
//! match; the same order and byte-range semantics are used here. Unlike h3sed,
//! detection results are graded: a recognized family is `Readable`, never a
//! write capability, and unsupported versions stay read-only.

use crate::definitions::{MAGIC_SAVES, VERSION_MAJOR_OFFSET, VERSION_MINOR_OFFSET};
use crate::error::SaveError;

/// Recognized game families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameFamily {
    /// Restoration of Erathia, save major 16-41 minor 0.
    RestorationOfErathia,
    /// Armageddon's Blade, save major 42 minor 1.
    ArmageddonsBlade,
    /// Shadow of Death, save major 42-43 minor 0-4.
    ShadowOfDeath,
    /// Heroes Chronicles, `HCHRONSVG` magic.
    HeroesChronicles,
    /// Horn of the Abyss, save major >= 44 minor >= 5.
    HornOfTheAbyss,
}

impl GameFamily {
    /// Human-readable title, following h3sed `version.VERSIONS` titles.
    pub fn title(self) -> &'static str {
        match self {
            GameFamily::RestorationOfErathia => "Restoration of Erathia",
            GameFamily::ArmageddonsBlade => "Armageddon's Blade",
            GameFamily::ShadowOfDeath => "Shadow of Death",
            GameFamily::HeroesChronicles => "Heroes Chronicles",
            GameFamily::HornOfTheAbyss => "Horn of the Abyss",
        }
    }
}

/// Which save magic the container carries (h3sed `H3SV[GC]` plus the
/// Chronicles `HCHRONSVG` adaptation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    /// `H3SVG` regular save.
    Regular,
    /// `H3SVC` campaign save.
    Campaign,
    /// `HCHRONSVG` Heroes Chronicles save.
    Chronicles,
}

/// What the library can do with a detected format.
///
/// A recognized family only ever yields `Readable` today: parsing is
/// read-only and no version has validated edit support. `Verified` is
/// reserved for a future layout + operation gate and must not be inferred
/// from a successful parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compatibility {
    /// Layout and operations validated for editing. None yet.
    Verified,
    /// Recognized family; container and map text parse read-only.
    Readable,
    /// Magic recognized but version is unknown or unsupported.
    Unsupported,
}

impl Compatibility {
    /// Whether any modification may be offered. Always false for now;
    /// edits stay gated until a layout is validated per-version.
    pub fn is_editable(self) -> bool {
        matches!(self, Compatibility::Verified)
    }
}

/// Detected save identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedFormat {
    /// Recognized family; `None` when the magic is known but the version
    /// bytes match no researched family.
    pub family: Option<GameFamily>,
    pub major: u32,
    pub minor: u32,
    pub container_kind: ContainerKind,
    pub compatibility: Compatibility,
}

impl DetectedFormat {
    /// Whether the HotA hero record layout applies. Only HotA-family saves
    /// have fixture evidence for the 1121/1122-byte record locator; other
    /// recognized families keep their container `Readable` but skip hero
    /// scanning rather than pretend an unproven layout.
    pub fn hero_layout_supported(&self) -> bool {
        matches!(
            self.compatibility,
            Compatibility::Readable | Compatibility::Verified
        ) && matches!(self.family, Some(GameFamily::HornOfTheAbyss))
    }

    /// Army writes and creature definitions have evidence only for exact 44/11.
    pub fn army_edit_supported(&self) -> bool {
        self.hero_layout_supported() && self.major == 44 && self.minor == 11
    }

    /// Research title for the family, if recognized.
    pub fn family_title(&self) -> Option<&'static str> {
        self.family.map(GameFamily::title)
    }
}

/// Identifies the decompressed payload. Returns a typed error for an
/// unrecognized magic; a known magic with unknown version bytes yields
/// `Compatibility::Unsupported` so callers can still inspect map text.
pub fn detect(payload: &[u8]) -> Result<DetectedFormat, SaveError> {
    let kind = MAGIC_SAVES
        .iter()
        .find(|magic| payload.starts_with(magic))
        .map(|magic| match *magic {
            b"H3SVG" => ContainerKind::Regular,
            b"H3SVC" => ContainerKind::Campaign,
            _ => ContainerKind::Chronicles,
        });
    let Some(container_kind) = kind else {
        let found_len = payload.len().min(9);
        return Err(SaveError::UnknownFormat {
            found: payload[..found_len].to_vec(),
        });
    };
    if payload.len() < 16 {
        return Err(SaveError::PayloadTooShort {
            needed: 16,
            actual: payload.len(),
        });
    }
    // h3sed reads the single version bytes at offsets 8 and 12; observed
    // saves carry u32 little-endian fields at those offsets, which agree for
    // every released version value (research: RESEARCH.md "Actual sample").
    let major = u32::from_le_bytes(
        payload[VERSION_MAJOR_OFFSET..VERSION_MAJOR_OFFSET + 4]
            .try_into()
            .expect("slice length checked above"),
    );
    let minor = u32::from_le_bytes(
        payload[VERSION_MINOR_OFFSET..VERSION_MINOR_OFFSET + 4]
            .try_into()
            .expect("slice length checked above"),
    );

    // Release-order detection as in h3sed `version.detect()`: RoE, AB, SoD,
    // HC (magic-only, already established above), then HotA.
    let family = if container_kind == ContainerKind::Chronicles {
        Some(GameFamily::HeroesChronicles)
    } else {
        match (major, minor) {
            (16..=41, 0) => Some(GameFamily::RestorationOfErathia),
            (42, 1) => Some(GameFamily::ArmageddonsBlade),
            (42..=43, 0..=4) => Some(GameFamily::ShadowOfDeath),
            (44.., 5..) => Some(GameFamily::HornOfTheAbyss),
            _ => None,
        }
    };
    let Some(family) = family else {
        return Ok(DetectedFormat {
            family: None,
            major,
            minor,
            container_kind,
            compatibility: Compatibility::Unsupported,
        });
    };
    Ok(DetectedFormat {
        family: Some(family),
        major,
        minor,
        container_kind,
        compatibility: Compatibility::Readable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sample_version() {
        let mut payload = vec![0u8; 64];
        payload[..5].copy_from_slice(b"H3SVG");
        payload[8..12].copy_from_slice(&44u32.to_le_bytes());
        payload[12..16].copy_from_slice(&11u32.to_le_bytes());
        let format = detect(&payload).unwrap();
        assert_eq!(format.family, Some(GameFamily::HornOfTheAbyss));
        assert_eq!((format.major, format.minor), (44, 11));
        assert_eq!(format.container_kind, ContainerKind::Regular);
        assert_eq!(format.compatibility, Compatibility::Readable);
        assert!(format.hero_layout_supported());
        assert!(!format.compatibility.is_editable());
    }

    #[test]
    fn detects_release_order_boundaries() {
        // AB (42,1) wins over the overlapping SoD (42-43,0-4) range.
        for (major, minor, family) in [
            (16u32, 0u32, GameFamily::RestorationOfErathia),
            (41, 0, GameFamily::RestorationOfErathia),
            (42, 1, GameFamily::ArmageddonsBlade),
            (42, 0, GameFamily::ShadowOfDeath),
            (43, 4, GameFamily::ShadowOfDeath),
            (44, 5, GameFamily::HornOfTheAbyss),
            (44, 11, GameFamily::HornOfTheAbyss),
        ] {
            let mut payload = vec![0u8; 64];
            payload[..5].copy_from_slice(b"H3SVG");
            payload[8..12].copy_from_slice(&major.to_le_bytes());
            payload[12..16].copy_from_slice(&minor.to_le_bytes());
            let format = detect(&payload).unwrap();
            assert_eq!(format.family, Some(family), "major {major} minor {minor}");
            assert_eq!(format.compatibility, Compatibility::Readable);
        }
    }

    #[test]
    fn unknown_version_is_unsupported_not_an_error() {
        let mut payload = vec![0u8; 64];
        payload[..5].copy_from_slice(b"H3SVG");
        payload[8..12].copy_from_slice(&1u32.to_le_bytes());
        let format = detect(&payload).unwrap();
        assert_eq!(format.family, None);
        assert_eq!(format.compatibility, Compatibility::Unsupported);
        assert!(!format.hero_layout_supported());
    }

    #[test]
    fn chronicles_detected_by_magic() {
        let mut payload = vec![0u8; 64];
        payload[..9].copy_from_slice(b"HCHRONSVG");
        let format = detect(&payload).unwrap();
        assert_eq!(format.family, Some(GameFamily::HeroesChronicles));
        assert_eq!(format.container_kind, ContainerKind::Chronicles);
        // Hero scanning has no Chronicles layout evidence.
        assert!(!format.hero_layout_supported());
    }

    #[test]
    fn unknown_magic_errors() {
        let payload = vec![0u8; 64];
        let error = detect(&payload).unwrap_err();
        assert!(matches!(error, SaveError::UnknownFormat { .. }));
    }
}
