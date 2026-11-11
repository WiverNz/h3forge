//! Spell icons from `spells.def`, mapped by game spell ID (H3F-006).
//!
//! Evidence for the positional mapping (verified 2026-11-09):
//!
//! * `spells.def` carries 70 frames, one per game spell, in game spell ID
//!   order (0 = Summon Boat .. 69 = Summon Air Elemental; IDs per h3sed
//!   `metadata.IDS` == h3mtxt `SpellType`, already verified against the save
//!   flag arrays). Frame-name labels follow the same order with known
//!   rename trivia (Sp03Iden = Visions' icon under its development name
//!   "Identify", Sp37Back = Magic Mirror, Sp52Wind/Sp53Muck = Haste/Slow).
//! * VCMI loads the spellbook exactly this way -
//!   `client/windows/CSpellWindow.cpp` renders `CAnimImage("Spells")` with
//!   `setFrame(mySpell->id.getNum())`, and
//!   `Mods/vcmi/Content/Sprites/spells.json` documents the one known art bug:
//!   the View Earth (ID 3) and View Air (ID 5) pictures were drawn swapped.
//!   H3Forge reproduces the original game data as-is (no silent correction).
//! * `SpellInt.def`/`SpellScr.def`/`SpellBon.def` mirror the same order with
//!   a leading null frame and trailing creature-ability frames (spell IDs
//!   70-77); only `spells.def` is used here.
//!
//! Unknown IDs never fall back to a neighbouring frame: they resolve to
//! `None` and the UI shows a placeholder.

use std::collections::HashMap;

use crate::{DefFile, LodArchive, ResourceError, ResourceLimits, encode_png};

/// Game spell IDs with a frame in `spells.def`: 0-69 inclusive.
pub const SPELL_ID_MIN: u32 = 0;
pub const SPELL_ID_MAX: u32 = 69;

/// Decoded spell-icon cache over `H3sprite.lod/spells.def`.
#[derive(Debug)]
pub struct SpellIcons {
    def: DefFile,
    cache: HashMap<u32, Vec<u8>>,
}

impl SpellIcons {
    pub fn load(archive: &LodArchive) -> Result<Self, ResourceError> {
        let bytes = archive.read_entry(archive.find_entry("spells.def")?)?;
        Ok(Self {
            def: DefFile::parse(
                bytes,
                ResourceLimits {
                    max_image_pixels: 78 * 65,
                    ..ResourceLimits::default()
                },
            )?,
            cache: HashMap::new(),
        })
    }

    /// PNG for one game spell ID, or `None` outside 0-69. Frames decode to
    /// the 78x65 spellbook canvas with palette index 0 transparent; only the
    /// requested frame is decoded and cached.
    pub fn png(&mut self, id: u32) -> Result<Option<&[u8]>, ResourceError> {
        if !(SPELL_ID_MIN..=SPELL_ID_MAX).contains(&id) {
            return Ok(None);
        }
        if !self.cache.contains_key(&id) {
            let image = self.def.decode_index(id as usize)?;
            if (image.width, image.height) != (78, 65) {
                return Err(ResourceError::DefInvalid {
                    reason: "unexpected spell icon dimensions",
                });
            }
            self.cache.insert(id, encode_png(&image.to_rgba()));
        }
        Ok(self.cache.get(&id).map(Vec::as_slice))
    }
}
