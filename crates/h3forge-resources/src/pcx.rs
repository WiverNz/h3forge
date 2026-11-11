//! Explicit decoding of Heroes III "PCX" images to indexed data or RGBA.
//!
//! Adapted from mmarch (`mmarch-rust/src/image.rs::decode_pcx`, MIT, copyright
//! Tom Chen; snapshot commit `fc104f2a7d8b170ed67bc05b9bfac5d0bb9a60b8`; full
//! notice in `licenses/mmarch-MIT.txt`). This is not the real PCX format
//! (RSLod.pas `TRSLod.UnpackPcx`): a 12-byte header of three little-endian
//! i32s - image size, width, height - followed by raw pixels and, for 8-bit
//! images, a 768-byte R,G,B palette.
//!
//! Adaptations for H3Forge:
//!
//! * mmarch converted straight to BMP and silently dropped undecodable PCX;
//!   this module returns structured errors and keeps the indexed pixels plus
//!   palette intact before any RGBA conversion (research: ARCHITECTURE.md
//!   "Resource integration and discovery": preserve indexed palette
//!   semantics; a cyan-looking RGB colour is not a universal transparency
//!   rule).
//! * 24-bit pixels are B,G,R triplets: both mmarch and the Delphi RSPak feed
//!   the buffer unswapped into a Windows 24-bit bitmap, whose byte order is
//!   B,G,R.
//! * Dimension, pixel-count and palette bounds are checked against explicit
//!   limits before allocation.

use crate::error::ResourceError;
use crate::lod::ResourceLimits;

const PCX_HEADER_SIZE: usize = 12;
const PALETTE_SIZE: usize = 256 * 3;

/// A decoded Heroes III PCX image, either 8-bit indexed with its own palette
/// or 24-bit direct colour. Both variants carry top-down, row-major pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcxImage {
    /// 8-bit palette indices; `palette` is 768 bytes, R,G,B per entry.
    Indexed {
        width: u32,
        height: u32,
        /// `width * height` palette indices, first row at the top.
        indices: Vec<u8>,
        palette: Vec<u8>,
    },
    /// 24-bit colour; `bgr` is `width * height` B,G,R triplets, first row at
    /// the top.
    Bgr24 {
        width: u32,
        height: u32,
        bgr: Vec<u8>,
    },
}

impl PcxImage {
    /// Image width in pixels.
    pub fn width(&self) -> u32 {
        match self {
            PcxImage::Indexed { width, .. } | PcxImage::Bgr24 { width, .. } => *width,
        }
    }

    /// Image height in pixels.
    pub fn height(&self) -> u32 {
        match self {
            PcxImage::Indexed { height, .. } | PcxImage::Bgr24 { height, .. } => *height,
        }
    }

    /// Converts to straight RGBA with full opacity, one byte per channel,
    /// row-major from the top-left pixel. Alpha is always 255: no
    /// transparency convention for Heroes PCX has fixture evidence in this
    /// milestone, so none is applied (research: ARCHITECTURE.md "Resource
    /// integration and discovery"). Indexed callers that need palette-index
    /// semantics can use the `Indexed` variant directly.
    pub fn to_rgba(&self) -> RgbaImage {
        let (width, height) = (self.width(), self.height());
        let pixels = width as usize * height as usize;
        let mut data = Vec::with_capacity(pixels * 4);
        match self {
            PcxImage::Indexed {
                indices, palette, ..
            } => {
                for &index in indices {
                    let at = index as usize * 3;
                    let (r, g, b) = (
                        palette.get(at).copied().unwrap_or(0),
                        palette.get(at + 1).copied().unwrap_or(0),
                        palette.get(at + 2).copied().unwrap_or(0),
                    );
                    data.extend_from_slice(&[r, g, b, 255]);
                }
            }
            PcxImage::Bgr24 { bgr, .. } => {
                for triplet in bgr.as_chunks::<3>().0 {
                    data.extend_from_slice(&[triplet[2], triplet[1], triplet[0], 255]);
                }
            }
        }
        RgbaImage {
            width,
            height,
            data,
        }
    }
}

/// A decoded RGBA8 image ready for later conversion to frontend-compatible
/// formats (PNG encoding and transfer arrive with the desktop command layer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    /// `width * height` RGBA quads, row-major from the top-left pixel.
    pub data: Vec<u8>,
}

/// Decodes a Heroes III PCX image with explicit limits. Returns the indexed
/// or 24-bit form; call [`PcxImage::to_rgba`] for RGBA pixels.
pub fn decode_pcx(data: &[u8], limits: ResourceLimits) -> Result<PcxImage, ResourceError> {
    if data.len() < PCX_HEADER_SIZE {
        return Err(ResourceError::PcxTruncated {
            needed: PCX_HEADER_SIZE,
            actual: data.len(),
        });
    }
    let read_i32 =
        |at: usize| i32::from_le_bytes(data[at..at + 4].try_into().expect("slice length checked"));
    let image_size = read_i32(0);
    let width = read_i32(4);
    let height = read_i32(8);
    if width <= 0 || height <= 0 || image_size <= 0 {
        return Err(ResourceError::PcxBadDimensions {
            image_size,
            width,
            height,
        });
    }
    let (w, h) = (width as usize, height as usize);
    let pixels = w.checked_mul(h).ok_or(ResourceError::PixelLimit {
        limit: limits.max_image_pixels,
        claimed: usize::MAX,
    })?;
    if pixels > limits.max_image_pixels {
        return Err(ResourceError::PixelLimit {
            limit: limits.max_image_pixels,
            claimed: pixels,
        });
    }
    let len = image_size as usize;
    let bytes_per_pixel = len / pixels;
    if bytes_per_pixel * pixels != len || !(bytes_per_pixel == 1 || bytes_per_pixel == 3) {
        return Err(ResourceError::PcxBadPixelLength {
            image_size: len,
            width: w,
            height: h,
        });
    }
    if PCX_HEADER_SIZE + len > data.len() {
        return Err(ResourceError::PcxTruncated {
            needed: PCX_HEADER_SIZE + len,
            actual: data.len(),
        });
    }
    let pixel_data = &data[PCX_HEADER_SIZE..PCX_HEADER_SIZE + len];
    if bytes_per_pixel == 1 {
        let palette_end = PCX_HEADER_SIZE + len + PALETTE_SIZE;
        if palette_end > data.len() {
            return Err(ResourceError::PcxMissingPalette {
                needed: palette_end,
                actual: data.len(),
            });
        }
        Ok(PcxImage::Indexed {
            width: width as u32,
            height: height as u32,
            indices: pixel_data.to_vec(),
            palette: data[PCX_HEADER_SIZE + len..palette_end].to_vec(),
        })
    } else {
        Ok(PcxImage::Bgr24 {
            width: width as u32,
            height: height as u32,
            bgr: pixel_data.to_vec(),
        })
    }
}
