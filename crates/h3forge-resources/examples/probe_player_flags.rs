//! Read-only local flag research; PNG output belongs in ignored scratch space.
use h3forge_resources::{DefFile, LodArchive, ResourceLimits, encode_png};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let lod = LodArchive::open(&args[1]).unwrap();
    let def = DefFile::parse(
        lod.read_entry(lod.find_entry("CREST58.def").unwrap())
            .unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    std::fs::create_dir_all(&args[2]).unwrap();
    for (i, f) in def.frames().iter().enumerate() {
        let image = def.decode_index(i).unwrap();
        println!("{i} {} {}x{}", f.name, image.width, image.height);
        std::fs::write(
            std::path::Path::new(&args[2]).join(format!("flag-{i}.png")),
            encode_png(&image.to_rgba()),
        )
        .unwrap();
    }
}
