//! Minimal PNG encoding for decoded RGBA images.
//!
//! Hand-rolled (no image crate): single-image PNG, 8-bit RGBA (color type 6),
//! filter type 0 on every scanline, one IDAT holding a zlib stream produced
//! by the workspace's existing `miniz_oxide`. CRC-32 via `crc32fast`.

use crate::pcx::RgbaImage;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Encodes a decoded image as PNG bytes.
pub fn encode_png(image: &RgbaImage) -> Vec<u8> {
    let mut out = Vec::with_capacity(image.data.len() + 512);
    out.extend_from_slice(&PNG_SIGNATURE);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&image.width.to_be_bytes());
    ihdr.extend_from_slice(&image.height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type: RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut out, b"IHDR", &ihdr);

    // Scanlines: filter byte 0 in front of each row of RGBA quads.
    let stride = image.width as usize * 4;
    let mut raw = Vec::with_capacity((stride + 1) * image.height as usize);
    for row in image.data.chunks_exact(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let idat = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
    write_chunk(&mut out, b"IDAT", &idat);

    write_chunk(&mut out, b"IEND", &[]);
    out
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32fast::hash(&out[out.len() - data.len() - 4..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_png_image() {
        let mut data = Vec::new();
        data.extend_from_slice(&[255, 0, 0, 255, 0, 255, 0, 255]);
        data.extend_from_slice(&[0, 0, 255, 255, 0, 0, 0, 0]);
        let image = RgbaImage {
            width: 2,
            height: 2,
            data,
        };
        let png = encode_png(&image);
        assert_eq!(&png[..8], &PNG_SIGNATURE);

        // IHDR: first chunk, 13 bytes, 2x2, 8-bit RGBA.
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], &2u32.to_be_bytes());
        assert_eq!(&png[20..24], &2u32.to_be_bytes());
        assert_eq!(png[24], 8);
        assert_eq!(png[25], 6);
        // IEND present at the tail.
        let n = png.len();
        assert_eq!(&png[n - 8..n - 4], b"IEND");
        assert_eq!(&png[n - 12..n - 8], &0u32.to_be_bytes());
    }
}
