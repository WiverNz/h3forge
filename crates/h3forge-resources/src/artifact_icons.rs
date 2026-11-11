//! Verified artifact artwork, read from user-owned archives only.
//! Base HrArt names use legacy numbering, not save artifact IDs.
//! IDs 2-6 use names 122-126; IDs 7-126 use names 2-121.
//! HrArt141-143 in the base DEF are unavailable tiles and are excluded.
//! HotA additions use a separately verified, bounded D32F mapping below.

use std::collections::HashMap;

use crate::{DefFile, LodArchive, ResourceError, ResourceLimits, encode_png};

/// Highest artifact ID with a verified `HrArt###` frame.
pub const ARTIFACT_ID_MAX: u32 = 140;

/// Decoded artifact-icon cache over `H3sprite.lod/Artifact.def`.
#[derive(Debug)]
pub struct ArtifactIcons {
    def: DefFile,
    /// Artifact ID -> frame index, resolved from frame names at load.
    by_id: Vec<Option<usize>>,
    cache: HashMap<u32, Vec<u8>>,
}

impl ArtifactIcons {
    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let bytes = archive.read_entry(archive.find_entry("Artifact.def")?)?;
        let def = DefFile::parse(
            bytes,
            ResourceLimits {
                max_image_pixels: 44 * 44,
                ..ResourceLimits::default()
            },
        )?;
        let mut by_id = vec![None; ARTIFACT_ID_MAX as usize + 1];
        for (index, frame) in def.frames().iter().enumerate() {
            let Some(rest) = frame.name.strip_prefix("HrArt") else {
                continue;
            };
            let Ok(number) = rest.strip_suffix(".pcx").unwrap_or("").parse::<usize>() else {
                continue; // HrArtSel / HrArtNOT overlays.
            };
            if frame.name != format!("HrArt{number:03}.pcx") {
                continue;
            }
            let id = match number {
                0..=1 | 127..=140 => number,
                2..=121 => number + 5,
                122..=126 => number - 120,
                _ => continue,
            };
            if let Some(slot) = by_id.get_mut(id) {
                if slot.replace(index).is_some() {
                    return Err(ResourceError::DefInvalid {
                        reason: "duplicate artifact frame name",
                    });
                }
            }
        }
        Ok(Self {
            def,
            by_id,
            cache: HashMap::new(),
        })
    }

    /// PNG for one artifact ID, or `None` outside 0-140 (HotA-specific
    /// artifacts). Frames are 44x44 with palette index 0 transparent; only
    /// the requested frame is decoded and cached.
    pub fn png(&mut self, id: u32) -> Result<Option<&[u8]>, ResourceError> {
        if id > ARTIFACT_ID_MAX {
            return Ok(None);
        }
        if !self.cache.contains_key(&id) {
            let Some(index) = self.by_id[id as usize] else {
                return Ok(None);
            };
            let image = self.def.decode_index(index)?;
            if (image.width, image.height) != (44, 44) {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected artifact icon dimensions",
                });
            }
            self.cache.insert(id, encode_png(&image.to_rgba()));
        }
        Ok(self.cache.get(&id).map(Vec::as_slice))
    }
}

/// Verified HotA additions. Base HrArt141-143 are unavailable-artwork tiles,
/// while this D32F contains the actual Diplomat/Reflection/Ogre artwork.
/// Names 147-165 were visually crossed to the verified artifact catalog.
#[derive(Debug)]
pub struct HotAArtifactIcons {
    file: crate::Hota32File,
    cache: HashMap<u32, Vec<u8>>,
}
impl HotAArtifactIcons {
    pub const RESOURCE_ID: &'static str = "4ae18dc00814380facbef89026300713";
    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        Ok(Self {
            file: crate::Hota32File::parse(
                archive.read_entry(archive.find_entry(Self::RESOURCE_ID)?)?,
                ResourceLimits {
                    max_image_pixels: 44 * 44,
                    ..ResourceLimits::default()
                },
            )?,
            cache: HashMap::new(),
        })
    }
    pub fn png(&mut self, id: u32) -> Result<Option<&[u8]>, ResourceError> {
        if !matches!(id, 141..=143 | 147..=165) {
            return Ok(None);
        }
        if !self.cache.contains_key(&id) {
            let name = format!("artifactA{id:03}");
            if self
                .file
                .frame_names()
                .filter(|frame| *frame == name)
                .count()
                != 1
            {
                return Ok(None);
            }
            let image = self.file.decode_named(&name)?;
            if (image.width, image.height) != (44, 44) {
                return Err(ResourceError::Hota32Invalid {
                    reason: "unexpected artifact icon dimensions",
                });
            }
            self.cache.insert(id, encode_png(&image));
        }
        Ok(self.cache.get(&id).map(Vec::as_slice))
    }
}
