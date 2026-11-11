//! Hero portrait catalog: indexes the `HPL###XX.PCX` / `HPS###XX.PCX` /
//! `PS###XX.PCX` entry families of a bitmaps LOD by portrait ID.
//!
//! Evidence (H3F-005): the h3svg specification names the runtime hero fields
//! `hero_type` / `hero_class` / `hero_portrait`; the supplied archives carry
//! portrait entries numbered 000-136 per family with a two-letter class
//! suffix (e.g. `HPL000KN.PCX` = portrait 0, knight), and the game binary
//! references the same `HPL<number><class>.PCX` shape. The catalog resolves a
//! `portrait_id` from the save record to archive entries purely by directory
//! name - no class-name table is required and unknown suffixes are kept as-is.

use std::collections::BTreeMap;

use crate::lod::EntryInfo;

/// Which portrait size family an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortraitKind {
    /// `HPL###XX.PCX` - the large portrait (58x64 in the sample archives).
    Large,
    /// `HPS###XX.PCX` - small portrait variant.
    Small,
    /// `PS###XX.PCX` - second small portrait variant.
    SmallAlt,
}

/// One resolved portrait asset: entry indexes into the scanned archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortraitAsset {
    pub id: u16,
    /// `HPL###XX.PCX` entry index, when present.
    pub large: Option<usize>,
    /// `HPS###XX.PCX` and `PS###XX.PCX` entry indexes, in directory order.
    pub small: Vec<usize>,
}

/// Directory-derived portrait index of one LOD archive. Cheap to build: it
/// walks the already-parsed entry table only.
#[derive(Debug, Default)]
pub struct PortraitCatalog {
    portraits: BTreeMap<u16, PortraitAsset>,
}

impl PortraitCatalog {
    /// Indexes every `HPL/HPS/PS###XX.PCX` entry of the archive directory.
    /// Malformed names (wrong length, non-digits) are ignored; conflicting
    /// duplicates overwrite, which directory validation already bounds.
    pub fn scan(entries: &[EntryInfo]) -> Self {
        let mut catalog = PortraitCatalog::default();
        for (index, entry) in entries.iter().enumerate() {
            let Some((id, kind)) = parse_portrait_name(&entry.name) else {
                continue;
            };
            let slot = catalog
                .portraits
                .entry(id)
                .or_insert_with(|| PortraitAsset {
                    id,
                    large: None,
                    small: Vec::new(),
                });
            match kind {
                PortraitKind::Large => slot.large = Some(index),
                PortraitKind::Small | PortraitKind::SmallAlt => slot.small.push(index),
            }
        }
        catalog
    }

    /// All indexed portrait IDs, ascending.
    pub fn ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.portraits.keys().copied()
    }

    /// Resolve one portrait ID to its archive entries.
    pub fn resolve(&self, portrait_id: u16) -> Option<&PortraitAsset> {
        self.portraits.get(&portrait_id)
    }
}

/// Parses `HPL000KN.PCX`-style names: family prefix, three digits, two
/// suffix letters, `.PCX` extension (case-insensitive).
fn parse_portrait_name(name: &str) -> Option<(u16, PortraitKind)> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".pcx")?;
    let (kind, rest) = match stem.strip_prefix("hpl") {
        Some(rest) => (PortraitKind::Large, rest),
        None => match stem.strip_prefix("hps") {
            Some(rest) => (PortraitKind::Small, rest),
            None => (PortraitKind::SmallAlt, stem.strip_prefix("ps")?),
        },
    };
    let digits: String = rest.chars().take(3).collect();
    if digits.len() != 3 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let suffix: String = rest.chars().skip(3).collect();
    if suffix.len() != 2 || !suffix.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let id = digits.parse().ok()?;
    Some((id, kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> EntryInfo {
        EntryInfo {
            name: name.to_string(),
            offset: 0,
            stored_size: 1,
            unpacked_size: 1,
            packed: false,
            compression: 0,
        }
    }

    #[test]
    fn parses_portrait_names() {
        assert_eq!(
            parse_portrait_name("HPL000KN.PCX"),
            Some((0, PortraitKind::Large))
        );
        assert_eq!(
            parse_portrait_name("hps129mk.pcx"),
            Some((129, PortraitKind::Small))
        );
        assert_eq!(
            parse_portrait_name("PS058HR.PCX"),
            Some((58, PortraitKind::SmallAlt))
        );
        assert_eq!(parse_portrait_name("HPL12KN.PCX"), None);
        assert_eq!(parse_portrait_name("HPL000KN.DEF"), None);
        assert_eq!(parse_portrait_name("ADOPBPNL.PCX"), None);
        assert_eq!(parse_portrait_name("HPL000KNX.PCX"), None);
    }

    #[test]
    fn scans_and_resolves() {
        let entries = vec![
            entry("ADOPBPNL.PCX"),
            entry("HPL000KN.PCX"),
            entry("HPS000KN.PCX"),
            entry("PS000KN.PCX"),
            entry("HPL129MK.PCX"),
        ];
        let catalog = PortraitCatalog::scan(&entries);
        let asset = catalog.resolve(0).expect("portrait 0 indexed");
        assert_eq!(asset.large, Some(1));
        assert_eq!(asset.small, vec![2, 3]);
        assert!(catalog.resolve(129).is_some());
        assert!(catalog.resolve(7).is_none());
        assert_eq!(catalog.ids().collect::<Vec<_>>(), vec![0, 129]);
    }
}
