use h3forge_resources::{DefFile, LodArchive, ResourceLimits, SkillIcons, skill_frame_index};

#[test]
fn local_skill_icons_resolve_all_28_and_preserve_fallback() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("no asset root; skipping skill icons");
        return;
    };
    let archive = LodArchive::open(std::path::PathBuf::from(root).join("H3sprite.lod")).unwrap();
    let def = DefFile::parse(
        archive
            .read_entry(archive.find_entry("secsk32.def").unwrap())
            .unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    let mut icons = SkillIcons::load(&archive).unwrap();
    // Actual directory labels: early skills use historical artwork numbering,
    // not binary skill IDs. Frame positions follow the verified VCMI mapping.
    let labels = [
        12, 1, 7, 13, 3, 10, 6, 14, 9, 8, 2, 4, 11, 5, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28,
    ];
    for id in 0..28 {
        let mut variants = Vec::new();
        for mastery in 1..=3 {
            assert_eq!(
                def.frames()[skill_frame_index(id, mastery).unwrap()]
                    .name
                    .to_lowercase(),
                format!(
                    "skl32{:02}{}.pcx",
                    labels[id as usize],
                    (b'a' + mastery - 1) as char
                )
            );
            let image = def
                .decode_index(skill_frame_index(id, mastery).unwrap())
                .unwrap();
            assert_eq!((image.width, image.height), (32, 32));
            assert!(image.indices.iter().any(|&index| index != 0));
            let png = icons.png(id, mastery).unwrap().unwrap().to_vec();
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
            assert_eq!(icons.png(id, mastery).unwrap().unwrap(), png); // cached result
            assert_eq!(png, h3forge_resources::encode_png(&image.to_rgba()));
            variants.push(png);
        }
        assert_ne!(variants[0], variants[1]);
        assert_ne!(variants[1], variants[2]);
        assert_eq!(icons.png(id, 1).unwrap().unwrap(), variants[0]);
    }
    for id in [28, 29, u32::MAX] {
        for mastery in 1..=3 {
            assert_eq!(icons.png(id, mastery).unwrap(), None);
        }
    }
    for mastery in [0, 4, 255] {
        assert_eq!(icons.png(0, mastery).unwrap(), None);
    }
    eprintln!(
        "Verified 84 real mastery-specific 32x32 secondary-skill icons; unknown IDs unresolved."
    );
}
