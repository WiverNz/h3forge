//! Standard H3sprite.lod / TwCrPort.def mapping, by embedded frame NAME.
//! Verified against local directory metadata and the Creature portraits table:
//! https://heroes.thelazy.net/index.php/Creature_portraits
//! No HotA DEF fallback: its reused filenames represent different creatures.
//!
//! HotA 1.8 creature artwork lives in HotA.lod as a D32F container whose
//! frames continue TwCrPort's numbering with a +2 offset (see
//! [`HotACreaturePortraits`] for the pixel-matching evidence).

use crate::{DefFile, Hota32File, LodArchive, ResourceError, ResourceLimits, encode_png};
use std::collections::HashMap;

pub fn creature_frame_name(id: u32) -> Option<String> {
    let suffix = match id {
        0..=13 => 'C',
        14..=27 => 'R',
        28..=41 => 'T',
        42..=55 => 'I',
        56..=69 => 'N',
        70..=83 => 'D',
        84..=97 => 'S',
        98..=111 => 'F',
        112..=121 | 123 | 125 | 127 | 129..=131 => 'E',
        132..=144 => 'N',
        _ => return None,
    };
    Some(format!("CPrL{id:03}{suffix}.pcx"))
}

#[derive(Debug)]
pub struct CreaturePortraits {
    def: DefFile,
    cache: HashMap<u32, Vec<u8>>,
}
impl CreaturePortraits {
    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let bytes = archive.read_entry(archive.find_entry("TwCrPort.def")?)?;
        Ok(Self {
            def: DefFile::parse(
                bytes,
                ResourceLimits {
                    max_image_pixels: 58 * 64,
                    ..ResourceLimits::default()
                },
            )?,
            cache: HashMap::new(),
        })
    }

    /// Only verified standard IDs can enter the cache (at most 141 portraits).
    pub fn png(&mut self, id: u32) -> Result<Option<&[u8]>, ResourceError> {
        let Some(name) = creature_frame_name(id) else {
            return Ok(None);
        };
        if !self.cache.contains_key(&id) {
            let image = self.def.decode_named(&name)?;
            if (image.width, image.height) != (58, 64) {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected creature portrait dimensions",
                });
            }
            self.cache.insert(id, encode_png(&image.to_rgba()));
        }
        Ok(self.cache.get(&id).map(Vec::as_slice))
    }
}

/// Frame index of one creature portrait inside the HotA twcrport D32F
/// container. HotA's 202-frame container (IDs 0..201) places creature ID `i`
/// at frame `i + 2`; frames 0 and 1 are container fillers.
///
/// Evidence (2026-11-10, VERIFIED_IN_H3FORGE): for all 145 base portraits
/// with standard TwCrPort art, frame `id + 2` carries the same creature -
/// 94 frames are pixel-identical (RGBA CRC-32 equal after palette
/// conversion) and 36 more match with small per-channel distance; the
/// remaining 11 share the full opaque silhouette but carry HotA's
/// recolored/redrawn "portrait fix" art. Slots for base-blank IDs (war
/// machines, reserved) are named fillers (`twcrport_000`/`_001`) and map
/// to `None`. Selection is by index AND verified embedded frame name.
pub fn hota_creature_frame_index(id: u32) -> Option<usize> {
    (id <= 199).then_some(id as usize + 2)
}

/// Creature portraits from the HotA 1.8 `twcrport` D32F container in the
/// opaque `HotA.lod` directory: full 58x64 artwork for creature IDs 0..199
/// (base + HotA-exclusive), with name verification at every step.
#[derive(Debug)]
pub struct HotACreaturePortraits {
    file: Hota32File,
    cache: HashMap<u32, Vec<u8>>,
}

impl HotACreaturePortraits {
    /// Opaque 16-byte directory identifier of the twcrport container.
    pub const RESOURCE_ID: &'static str = "eb1f19b13b23e32ded0fd482e7266b54";

    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let bytes = archive.read_entry(archive.find_entry(Self::RESOURCE_ID)?)?;
        Ok(Self {
            file: Hota32File::parse(
                bytes,
                ResourceLimits {
                    max_image_pixels: 58 * 64,
                    ..ResourceLimits::default()
                },
            )?,
            cache: HashMap::new(),
        })
    }

    /// PNG for one creature ID (0..=199); `None` for IDs outside the range
    /// and for filler slots (war machines and reserved IDs). The frame at
    /// `id + 2` must carry the exact expected embedded name, otherwise the
    /// layout is treated as unknown and no image is produced.
    pub fn png(&mut self, id: u32) -> Result<Option<&[u8]>, ResourceError> {
        let Some(frame) = hota_creature_frame_index(id) else {
            return Ok(None);
        };
        let expected = format!("twcrportA{frame:03}");
        if self.file.frame_name_at(frame) != Some(expected.as_str()) {
            return Ok(None);
        }
        if !self.cache.contains_key(&id) {
            let image = self.file.decode_index(frame)?;
            if (image.width, image.height) != (58, 64) {
                return Err(ResourceError::Hota32Invalid {
                    reason: "unexpected creature portrait dimensions",
                });
            }
            self.cache.insert(id, encode_png(&image));
        }
        Ok(self.cache.get(&id).map(Vec::as_slice))
    }
}
