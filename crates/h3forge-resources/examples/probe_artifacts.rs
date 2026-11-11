use h3forge_resources::{DefFile, Hota32File, LodArchive, ResourceLimits, encode_png};
fn main() {
    let base = LodArchive::open("Data/H3sprite.lod").unwrap();
    let def = DefFile::parse(
        base.read_entry(base.find_entry("Artifact.def").unwrap())
            .unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    let hota = LodArchive::open("Data/HotA.lod").unwrap();
    let file = Hota32File::parse(
        hota.read_entry(hota.find_entry("4ae18dc00814380facbef89026300713").unwrap())
            .unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    let images: Vec<_> = (0..file.frame_count())
        .map(|i| file.decode_index(i).unwrap())
        .collect();
    for (idx, f) in def.frames().iter().enumerate() {
        let a = def.decode_index(idx).unwrap().to_rgba();
        let best = images
            .iter()
            .enumerate()
            .filter(|(_, b)| a.width == b.width && a.height == b.height)
            .map(|(i, b)| {
                let diff: u64 = a
                    .data
                    .chunks_exact(4)
                    .zip(b.data.chunks_exact(4))
                    .map(|(a, b)| {
                        if a[3] == 0 && b[3] == 0 {
                            0
                        } else {
                            a.iter().zip(b).map(|(x, y)| x.abs_diff(*y) as u64).sum()
                        }
                    })
                    .sum();
                (diff, i)
            })
            .min()
            .unwrap();
        println!(
            "{} => {} diff={}",
            f.name,
            file.frame_name_at(best.1).unwrap(),
            best.0
        );
    }
    std::fs::create_dir_all(".local/artifact-probe").unwrap();
    for id in 141..=143 {
        let i = def
            .frames()
            .iter()
            .position(|f| f.name == format!("HrArt{id:03}.pcx"))
            .unwrap();
        std::fs::write(
            format!(".local/artifact-probe/base{id}.png"),
            encode_png(&def.decode_index(i).unwrap().to_rgba()),
        )
        .unwrap();
    }
    for i in 141..167 {
        std::fs::write(
            format!(".local/artifact-probe/{i}.png"),
            encode_png(&images[i]),
        )
        .unwrap();
    }
}
