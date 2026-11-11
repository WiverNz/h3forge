//! Original Heroes III player banners from CREST58.def.
//! Frame order 0-7 visually verified against local artwork: red, blue,
//! tan, green, orange, purple, teal, pink. The embedded legacy suffixes
//! do not name today's colors. Frame 8 is neutral and not a player slot.
use std::collections::HashMap;

use crate::{DefFile, LodArchive, ResourceError, ResourceLimits, encode_png};

/// Verified player colors: 0-7 inclusive.
pub const PLAYER_COLOR_MIN: u32 = 0;
pub const PLAYER_COLOR_MAX: u32 = 7;

/// Decoded player-flag cache over `H3sprite.lod/CREST58.def`.
#[derive(Debug)]
pub struct PlayerFlags {
    def: DefFile,
    cache: HashMap<u32, Vec<u8>>,
}

const FRAME_NAMES: [&str; 9] = [
    "crest0L.pcx",
    "crest1G.pcx",
    "crest2R.pcx",
    "crest3D.pcx",
    "crest4B.pcx",
    "crest5P.pcx",
    "crest6W.pcx",
    "crest7K.pcx",
    "crest8N.pcx",
];

impl PlayerFlags {
    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let bytes = archive.read_entry(archive.find_entry("CREST58.def")?)?;
        let def = DefFile::parse(bytes, ResourceLimits::default())?;
        if def.frames().len() != 9
            || def
                .frames()
                .iter()
                .enumerate()
                .any(|(i, f)| !f.name.eq_ignore_ascii_case(FRAME_NAMES[i]))
        {
            return Err(ResourceError::DefInvalid {
                reason: "unexpected player flag frame names",
            });
        }
        Ok(Self {
            def,
            cache: HashMap::new(),
        })
    }

    /// PNG for one player color, or `None` outside 0-7. Frames decode to
    /// the 58x64 banner canvas with palette index 0 transparent; only the
    /// requested frame is decoded and cached.
    pub fn png(&mut self, id: u32) -> Result<Option<&[u8]>, ResourceError> {
        if !(PLAYER_COLOR_MIN..=PLAYER_COLOR_MAX).contains(&id) {
            return Ok(None);
        }
        if !self.cache.contains_key(&id) {
            let image = self.def.decode_index(id as usize)?;
            if (image.width, image.height) != (58, 64) {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected player flag dimensions",
                });
            }
            self.cache.insert(id, encode_png(&image.to_rgba()));
        }
        Ok(self.cache.get(&id).map(Vec::as_slice))
    }
}
