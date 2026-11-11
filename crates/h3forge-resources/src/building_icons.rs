//! Town building icons by town type and building ID (H3F-022).
//!
//! The Town Hall screen draws one 150x70 picture per building from a
//! per-faction container with 44 frames (Factory: 45), where the frame
//! index is the building ID used by the savegame building masks
//! (0-4 Mage Guild, 5 Tavern, ... 30-36 dwellings, 37-43 upgraded dwellings).
//!
//! Evidence (2026-11-10, VERIFIED_IN_H3FORGE):
//!
//! * `H3sprite.lod` holds one DEF per original faction (`HALLCSTL`,
//!   `HALLRAMP`, `HallTowr`, `HALLINFR`, `HALLNECR`, `HallDung`, `HALLSTRN`,
//!   `hallfort`, `HallElem`). Their embedded frame names describe the
//!   building at each index (`ThCsMag1`..`ThCsMag4` at 0-3, `ThCsTav1` at 5,
//!   `ThCsCas1`-`3` at 7-9, `ThCsHal1`-`4` at 10-13, `ThCsPik1` at 30,
//!   `ThCsPik2` at 37, ...), which fixes index = building ID.
//! * `HotA.lod` holds `HALLCOVE` and `hallfact` as classic DEFs and
//!   `hallblwk` as a D32F container under opaque directory IDs. All frames
//!   were decoded and inspected: the same index layout, with the faction
//!   structures at the slots the game's own name tables give them (Cove 18
//!   Pub, 21 Gunpowder Warehouse, 23 Grotto, 24 Roost; Factory 6 Mana
//!   Generator, 22/23 Gantry; Bulwark 21/22 rune structures).
//! * Slots a faction does not use carry a filler: a copy of frame 0 (DEF
//!   containers) or a blank frame (Bulwark). Fillers resolve to `None`, so
//!   an unused slot never shows the Mage Guild picture.
//!
//! Unknown town types and building IDs resolve to `None`; there is no
//! fallback picture.

use std::collections::HashMap;

use crate::{
    DefFile, Hota32File, LodArchive, ResourceError, ResourceLimits, RgbaImage, encode_png,
};

/// Icon canvas of every hall container.
const ICON_SIZE: (u32, u32) = (150, 70);
/// Highest building ID with a hall picture.
pub const BUILDING_ID_MAX: u8 = 43;

/// `H3sprite.lod` hall containers by town type 0-8.
const BASE_HALLS: [&str; 9] = [
    "HALLCSTL.def",
    "HALLRAMP.def",
    "HallTowr.def",
    "HALLINFR.def",
    "HALLNECR.def",
    "HallDung.def",
    "HALLSTRN.def",
    "hallfort.def",
    "HallElem.def",
];

/// `HotA.lod` hall containers by town type 9-11: opaque directory ID and the
/// embedded frame-name stem that must match.
const HOTA_HALLS: [(&str, &str); 3] = [
    ("5769101d44726072eec526bc8cce5102", "HALLCOVE"),
    ("f76f7faf3112ab704b85de3719fead74", "hallfact"),
    ("6c77e5296feb30309e7d810483d3a9db", "hallblwk"),
];

fn limits() -> ResourceLimits {
    ResourceLimits {
        max_image_pixels: (ICON_SIZE.0 * ICON_SIZE.1) as usize,
        ..ResourceLimits::default()
    }
}

/// A frame that carries no picture of its own: uniform, or (for any index
/// but 0) a copy of the frame 0 picture.
fn is_filler(image: &RgbaImage, first: Option<&RgbaImage>) -> bool {
    let uniform = image
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| pixel[..] == image.data[..4]);
    uniform || first.is_some_and(|first| first.data == image.data)
}

fn checked(image: RgbaImage, error: ResourceError) -> Result<RgbaImage, ResourceError> {
    if (image.width, image.height) == ICON_SIZE {
        Ok(image)
    } else {
        Err(error)
    }
}

/// One decoded hall container: frame 0 for filler detection plus a PNG cache.
#[derive(Debug)]
struct Hall<T> {
    file: T,
    first: RgbaImage,
    cache: HashMap<u8, Option<Vec<u8>>>,
}

impl<T> Hall<T> {
    fn png(
        &mut self,
        building: u8,
        decode: impl Fn(&T, usize) -> Result<RgbaImage, ResourceError>,
    ) -> Result<Option<&[u8]>, ResourceError> {
        if building > BUILDING_ID_MAX {
            return Ok(None);
        }
        if !self.cache.contains_key(&building) {
            let image = decode(&self.file, building as usize)?;
            let first = (building != 0).then_some(&self.first);
            let png = (!is_filler(&image, first)).then(|| encode_png(&image));
            self.cache.insert(building, png);
        }
        Ok(self.cache.get(&building).and_then(|png| png.as_deref()))
    }
}

/// Building icons of the nine original factions from `H3sprite.lod`.
/// Containers are parsed on first use, one per town type.
#[derive(Debug, Default)]
pub struct BuildingIcons {
    halls: HashMap<u8, Hall<DefFile>>,
}

fn decode_def(def: &DefFile, index: usize) -> Result<RgbaImage, ResourceError> {
    checked(
        def.decode_index(index)?.to_rgba(),
        ResourceError::DefInvalid {
            reason: "unexpected building icon dimensions",
        },
    )
}

impl BuildingIcons {
    pub fn new() -> Self {
        Self::default()
    }

    /// PNG for one building of a town type 0-8; `None` for other types,
    /// IDs above 43 and slots the faction does not use.
    pub fn png(
        &mut self,
        archive: &LodArchive,
        town_type: u8,
        building: u8,
    ) -> Result<Option<&[u8]>, ResourceError> {
        let Some(name) = BASE_HALLS.get(town_type as usize) else {
            return Ok(None);
        };
        if let std::collections::hash_map::Entry::Vacant(slot) = self.halls.entry(town_type) {
            let file = DefFile::parse(archive.read_entry(archive.find_entry(name)?)?, limits())?;
            if file.frames().len() <= BUILDING_ID_MAX as usize {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected building icon frame count",
                });
            }
            let first = decode_def(&file, 0)?;
            slot.insert(Hall {
                file,
                first,
                cache: HashMap::new(),
            });
        }
        let hall = self.halls.get_mut(&town_type).expect("inserted above");
        hall.png(building, decode_def)
    }

    /// Whether a town type is served by the base archive.
    pub fn covers(town_type: u8) -> bool {
        (town_type as usize) < BASE_HALLS.len()
    }
}

#[derive(Debug)]
enum HotAHall {
    Def(Hall<DefFile>),
    D32f(Hall<Hota32File>),
}

/// Building icons of Cove, Factory and Bulwark (town types 9-11) from the
/// opaque `HotA.lod` directory.
#[derive(Debug, Default)]
pub struct HotABuildingIcons {
    halls: HashMap<u8, HotAHall>,
}

impl HotABuildingIcons {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a town type is served by the HotA archive.
    pub fn covers(town_type: u8) -> bool {
        (9..=11).contains(&town_type)
    }

    /// PNG for one building of a town type 9-11; `None` otherwise and for
    /// unused slots. A container whose frame names do not carry the expected
    /// stem is treated as unknown and yields no image.
    pub fn png(
        &mut self,
        archive: &LodArchive,
        town_type: u8,
        building: u8,
    ) -> Result<Option<&[u8]>, ResourceError> {
        if !Self::covers(town_type) {
            return Ok(None);
        }
        if let std::collections::hash_map::Entry::Vacant(slot) = self.halls.entry(town_type) {
            let (id, stem) = HOTA_HALLS[(town_type - 9) as usize];
            let data = archive.read_entry(archive.find_entry(id)?)?;
            let expected = format!("{stem}A000");
            let hall = if let Ok(file) = Hota32File::parse(data.clone(), limits()) {
                if file.frame_count() <= BUILDING_ID_MAX as usize
                    || file.frame_name_at(0) != Some(expected.as_str())
                {
                    return Ok(None);
                }
                let first = decode_d32f(&file, 0)?;
                HotAHall::D32f(Hall {
                    file,
                    first,
                    cache: HashMap::new(),
                })
            } else {
                let file = DefFile::parse(data, limits())?;
                if file.frames().len() <= BUILDING_ID_MAX as usize
                    || file.frames()[0].name != expected
                {
                    return Ok(None);
                }
                let first = decode_def(&file, 0)?;
                HotAHall::Def(Hall {
                    file,
                    first,
                    cache: HashMap::new(),
                })
            };
            slot.insert(hall);
        }
        match self.halls.get_mut(&town_type).expect("inserted above") {
            HotAHall::Def(hall) => hall.png(building, decode_def),
            HotAHall::D32f(hall) => hall.png(building, decode_d32f),
        }
    }
}

fn decode_d32f(file: &Hota32File, index: usize) -> Result<RgbaImage, ResourceError> {
    checked(
        file.decode_index(index)?,
        ResourceError::Hota32Invalid {
            reason: "unexpected building icon dimensions",
        },
    )
}
