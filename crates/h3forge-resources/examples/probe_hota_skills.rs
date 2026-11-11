use h3forge_resources::{DefFile, LodArchive, ResourceLimits};
use miniz_oxide::inflate::core::{
    DecompressorOxide, decompress,
    inflate_flags::{TINFL_FLAG_PARSE_ZLIB_HEADER, TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF},
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};
fn main() {
    let path = std::env::args().nth(1).expect("archive path");
    let lod = LodArchive::open(&path).unwrap();
    let mut file = File::open(path).unwrap();
    println!(
        "entries={} opaque={}",
        lod.entries().len(),
        lod.has_opaque_ids()
    );
    let mut png_candidates = 0;
    let mut pcx_candidates = 0;
    for (i, e) in lod.entries().iter().enumerate() {
        if e.unpacked_size < 800 || e.unpacked_size > 1_000_000 {
            continue;
        }
        file.seek(SeekFrom::Start(e.offset)).unwrap();
        let mut prefix = vec![0; (e.stored_size as usize).min(4096)];
        file.read_exact(&mut prefix).unwrap();
        let mut head = [0u8; 32];
        if e.compression == 0 {
            head.copy_from_slice(&prefix[..32]);
        } else if e.compression == 3 {
            let mut d = Box::<DecompressorOxide>::default();
            let (_, _, n) = decompress(
                &mut d,
                &prefix,
                &mut head,
                0,
                TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
            );
            if n != 32 {
                continue;
            }
        } else {
            continue;
        }
        let w = |a| u32::from_le_bytes(head[a..a + 4].try_into().unwrap());
        if &head[..8] == b"\x89PNG\r\n\x1a\n" {
            let pw = u32::from_be_bytes(head[16..20].try_into().unwrap());
            let ph = u32::from_be_bytes(head[20..24].try_into().unwrap());
            if [(32, 32), (44, 44), (82, 93)].contains(&(pw, ph)) {
                png_candidates += 1;
            }
        }
        if [(32, 32), (44, 44), (82, 93)].contains(&(w(4), w(8))) && w(0) != 0x47 {
            if e.unpacked_size == 12 + w(4) * w(8) + 768 || e.unpacked_size == 12 + 3 * w(4) * w(8)
            {
                pcx_candidates += 1;
            }
        }
        if w(0) != 0x47 || ![(32, 32), (44, 44), (82, 93)].contains(&(w(4), w(8))) {
            continue;
        }
        let data = lod.read_entry(i).unwrap();
        let def = DefFile::parse(data, ResourceLimits::default()).unwrap();
        println!(
            "{} {} {}x{} frames={} last={:?}",
            i,
            e.name,
            w(4),
            w(8),
            def.frames().len(),
            def.frames()
                .iter()
                .rev()
                .take(12)
                .map(|f| &f.name)
                .collect::<Vec<_>>()
        );
    }
    println!("Unidentified image-size candidates: PNG={png_candidates}, PCX={pcx_candidates}");
}
