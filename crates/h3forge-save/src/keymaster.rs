//! HotA 44/11 global Keymaster Tent masks. See SAVEGAME_FORMAT.md H3F-028.
use crate::{BytePatch, EditError, SaveDocument};

pub const TENT_COLORS: [&str; 8] = [
    "Light Blue",
    "Green",
    "Red",
    "Dark Blue",
    "Brown",
    "Purple",
    "White",
    "Black",
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TentEdit {
    pub player: u8,
    pub color: u8,
    pub visited: bool,
}

fn invalid(reason: &str) -> EditError {
    EditError::InvalidValue {
        field: "Keymaster Tent",
        reason: reason.into(),
    }
}

impl SaveDocument {
    fn tent_offset(&self) -> Option<usize> {
        // Re-derive from immutable bytes, never trust caller-modified public views.
        if !crate::detect::detect(self.payload())
            .ok()?
            .army_edit_supported()
        {
            return None;
        }
        let heroes = crate::locate::locate_heroes(self.payload());
        // Exact verified HotA roster. Other roster layouts remain unsupported.
        if heroes.len() != 215
            || heroes.iter().enumerate().any(|(i, h)| {
                !h.verified || self.payload().get(h.span.start + 10) != Some(&(i as u8))
            })
        {
            return None;
        }
        let owner = heroes.last()?.span.end.checked_add(12)?;
        let owners = self.payload().get(owner..owner + 215)?;
        if heroes.iter().zip(owners).any(|(h, &v)| {
            let stored = self.payload()[h.span.start];
            if stored < 8 { v != stored } else { v < 8 }
        }) {
            return None;
        }
        // owner[215], hireable[215], fixed globals[109], tents[8].
        let globals = owner.checked_add(430)?;
        let tail = self.payload().get(globals..globals + 130)?;
        // The verified fixed header: cheat bool and day/week/month u16.
        if tail[10] > 1 {
            return None;
        }
        let day = u16::from_le_bytes([tail[11], tail[12]]);
        let week = u16::from_le_bytes([tail[13], tail[14]]);
        if day > 7 || week > 4 {
            return None;
        }
        Some(globals + 109)
    }

    pub fn keymaster_masks(&self) -> Option<[u8; 8]> {
        let at = self.tent_offset()?;
        self.payload().get(at..at + 8)?.try_into().ok()
    }

    /// Plan all players together: one checked patch per changed color byte.
    /// No ally propagation, object removal, movement, fog or replay changes.
    pub fn plan_tent_edits(&self, edits: &[TentEdit]) -> Result<Vec<BytePatch>, EditError> {
        let at = self.tent_offset().ok_or_else(|| {
            invalid("Keymaster Tent layout is unverified; requires the complete HotA 44/11 roster.")
        })?;
        let original: [u8; 8] = self.payload()[at..at + 8]
            .try_into()
            .map_err(|_| invalid("Truncated tent table"))?;
        let mut masks = original;
        let mut seen = [[false; 8]; 8];
        for edit in edits {
            if edit.player > 7 || edit.color > 7 {
                return Err(invalid("Player and tent color must be 0-7."));
            }
            let c = edit.color as usize;
            let p = edit.player as usize;
            if seen[c][p] {
                return Err(invalid("Duplicate player/tent operation."));
            }
            seen[c][p] = true;
            let bit = 1u8 << p;
            masks[c] = if edit.visited {
                masks[c] | bit
            } else {
                masks[c] & !bit
            };
        }
        (0..8)
            .filter(|&c| masks[c] != original[c])
            .map(|c| {
                BytePatch::new(
                    at + c,
                    vec![original[c]],
                    vec![masks[c]],
                    format!("{} Keymaster Tent", TENT_COLORS[c]),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn private_current_tent_object_mapping() {
        let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
            eprintln!("skipping private tent object mapping");
            return;
        };
        let original = std::fs::read(
            std::path::PathBuf::from(root).join("SaAndVla_before_visit_red_tent.GM2"),
        )
        .unwrap();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.local/fixtures/keymaster-map");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("source.GM2"), original).unwrap();
        let doc =
            crate::SaveDocument::open(std::fs::read(dir.join("source.GM2")).unwrap()).unwrap();
        let (tiles, objects) = crate::neutral::current_map(&doc).unwrap();
        let tile = tiles[(252 + 131) * 252 + 142];
        assert!(
            objects
                .objects
                .iter()
                .any(|(pos, class, sub, _)| *pos == [142, 131, 1] && *class == 10 && *sub == 2)
        );
        let mut colors = [false; 8];
        for (name, raw) in &objects.templates {
            if u16::from_le_bytes([raw[26], raw[27]]) == 10 {
                let color = u16::from_le_bytes([raw[28], raw[29]]) as usize;
                assert!(color < 8);
                colors[color] = true;
                assert!(name.eq_ignore_ascii_case(format!("AVXkey{color}0.def").as_bytes()));
            }
        }
        assert!(colors.into_iter().all(|v| v));
        assert_eq!(
            u16::from_le_bytes([doc.payload()[tile + 8], doc.payload()[tile + 9]]),
            10
        );
    }
}
