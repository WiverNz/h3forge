//! Bounded static interface DEF reader, written for H3F-006.
//! Format facts cross-checked with VCMI client/render/CDefFile.cpp and local
//! TwCrPort.def headers; no third-party decoder implementation is copied.
//! Supports interface 0x47 and map 0x43; raw (0), row RLE (1), block RLE (3).

use crate::{ResourceError, ResourceLimits, RgbaImage};

fn invalid(reason: &'static str) -> ResourceError {
    ResourceError::DefInvalid { reason }
}
fn bytes(data: &[u8], at: usize, len: usize) -> Result<&[u8], ResourceError> {
    let end = at
        .checked_add(len)
        .ok_or_else(|| invalid("offset overflow"))?;
    data.get(at..end)
        .ok_or_else(|| invalid("truncated DEF data"))
}
fn word(data: &[u8], at: usize) -> Result<usize, ResourceError> {
    let b = bytes(data, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}
fn area(width: usize, height: usize, limits: ResourceLimits) -> Result<usize, ResourceError> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| invalid("dimensions overflow"))?;
    if width == 0 || height == 0 {
        return Err(invalid("zero dimensions"));
    }
    if pixels > limits.max_image_pixels {
        return Err(ResourceError::PixelLimit {
            limit: limits.max_image_pixels,
            claimed: pixels,
        });
    }
    Ok(pixels)
}

#[derive(Debug)]
pub struct DefFrame {
    pub name: String,
    pub group: u32,
    offset: usize,
}

#[derive(Debug)]
pub struct DefFile {
    data: Vec<u8>,
    frames: Vec<DefFrame>,
    limits: ResourceLimits,
}

/// Full canvas, preserving the palette and index-zero transparent padding.
#[derive(Debug)]
pub struct DefImage {
    pub width: u32,
    pub height: u32,
    pub indices: Vec<u8>,
    pub palette: Vec<u8>,
}
impl DefImage {
    /// Interface DEF transparency is palette index 0, not any cyan RGB value.
    /// Shadow/selection semantics of battle/map DEF types are not supported.
    pub fn to_rgba(&self) -> RgbaImage {
        let mut data = Vec::with_capacity(self.indices.len() * 4);
        for &index in &self.indices {
            let at = index as usize * 3;
            data.extend_from_slice(&self.palette[at..at + 3]);
            data.push(if index == 0 { 0 } else { 255 });
        }
        RgbaImage {
            width: self.width,
            height: self.height,
            data,
        }
    }
}

impl DefFile {
    pub fn parse(data: Vec<u8>, limits: ResourceLimits) -> Result<Self, ResourceError> {
        if data.len() > limits.max_entry_bytes {
            return Err(invalid("DEF exceeds entry byte limit"));
        }
        bytes(&data, 0, 784)?;
        if ![0x47, 0x43].contains(&word(&data, 0)?) {
            return Err(invalid(
                "only interface/map DEF types 0x47/0x43 are supported",
            ));
        }
        area(word(&data, 4)?, word(&data, 8)?, limits)?;
        let groups = word(&data, 12)?;
        if groups > limits.max_entries {
            return Err(invalid("too many groups"));
        }
        let mut at = 784;
        let mut frames = Vec::new();
        for _ in 0..groups {
            let group = word(&data, at)? as u32;
            let count = word(&data, at + 4)?;
            if count > limits.max_entries.saturating_sub(frames.len()) {
                return Err(invalid("too many frames"));
            }
            at += 16;
            let table_len = count
                .checked_mul(17)
                .ok_or_else(|| invalid("table overflow"))?;
            bytes(&data, at, table_len)?;
            for i in 0..count {
                let raw = bytes(&data, at + i * 13, 13)?;
                let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
                if end == 0 || !raw[..end].is_ascii() {
                    return Err(invalid("invalid frame name"));
                }
                frames.push(DefFrame {
                    name: String::from_utf8_lossy(&raw[..end]).into_owned(),
                    group,
                    offset: word(&data, at + count * 13 + i * 4)?,
                });
            }
            at += table_len;
        }
        for frame in &frames {
            if frame.offset < at {
                return Err(invalid("frame overlaps directory"));
            }
            bytes(&data, frame.offset, 32)?;
        }
        Ok(Self {
            data,
            frames,
            limits,
        })
    }

    pub fn frames(&self) -> &[DefFrame] {
        &self.frames
    }

    /// Decodes the frame at its directory-table index. Used where the format
    /// itself defines positional semantics (e.g. spells.def frame index =
    /// game spell ID); name-based callers should prefer [`decode_named`].
    pub fn decode_index(&self, index: usize) -> Result<DefImage, ResourceError> {
        let frame = self.frames.get(index).ok_or(ResourceError::BadIndex {
            index,
            entries: self.frames.len(),
        })?;
        self.decode_frame(frame)
    }

    pub fn decode_named(&self, name: &str) -> Result<DefImage, ResourceError> {
        let mut matches = self
            .frames
            .iter()
            .filter(|f| f.name.eq_ignore_ascii_case(name));
        let frame = matches
            .next()
            .ok_or_else(|| ResourceError::EntryNotFound { name: name.into() })?;
        if matches.next().is_some() {
            return Err(ResourceError::AmbiguousName { name: name.into() });
        }
        self.decode_frame(frame)
    }

    fn decode_frame(&self, frame: &DefFrame) -> Result<DefImage, ResourceError> {
        let end = self
            .frames
            .iter()
            .filter(|f| f.offset > frame.offset)
            .map(|f| f.offset)
            .min()
            .unwrap_or(self.data.len());
        let data = bytes(&self.data, frame.offset, end - frame.offset)?;
        let size = word(data, 0)?;
        let format = word(data, 4)?;
        if ![0, 1, 3].contains(&format) {
            return Err(ResourceError::DefCompression {
                format: format as u32,
            });
        }
        let (full_w, full_h) = (word(data, 8)?, word(data, 12)?);
        let pixels = area(full_w, full_h, self.limits)?;
        let (w, h, x, y) = (
            word(data, 16)?,
            word(data, 20)?,
            word(data, 24)?,
            word(data, 28)?,
        );
        area(w, h, self.limits)?;
        if x.checked_add(w).is_none_or(|v| v > full_w)
            || y.checked_add(h).is_none_or(|v| v > full_h)
        {
            return Err(invalid("crop lies outside canvas"));
        }
        let body = bytes(data, 32, size)?;
        let mut indices = vec![0; pixels];
        for row in 0..h {
            let target = &mut indices[(y + row) * full_w + x..(y + row) * full_w + x + w];
            if format == 0 {
                target.copy_from_slice(bytes(body, row * w, w)?);
                continue;
            }
            if format == 3 {
                if w % 32 != 0 {
                    return Err(invalid("block RLE width must be divisible by 32"));
                }
                let blocks = w / 32;
                let table_size = h
                    .checked_mul(blocks)
                    .and_then(|v| v.checked_mul(2))
                    .ok_or_else(|| invalid("block table overflow"))?;
                bytes(body, 0, table_size)?;
                for block in 0..blocks {
                    let raw = bytes(body, (row * blocks + block) * 2, 2)?;
                    let mut cursor = u16::from_le_bytes([raw[0], raw[1]]) as usize;
                    if cursor < table_size {
                        return Err(invalid("block overlaps offset table"));
                    }
                    let mut filled = 0;
                    while filled < 32 {
                        let tag = bytes(body, cursor, 1)?[0];
                        cursor += 1;
                        let len = (tag & 31) as usize + 1;
                        if len > 32 - filled {
                            return Err(invalid("RLE run exceeds block"));
                        }
                        let dest = &mut target[block * 32 + filled..block * 32 + filled + len];
                        if tag >> 5 == 7 {
                            dest.copy_from_slice(bytes(body, cursor, len)?);
                            cursor += len;
                        } else {
                            dest.fill(tag >> 5);
                        }
                        filled += len;
                    }
                }
                continue;
            }
            let table_size = h
                .checked_mul(4)
                .ok_or_else(|| invalid("row table overflow"))?;
            bytes(body, 0, table_size)?;
            let mut cursor = word(body, row * 4)?;
            if cursor < table_size {
                return Err(invalid("row overlaps offset table"));
            }
            let mut filled = 0;
            while filled < w {
                let packet = bytes(body, cursor, 2)?;
                let (value, len) = (packet[0], packet[1] as usize + 1);
                cursor += 2;
                if len > w - filled {
                    return Err(invalid("RLE run exceeds row"));
                }
                if value == 255 {
                    target[filled..filled + len].copy_from_slice(bytes(body, cursor, len)?);
                    cursor += len;
                } else {
                    target[filled..filled + len].fill(value);
                }
                filled += len;
            }
        }
        Ok(DefImage {
            width: full_w as u32,
            height: full_h as u32,
            indices,
            palette: self.data[16..784].to_vec(),
        })
    }
}
