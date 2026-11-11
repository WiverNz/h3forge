//! Static town icons by town type (H3F-022).
//!
//! Two icon families exist: `ITPT` (58x64, hero/town screens) and `ITPA`
//! (48x32, adventure-map town list). Each town type has a fort and a
//! village (no fort) picture, each followed by its "built this turn"
//! variant, which H3Forge never selects.
//!
//! Evidence (2026-11-10, VERIFIED_IN_H3FORGE):
//!
//! * `H3sprite.lod` `ITPT.def` has 36 frames and `ITPA.def` 39. Their
//!   embedded frame names carry the faction stem (`itptcast`, `itptramp`,
//!   `itpttowr`, `itptinfr`, `itptnecr`, `itptdung`, `itptstrn`, `itptfort`,
//!   `itptelem`, in town type order 0-8) with suffix-less/`1` names for the
//!   fort picture and `3` for the village picture. Selection is by NAME.
//! * `HotA.lod` carries D32F replacements for both (48 and 51 frames, frame
//!   names `ITPT.defA000`...). Decoding every frame shows the same order
//!   extended to twelve types: fort picture at `2 * type` (`ITPA`: plus 2
//!   after the blank/unknown fillers), village block after all fort pairs
//!   (`ITPT` 24, `ITPA` 26), with Cove, Factory and Bulwark as types 9-11.
//!   The nine base pictures match the base DEF subjects.
//!
//! Unknown town types resolve to `None`; there is no fallback picture.

use std::collections::HashMap;

use crate::{DefFile, Hota32File, LodArchive, ResourceError, ResourceLimits, encode_png};

/// Which icon family to serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TownIconSize {
    /// `ITPA`, 48x32.
    Small,
    /// `ITPT`, 58x64.
    Large,
}

impl TownIconSize {
    fn dimensions(self) -> (u32, u32) {
        match self {
            TownIconSize::Small => (48, 32),
            TownIconSize::Large => (58, 64),
        }
    }
}

/// Highest town type with a base-game icon (Conflux).
pub const BASE_TOWN_TYPE_MAX: u8 = 8;
/// Highest town type with a HotA icon (Bulwark).
pub const HOTA_TOWN_TYPE_MAX: u8 = 11;

const LARGE_STEMS: [&str; 9] = [
    "cas", "ram", "tow", "inf", "nec", "dun", "str", "for", "ele",
];
const LARGE_FORT_NAMES: [&str; 9] = [
    "itptcast", "itptramp", "itpttowr", "itptinfr", "itptnecr", "itptdung", "itptstrn", "itptfort",
    "itptelem",
];
const SMALL_STEMS: [&str; 9] = [
    "cast", "ramp", "towr", "infr", "necr", "dung", "strg", "fort", "elem",
];

/// Embedded frame name of a base-game town icon, for town types 0-8.
pub fn town_icon_frame_name(town_type: u8, fort: bool, size: TownIconSize) -> Option<String> {
    let index = usize::from(town_type);
    if town_type > BASE_TOWN_TYPE_MAX {
        return None;
    }
    Some(match (size, fort) {
        (TownIconSize::Large, true) => format!("{}.pcx", LARGE_FORT_NAMES[index]),
        (TownIconSize::Large, false) => format!("itpt{}3.pcx", LARGE_STEMS[index]),
        (TownIconSize::Small, true) => format!("itp{}1.pcx", SMALL_STEMS[index]),
        (TownIconSize::Small, false) => format!("itp{}3.pcx", SMALL_STEMS[index]),
    })
}

/// Frame index of a town icon inside the HotA D32F containers, for town
/// types 0-11.
pub fn hota_town_icon_frame(town_type: u8, fort: bool, size: TownIconSize) -> Option<usize> {
    if town_type > HOTA_TOWN_TYPE_MAX {
        return None;
    }
    let pair = usize::from(town_type) * 2;
    Some(match (size, fort) {
        (TownIconSize::Large, true) => pair,
        (TownIconSize::Large, false) => 24 + pair,
        (TownIconSize::Small, true) => 2 + pair,
        (TownIconSize::Small, false) => 26 + pair,
    })
}

type IconKey = (u8, bool, TownIconSize);

/// Base-game town icons from `H3sprite.lod` (`ITPT.def`, `ITPA.def`).
#[derive(Debug)]
pub struct TownIcons {
    large: DefFile,
    small: DefFile,
    cache: HashMap<IconKey, Vec<u8>>,
}

impl TownIcons {
    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let parse = |name: &str, size: TownIconSize| {
            let (width, height) = size.dimensions();
            DefFile::parse(
                archive.read_entry(archive.find_entry(name)?)?,
                ResourceLimits {
                    max_image_pixels: (width * height) as usize,
                    ..ResourceLimits::default()
                },
            )
        };
        Ok(Self {
            large: parse("ITPT.def", TownIconSize::Large)?,
            small: parse("ITPA.def", TownIconSize::Small)?,
            cache: HashMap::new(),
        })
    }

    /// PNG for one town type 0-8, or `None` outside that range.
    pub fn png(
        &mut self,
        town_type: u8,
        fort: bool,
        size: TownIconSize,
    ) -> Result<Option<&[u8]>, ResourceError> {
        let Some(name) = town_icon_frame_name(town_type, fort, size) else {
            return Ok(None);
        };
        let key = (town_type, fort, size);
        if !self.cache.contains_key(&key) {
            let def = match size {
                TownIconSize::Large => &self.large,
                TownIconSize::Small => &self.small,
            };
            let image = def.decode_named(&name)?;
            if (image.width, image.height) != size.dimensions() {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected town icon dimensions",
                });
            }
            self.cache.insert(key, encode_png(&image.to_rgba()));
        }
        Ok(self.cache.get(&key).map(Vec::as_slice))
    }
}

/// Town icons from the HotA D32F containers in the opaque `HotA.lod`
/// directory; covers all twelve town types.
#[derive(Debug)]
pub struct HotATownIcons {
    large: Hota32File,
    small: Hota32File,
    cache: HashMap<IconKey, Vec<u8>>,
}

impl HotATownIcons {
    /// Opaque directory identifier of the `ITPT` container (58x64).
    pub const LARGE_RESOURCE_ID: &'static str = "f6f650f6c7c757b801daf48c5d7f2010";
    /// Opaque directory identifier of the `ITPA` container (48x32).
    pub const SMALL_RESOURCE_ID: &'static str = "f4fd5aaccf7268ccfd64d4d39649e87c";
    const LARGE_FRAMES: usize = 48;
    const SMALL_FRAMES: usize = 51;

    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let parse = |id: &str, size: TownIconSize, frames: usize| {
            let (width, height) = size.dimensions();
            let file = Hota32File::parse(
                archive.read_entry(archive.find_entry(id)?)?,
                ResourceLimits {
                    max_image_pixels: (width * height) as usize,
                    ..ResourceLimits::default()
                },
            )?;
            if file.frame_count() != frames {
                return Err(ResourceError::Hota32Invalid {
                    reason: "unexpected town icon frame count",
                });
            }
            Ok(file)
        };
        Ok(Self {
            large: parse(
                Self::LARGE_RESOURCE_ID,
                TownIconSize::Large,
                Self::LARGE_FRAMES,
            )?,
            small: parse(
                Self::SMALL_RESOURCE_ID,
                TownIconSize::Small,
                Self::SMALL_FRAMES,
            )?,
            cache: HashMap::new(),
        })
    }

    /// PNG for one town type 0-11. The frame must carry the expected
    /// embedded name, otherwise the layout is treated as unknown.
    pub fn png(
        &mut self,
        town_type: u8,
        fort: bool,
        size: TownIconSize,
    ) -> Result<Option<&[u8]>, ResourceError> {
        let Some(frame) = hota_town_icon_frame(town_type, fort, size) else {
            return Ok(None);
        };
        let (file, stem) = match size {
            TownIconSize::Large => (&self.large, "ITPT"),
            TownIconSize::Small => (&self.small, "ITPA"),
        };
        let expected = format!("{stem}.defA{frame:03}");
        if file.frame_name_at(frame) != Some(expected.as_str()) {
            return Ok(None);
        }
        let key = (town_type, fort, size);
        if let std::collections::hash_map::Entry::Vacant(entry) = self.cache.entry(key) {
            let image = file.decode_index(frame)?;
            if (image.width, image.height) != size.dimensions() {
                return Err(ResourceError::Hota32Invalid {
                    reason: "unexpected town icon dimensions",
                });
            }
            entry.insert(encode_png(&image));
        }
        Ok(self.cache.get(&key).map(Vec::as_slice))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_frame_names() {
        let name = |t, fort, size| town_icon_frame_name(t, fort, size).unwrap();
        assert_eq!(name(0, true, TownIconSize::Large), "itptcast.pcx");
        assert_eq!(name(0, false, TownIconSize::Large), "itptcas3.pcx");
        assert_eq!(name(6, true, TownIconSize::Large), "itptstrn.pcx");
        assert_eq!(name(8, false, TownIconSize::Large), "itptele3.pcx");
        assert_eq!(name(2, true, TownIconSize::Small), "itptowr1.pcx");
        assert_eq!(name(7, false, TownIconSize::Small), "itpfort3.pcx");
        assert_eq!(town_icon_frame_name(9, true, TownIconSize::Large), None);
    }

    #[test]
    fn hota_frame_indexes() {
        assert_eq!(hota_town_icon_frame(0, true, TownIconSize::Large), Some(0));
        assert_eq!(
            hota_town_icon_frame(11, true, TownIconSize::Large),
            Some(22)
        );
        assert_eq!(
            hota_town_icon_frame(9, false, TownIconSize::Large),
            Some(42)
        );
        assert_eq!(hota_town_icon_frame(0, true, TownIconSize::Small), Some(2));
        assert_eq!(
            hota_town_icon_frame(11, false, TownIconSize::Small),
            Some(48)
        );
        assert_eq!(hota_town_icon_frame(12, true, TownIconSize::Small), None);
    }
}
