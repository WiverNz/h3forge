//! Original 32x32 resource icons, verified against local H3sprite.lod.
//! resource.def frames rsrc0.pcx through rsrc6.pcx visually match Wood,
//! Mercury, Ore, Sulfur, Crystal, Gems, Gold in h3svg ResourceType order.
//! Unknown layouts yield placeholders; no game artwork is bundled.
use std::collections::HashMap;

use crate::{DefFile, LodArchive, ResourceError, ResourceLimits, encode_png};

/// Verified resource IDs: 0-6 inclusive.
pub const RESOURCE_ID_MIN: u32 = 0;
pub const RESOURCE_ID_MAX: u32 = 6;

/// Decoded resource-icon cache over `H3sprite.lod/resource.def`.
#[derive(Debug)]
pub struct ResourceIcons {
    def: DefFile,
    cache: HashMap<u32, Vec<u8>>,
}

impl ResourceIcons {
    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let bytes = archive.read_entry(archive.find_entry("resource.def")?)?;
        let def = DefFile::parse(bytes, ResourceLimits::default())?;
        if def.frames().len() != 7
            || def
                .frames()
                .iter()
                .enumerate()
                .any(|(i, f)| !f.name.eq_ignore_ascii_case(&format!("rsrc{i}.pcx")))
        {
            return Err(ResourceError::DefInvalid {
                reason: "unexpected resource frame names",
            });
        }
        Ok(Self {
            def,
            cache: HashMap::new(),
        })
    }

    /// PNG for one resource ID, or `None` outside 0-6. Frames decode to
    /// the 32x32 resource canvas with palette index 0 transparent; only the
    /// requested frame is decoded and cached.
    pub fn png(&mut self, id: u32) -> Result<Option<&[u8]>, ResourceError> {
        if !(RESOURCE_ID_MIN..=RESOURCE_ID_MAX).contains(&id) {
            return Ok(None);
        }
        if !self.cache.contains_key(&id) {
            let image = self.def.decode_index(id as usize)?;
            if (image.width, image.height) != (32, 32) {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected resource icon dimensions",
                });
            }
            self.cache.insert(id, encode_png(&image.to_rgba()));
        }
        Ok(self.cache.get(&id).map(Vec::as_slice))
    }
}
