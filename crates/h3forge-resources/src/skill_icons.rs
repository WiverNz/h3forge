//! Secondary skill artwork from H3sprite.lod/secsk32.def.
//! Mapping evidence (2026-11-09): VCMI CSkill::registerIcons uses
//! frame = 2 + mastery (1..3) + 3 * binary skill ID for SECSK32.
//! https://github.com/vcmi/vcmi/blob/develop/lib/CSkillHandler.cpp
//! Basic/Advanced/Expert use frame 2 + mastery + 3 * ID;
//! IDs 0..27 follow H3SED's binary secondary-skill table.
//! No VCMI implementation copied; only the resource mapping is used.
//! HotA-only skills (28 Interference, 29 Runes) have no secsk32.def
//! frames; their artwork lives in HotA.lod as a D32F container and is
//! served by [`HotASkillIcons`] (see `hota32.rs` for the pixel evidence).

use std::collections::HashMap;

use crate::{DefFile, Hota32File, LodArchive, ResourceError, ResourceLimits, encode_png};

/// Base secondary skill IDs; HotA-specific IDs are intentionally excluded.
pub const SKILL_ID_MIN: u32 = 0;
pub const SKILL_ID_MAX: u32 = 27;

/// Decoded skill-icon cache over `H3sprite.lod/secsk32.def`.
#[derive(Debug)]
pub struct SkillIcons {
    def: DefFile,
    cache: HashMap<(u32, u8), Vec<u8>>,
}

impl SkillIcons {
    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let bytes = archive.read_entry(archive.find_entry("secsk32.def")?)?;
        Ok(Self {
            def: DefFile::parse(
                bytes,
                ResourceLimits {
                    max_image_pixels: 32 * 32,
                    ..ResourceLimits::default()
                },
            )?,
            cache: HashMap::new(),
        })
    }

    /// PNG for one skill ID, or `None` outside 0-27. Frames decode to
    /// the 32x32 canvas with palette index 0 transparent; only the
    /// requested frame is decoded and cached.
    pub fn png(&mut self, id: u32, mastery: u8) -> Result<Option<&[u8]>, ResourceError> {
        let Some(frame) = skill_frame_index(id, mastery) else {
            return Ok(None);
        };
        let key = (id, mastery);
        if !self.cache.contains_key(&key) {
            let image = self.def.decode_index(frame)?;
            if (image.width, image.height) != (32, 32) {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected skill icon dimensions",
                });
            }
            self.cache.insert(key, encode_png(&image.to_rgba()));
        }
        Ok(self.cache.get(&key).map(Vec::as_slice))
    }
}

/// Mastery triplets after three blank frames. Never use display order.
pub fn skill_frame_index(id: u32, mastery: u8) -> Option<usize> {
    ((SKILL_ID_MIN..=SKILL_ID_MAX).contains(&id) && (1..=3).contains(&mastery))
        .then(|| 2 + mastery as usize + 3 * id as usize)
}

/// Frame number of one HotA-only skill icon inside the HotA.lod D32F
/// container. The formula continues the base-game secsk32.def numbering
/// (`2 + mastery + 3 * ID`): Interference (28) -> 87/88/89 and
/// Runes (29) -> 90/91/92, verified against decoded pixels (H3F-014,
/// `hota32.rs` module evidence).
pub fn hota_skill_frame_index(id: u32, mastery: u8) -> Option<usize> {
    ((28..=29).contains(&id) && (1..=3).contains(&mastery))
        .then(|| 2 + mastery as usize + 3 * id as usize)
}

/// Mastery-specific artwork for the two HotA-only secondary skills
/// (Interference 28, Runes 29) from the opaque `HotA.lod` directory.
///
/// Evidence (H3F-014, 2026-11-09): the artwork sits in directory entry
/// `48043b9164e06a6f8e626a6b6ca7caa1` as a D32F container whose frames are
/// named `secsk32.A000`..`secsk32.A092`. Frames 87-89 decode pixel-exactly
/// to the verified Interference Basic/Advanced/Expert references and
/// 90-92 to the Runes references (same alpha silhouettes, per-frame minimum
/// RGB distance; see `hota32.rs`). Frames are selected by exact embedded
/// name, never by directory position.
#[derive(Debug)]
pub struct HotASkillIcons {
    file: Hota32File,
    cache: HashMap<(u32, u8), Vec<u8>>,
}

impl HotASkillIcons {
    /// Opaque 16-byte directory identifier of the skill-icon container.
    pub const RESOURCE_ID: &'static str = "48043b9164e06a6f8e626a6b6ca7caa1";

    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let bytes = archive.read_entry(archive.find_entry(Self::RESOURCE_ID)?)?;
        Ok(Self {
            file: Hota32File::parse(bytes, ResourceLimits::default())?,
            cache: HashMap::new(),
        })
    }

    /// PNG for skill 28 or 29 at mastery 1..3; `None` for any other ID or
    /// mastery. Only the requested frame is decoded and cached.
    pub fn png(&mut self, id: u32, mastery: u8) -> Result<Option<&[u8]>, ResourceError> {
        let Some(frame) = hota_skill_frame_index(id, mastery) else {
            return Ok(None);
        };
        let name = format!("secsk32.A{frame:03}");
        let key = (id, mastery);
        if !self.cache.contains_key(&key) {
            let image = self.file.decode_named(&name)?;
            if (image.width, image.height) != (32, 32) {
                return Err(ResourceError::Hota32Invalid {
                    reason: "unexpected skill icon dimensions",
                });
            }
            self.cache.insert(key, encode_png(&image));
        }
        Ok(self.cache.get(&key).map(Vec::as_slice))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skill_mapping_and_unknown_fallback() {
        assert_eq!(skill_frame_index(0, 1), Some(3)); // Pathfinding
        assert_eq!(skill_frame_index(7, 2), Some(25)); // Wisdom
        assert_eq!(skill_frame_index(27, 3), Some(86)); // First Aid
        for mastery in [0, 4, 255] {
            assert_eq!(skill_frame_index(0, mastery), None);
        }
        for id in [28, 29, u32::MAX] {
            for mastery in 1..=3 {
                assert_eq!(skill_frame_index(id, mastery), None);
            }
        }
    }

    #[test]
    fn hota_skill_mapping_and_fallback() {
        // Continues the secsk32 formula past the base game's 28 skills.
        assert_eq!(hota_skill_frame_index(28, 1), Some(87));
        assert_eq!(hota_skill_frame_index(28, 3), Some(89));
        assert_eq!(hota_skill_frame_index(29, 1), Some(90));
        assert_eq!(hota_skill_frame_index(29, 3), Some(92));
        for (id, mastery) in [(27, 1), (30, 1), (28, 0), (29, 4)] {
            assert_eq!(hota_skill_frame_index(id, mastery), None);
        }
    }
}
