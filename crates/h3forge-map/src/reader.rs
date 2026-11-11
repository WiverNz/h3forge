//! Bounds-checked little-endian cursor over the decompressed payload.

use crate::error::MapError;

pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    max_text: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], max_text: usize) -> Self {
        Reader {
            data,
            pos: 0,
            max_text,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn bytes(&mut self, len: usize, what: &'static str) -> Result<&'a [u8], MapError> {
        let end = self
            .pos
            .checked_add(len)
            .filter(|&end| end <= self.data.len())
            .ok_or(MapError::Truncated {
                offset: self.pos,
                what,
            })?;
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    pub fn skip(&mut self, len: usize, what: &'static str) -> Result<(), MapError> {
        self.bytes(len, what).map(|_| ())
    }

    pub fn u8(&mut self, what: &'static str) -> Result<u8, MapError> {
        Ok(self.bytes(1, what)?[0])
    }

    /// Booleans are one byte; like the game, only the low bit is used.
    pub fn bool(&mut self, what: &'static str) -> Result<bool, MapError> {
        Ok(self.u8(what)? & 1 != 0)
    }

    pub fn u16(&mut self, what: &'static str) -> Result<u16, MapError> {
        let bytes = self.bytes(2, what)?;
        Ok(u16::from_le_bytes(bytes.try_into().expect("length 2")))
    }

    pub fn u32(&mut self, what: &'static str) -> Result<u32, MapError> {
        let bytes = self.bytes(4, what)?;
        Ok(u32::from_le_bytes(bytes.try_into().expect("length 4")))
    }

    /// A `u32` used as an element count, bounded by `limit`.
    pub fn count(&mut self, limit: usize, what: &'static str) -> Result<usize, MapError> {
        let offset = self.pos;
        let value = self.u32(what)? as usize;
        if value > limit {
            return Err(MapError::Invalid { offset, what });
        }
        Ok(value)
    }

    /// Length-prefixed (`u32`) raw text.
    pub fn string(&mut self, what: &'static str) -> Result<&'a [u8], MapError> {
        let offset = self.pos;
        let len = self.u32(what)? as usize;
        if len > self.max_text {
            return Err(MapError::Invalid { offset, what });
        }
        self.bytes(len, what)
    }

    pub fn skip_string(&mut self, what: &'static str) -> Result<(), MapError> {
        self.string(what).map(|_| ())
    }
}
