//! Original Keymaster Tent map sprites, verified against H3M class 10 subtypes.
use crate::{DefFile, LodArchive, ResourceError, ResourceLimits, encode_png};
use std::collections::HashMap;
#[derive(Debug)]
pub struct KeymasterIcons {
    frames: Vec<DefFile>,
    cache: HashMap<u32, Vec<u8>>,
}
impl KeymasterIcons {
    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let mut frames = Vec::new();
        for color in 0..8 {
            let name = format!("AVXkey{color}0");
            let bytes = archive.read_entry(archive.find_entry(&format!("{name}.def"))?)?;
            if bytes.get(..4) != Some(&[0x43, 0, 0, 0]) {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected tent DEF type",
                });
            }
            let def = DefFile::parse(bytes, ResourceLimits::default())?;
            if def.frames().len() != 1
                || !def.frames()[0]
                    .name
                    .eq_ignore_ascii_case(&format!("{name}.pcx"))
            {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected tent frame mapping",
                });
            }
            frames.push(def);
        }
        Ok(Self {
            frames,
            cache: HashMap::new(),
        })
    }
    pub fn png(&mut self, color: u32) -> Result<Option<&[u8]>, ResourceError> {
        if color > 7 {
            return Ok(None);
        }
        if !self.cache.contains_key(&color) {
            let image = self.frames[color as usize].decode_index(0)?;
            if (image.width, image.height) != (96, 64) {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected tent dimensions",
                });
            }
            let mut rgba = image.to_rgba();
            // Map-only shadow/selection indices are omitted for a compact icon.
            // Palette colors 8..255 and the original canvas remain unchanged.
            for (pixel, &index) in rgba.data.chunks_exact_mut(4).zip(&image.indices) {
                if index < 8 {
                    pixel[3] = 0;
                }
            }
            self.cache.insert(color, encode_png(&rgba));
        }
        Ok(self.cache.get(&color).map(Vec::as_slice))
    }
}
