//! Decode verified original tent icons into an ignored research directory.
use h3forge_resources::{KeymasterIcons, LodArchive};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 3 {
        return Err("usage: probe_tents <H3sprite.lod> <ignored output directory>".into());
    }
    let archive = LodArchive::open(&a[1])?;
    let mut icons = KeymasterIcons::load(&archive)?;
    std::fs::create_dir_all(&a[2])?;
    for color in 0..8 {
        if let Some(png) = icons.png(color)? {
            std::fs::write(
                std::path::Path::new(&a[2]).join(format!("tent-{color}.png")),
                png,
            )?;
        }
    }
    Ok(())
}
