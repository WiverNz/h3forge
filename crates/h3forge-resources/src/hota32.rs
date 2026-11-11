//! HotA "D32F" named-frame 32-bit image container.
//!
//! Evidence (2026-11-09, VERIFIED_IN_H3FORGE): HotA 1.8 `HotA.lod` stores
//! the secondary-skill artwork for the two HotA-only skills as a container
//! with magic `D32F` (little-endian word `0x46323344`): a fixed 32-byte
//! header, then group records of 13-byte frame names and 32-byte frame
//! records holding bottom-up BGRA pixels. Verified by decoding the skill
//! resource `48043b9164e06a6f8e626a6b6ca7caa1`: frames 87-89 match the VCMI
//! HotA `interference-{basic,advanced,expert}.png` references pixel-exactly
//! (RGBA SHA-256 equal), frames 90-92 match the Runes references by alpha
//! silhouette and per-frame minimum distance (the published reference PNGs
//! are re-brightened re-encodes of the same art). The frame numbering
//! continues the base-game `secsk32.def` formula `2 + mastery + 3 * id`:
//! Interference (28) -> 87/88/89, Runes (29) -> 90/91/92 for
//! Basic/Advanced/Expert. No third-party decoder was reused; the layout was
//! reverse-engineered from the local archive and cross-checked against
//! decoded pixels.

use crate::ResourceError;
use crate::pcx::RgbaImage;

const HEADER_SIZE: usize = 32;
const GROUP_HEADER_SIZE: usize = 16;
const NAME_SIZE: usize = 13;
const FRAME_HEADER_SIZE: usize = 40;
const MAGIC: u32 = 0x4632_3344; // "D32F"
const EXPECTED_VERSION: u32 = 1;
const EXPECTED_HEADER_WORD: u32 = 24;
const EXPECTED_GROUP_TRAILER: u32 = 8;

/// One frame directory record: embedded name and pixel-data location.
#[derive(Debug, Clone)]
struct FrameDir {
    name: String,
    offset: usize,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
}

/// Parsed D32F container: the raw entry bytes plus a validated frame table.
/// Pixels are decoded only for requested frames.
#[derive(Debug)]
pub struct Hota32File {
    data: Vec<u8>,
    canvas_width: usize,
    canvas_height: usize,
    frames: Vec<FrameDir>,
}

impl Hota32File {
    /// Parses and fully bounds-checks the container; no pixel is touched.
    pub fn parse(data: Vec<u8>, limits: crate::ResourceLimits) -> Result<Self, ResourceError> {
        if data.len() < HEADER_SIZE {
            return Err(ResourceError::Hota32Invalid {
                reason: "shorter than the 32-byte header",
            });
        }
        let word = |at: usize| u32::from_le_bytes(data[at..at + 4].try_into().expect("bounded"));
        if word(0) != MAGIC {
            return Err(ResourceError::Hota32Invalid {
                reason: "missing D32F magic",
            });
        }
        if word(4) != EXPECTED_VERSION || word(8) != EXPECTED_HEADER_WORD {
            return Err(ResourceError::Hota32Invalid {
                reason: "unrecognized header variant",
            });
        }
        let canvas_width = word(12) as usize;
        let canvas_height = word(16) as usize;
        if canvas_width == 0
            || canvas_height == 0
            || canvas_width
                .checked_mul(canvas_height)
                .is_none_or(|pixels| pixels > limits.max_image_pixels)
        {
            return Err(ResourceError::Hota32Invalid {
                reason: "canvas dimensions are zero or over the pixel limit",
            });
        }
        let groups = word(20) as usize;
        if word(24) != EXPECTED_GROUP_TRAILER {
            return Err(ResourceError::Hota32Invalid {
                reason: "unrecognized group trailer word",
            });
        }
        if groups == 0 || groups > data.len().saturating_sub(HEADER_SIZE) / GROUP_HEADER_SIZE {
            return Err(ResourceError::Hota32Invalid {
                reason: "group count exceeds the remaining bytes",
            });
        }

        let mut frames = Vec::new();
        let mut at = HEADER_SIZE;
        for _ in 0..groups {
            if at + GROUP_HEADER_SIZE > data.len() {
                return Err(ResourceError::Hota32Invalid {
                    reason: "group header truncated",
                });
            }
            let record_size = word(at) as usize;
            let count = word(at + 8) as usize;
            if word(at + 12) != 4 || record_size != GROUP_HEADER_SIZE + 17 * count {
                return Err(ResourceError::Hota32Invalid {
                    reason: "group record size does not match its frame count",
                });
            }
            if count > (data.len() - at - GROUP_HEADER_SIZE) / 17 {
                return Err(ResourceError::Hota32Invalid {
                    reason: "frame count exceeds the remaining bytes",
                });
            }
            let names = at + GROUP_HEADER_SIZE;
            let offsets = names + NAME_SIZE * count;
            let group_end = offsets + 4 * count;
            if group_end > data.len() {
                return Err(ResourceError::Hota32Invalid {
                    reason: "group table truncated",
                });
            }
            for index in 0..count {
                let name = parse_frame_name(
                    &data[names + index * NAME_SIZE..names + (index + 1) * NAME_SIZE],
                )?;
                let offset = word(offsets + 4 * index) as usize;
                let dir = parse_frame_record(
                    &data,
                    offset,
                    canvas_width,
                    canvas_height,
                    limits.max_image_pixels,
                    name,
                )?;
                frames.push(dir);
            }
            at = group_end;
        }
        if frames.is_empty() {
            return Err(ResourceError::Hota32Invalid {
                reason: "container holds no frames",
            });
        }
        Ok(Hota32File {
            data,
            canvas_width,
            canvas_height,
            frames,
        })
    }

    /// Names of all frames in directory order.
    pub fn frame_names(&self) -> impl Iterator<Item = &str> {
        self.frames.iter().map(|frame| frame.name.as_str())
    }

    /// Number of frames in the container.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Name of the frame at `index` in directory order, when in range.
    pub fn frame_name_at(&self, index: usize) -> Option<&str> {
        self.frames.get(index).map(|frame| frame.name.as_str())
    }

    /// Decodes the frame at `index` in directory order. Callers that care
    /// about identity must verify `frame_name_at(index)` first: duplicate
    /// filler names make name lookup alone ambiguous.
    pub fn decode_index(&self, index: usize) -> Result<RgbaImage, ResourceError> {
        let frame = self.frames.get(index).ok_or(ResourceError::BadIndex {
            index,
            entries: self.frames.len(),
        })?;
        self.decode_frame(frame)
    }

    /// Decodes one frame onto the full canvas (unpainted pixels are
    /// transparent). Pixels are stored bottom-up BGRA; the returned image is
    /// top-down RGBA. Frames are selected by exact embedded name.
    pub fn decode_named(&self, name: &str) -> Result<RgbaImage, ResourceError> {
        let frame = self
            .frames
            .iter()
            .find(|frame| frame.name == name)
            .ok_or_else(|| ResourceError::EntryNotFound {
                name: name.to_string(),
            })?;
        self.decode_frame(frame)
    }

    /// Shared pixel composition for named and indexed decode.
    fn decode_frame(&self, frame: &FrameDir) -> Result<RgbaImage, ResourceError> {
        let pixels = frame.width * frame.height * 4;
        let start = frame.offset + FRAME_HEADER_SIZE;
        let stored = &self.data[start..start + pixels];
        let mut rgba = vec![0u8; self.canvas_width * self.canvas_height * 4];
        for yy in 0..frame.height {
            for xx in 0..frame.width {
                let src = ((frame.height - 1 - yy) * frame.width + xx) * 4;
                let dst = ((frame.y + yy) * self.canvas_width + (frame.x + xx)) * 4;
                rgba[dst..dst + 4].copy_from_slice(&[
                    stored[src + 2],
                    stored[src + 1],
                    stored[src],
                    stored[src + 3],
                ]);
            }
        }
        Ok(RgbaImage {
            width: self.canvas_width as u32,
            height: self.canvas_height as u32,
            data: rgba,
        })
    }
}

/// Validates a 13-byte frame name: NUL-terminated or full-length printable
/// ASCII, like the standard LOD directory names.
fn parse_frame_name(raw: &[u8]) -> Result<String, ResourceError> {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    if end == 0 {
        return Err(ResourceError::Hota32Invalid {
            reason: "empty frame name",
        });
    }
    if raw[..end].iter().any(|&b| !(0x20..=0x7E).contains(&b)) {
        return Err(ResourceError::Hota32Invalid {
            reason: "frame name is not printable ASCII",
        });
    }
    Ok(String::from_utf8(raw[..end].to_vec()).expect("ASCII is valid UTF-8"))
}

/// Bounds one frame record against the container: 40-byte header, pixel
/// word count, and placement inside the canvas.
fn parse_frame_record(
    data: &[u8],
    offset: usize,
    canvas_width: usize,
    canvas_height: usize,
    max_image_pixels: usize,
    name: String,
) -> Result<FrameDir, ResourceError> {
    let word = |at: usize| -> Result<usize, ResourceError> {
        let end = at.checked_add(4).ok_or(ResourceError::Hota32Invalid {
            reason: "frame record offset overflows",
        })?;
        let bytes = data.get(at..end).ok_or(ResourceError::Hota32Invalid {
            reason: "frame record truncated",
        })?;
        Ok(u32::from_le_bytes(bytes.try_into().expect("bounded")) as usize)
    };
    if word(offset)? != FRAME_HEADER_SIZE - 8 {
        return Err(ResourceError::Hota32Invalid {
            reason: "frame record header size mismatch",
        });
    }
    let (full_w, full_h) = (word(offset + 8)?, word(offset + 12)?);
    let (width, height) = (word(offset + 16)?, word(offset + 20)?);
    let (x, y) = (word(offset + 24)?, word(offset + 28)?);
    if full_w != canvas_width || full_h != canvas_height {
        return Err(ResourceError::Hota32Invalid {
            reason: "frame canvas does not match the container canvas",
        });
    }
    if width == 0 || height == 0 || width * height > max_image_pixels {
        return Err(ResourceError::Hota32Invalid {
            reason: "frame dimensions are zero or over the pixel limit",
        });
    }
    if word(offset + 4)? != width * height * 4 {
        return Err(ResourceError::Hota32Invalid {
            reason: "frame pixel byte count does not match its dimensions",
        });
    }
    if x + width > canvas_width || y + height > canvas_height {
        return Err(ResourceError::Hota32Invalid {
            reason: "frame lies outside the canvas",
        });
    }
    let end = offset
        .checked_add(FRAME_HEADER_SIZE)
        .and_then(|v| v.checked_add(width * height * 4))
        .ok_or(ResourceError::Hota32Invalid {
            reason: "frame pixel range overflows",
        })?;
    if end > data.len() {
        return Err(ResourceError::Hota32Invalid {
            reason: "frame pixels truncated",
        });
    }
    Ok(FrameDir {
        name,
        offset,
        width,
        height,
        x,
        y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32le(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    /// Container with one 2x1 frame named "ab" holding pixels BGRA bottom-up.
    fn fixture() -> Vec<u8> {
        let mut data = Vec::new();
        u32le(&mut data, MAGIC);
        u32le(&mut data, 1);
        u32le(&mut data, 24);
        u32le(&mut data, 2); // canvas width
        u32le(&mut data, 2); // canvas height
        u32le(&mut data, 1); // group count
        u32le(&mut data, 8);
        u32le(&mut data, 0); // unused header word at offset 28
        u32le(&mut data, 16 + 17); // group record size = 16 + 17 * 1
        u32le(&mut data, 0); // group id
        u32le(&mut data, 1); // frame count
        u32le(&mut data, 4);
        data.extend_from_slice(b"ab\0");
        data.extend_from_slice(&[0; 10]);
        let frame_offset = 32 + 16 + 13 + 4; // header + group record bytes
        u32le(&mut data, frame_offset as u32);
        while data.len() < frame_offset {
            data.push(0);
        }
        u32le(&mut data, 32); // frame header size
        u32le(&mut data, 16); // pixel bytes = 2 * 2 * 4
        u32le(&mut data, 2); // full width
        u32le(&mut data, 2); // full height
        u32le(&mut data, 2); // pixel width
        u32le(&mut data, 2); // pixel height
        u32le(&mut data, 0); // x
        u32le(&mut data, 0); // y
        u32le(&mut data, 8);
        u32le(&mut data, 0); // unused frame header word at +36
        // Bottom-up BGRA: bottom row first, then top row.
        data.extend_from_slice(&[30, 20, 10, 255, 60, 50, 40, 255]); // bottom
        data.extend_from_slice(&[130, 120, 110, 255, 160, 150, 140, 128]); // top
        data
    }

    #[test]
    fn decodes_bottom_up_bgra_to_top_down_rgba() {
        let file = Hota32File::parse(fixture(), crate::ResourceLimits::default()).unwrap();
        assert_eq!(file.frame_names().collect::<Vec<_>>(), vec!["ab"]);
        let image = file.decode_named("ab").unwrap();
        assert_eq!((image.width, image.height), (2, 2));
        // Top-down RGBA: top row (BGRA [130,120,110,255] / [160,150,140,128])
        // first, then the bottom row.
        assert_eq!(
            image.data,
            vec![
                110, 120, 130, 255, 140, 150, 160, 128, // top
                10, 20, 30, 255, 40, 50, 60, 255, // bottom
            ]
        );
        assert!(file.decode_named("zz").is_err());
    }

    #[test]
    fn rejects_malformed_containers() {
        let limits = crate::ResourceLimits::default();
        assert!(Hota32File::parse(vec![0; 12], limits).is_err());
        let mut bad_magic = fixture();
        bad_magic[0] = b'X';
        assert!(Hota32File::parse(bad_magic, limits).is_err());
        let mut bad_frame = fixture();
        // Shrink the declared pixel byte count word (frame record + 4).
        let frame_offset = 32 + 16 + 13 + 4;
        bad_frame[frame_offset + 4..frame_offset + 8].copy_from_slice(&4u32.to_le_bytes());
        assert!(Hota32File::parse(bad_frame, limits).is_err());
        let mut truncated = fixture();
        truncated.truncate(truncated.len() - 3);
        assert!(Hota32File::parse(truncated, limits).is_err());
        let mut bad_group = fixture();
        bad_group[32..36].copy_from_slice(&15u32.to_le_bytes());
        assert!(Hota32File::parse(bad_group, limits).is_err());
    }
}
