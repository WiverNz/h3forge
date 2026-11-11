//! Local research only. Writes decoded PNGs to the supplied scratch folder.
use h3forge_resources::{DefFile, LodArchive, ResourceLimits, encode_png};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let archive = LodArchive::open(&args[1]).unwrap();
    for e in archive
        .entries()
        .iter()
        .filter(|e| e.name.to_lowercase().contains("resour"))
    {
        println!("{}", e.name);
    }
    let Ok(index) = archive.find_entry("resource.def") else {
        return;
    };
    let def = DefFile::parse(
        archive.read_entry(index).unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    std::fs::create_dir_all(&args[2]).unwrap();
    for (i, frame) in def.frames().iter().enumerate() {
        let image = def.decode_index(i).unwrap();
        println!("{i}: {} {}x{}", frame.name, image.width, image.height);
        std::fs::write(
            std::path::Path::new(&args[2]).join(format!("resource-{i}.png")),
            encode_png(&image.to_rgba()),
        )
        .unwrap();
    }
}
