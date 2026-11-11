use h3forge_resources::{
    CreaturePortraits, DefFile, LodArchive, ResourceLimits, RgbaImage, creature_frame_name,
    encode_png,
};

fn put(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn synthetic(names: &[&str], format: u32, body: &[u8]) -> Vec<u8> {
    let offset = 800 + 17 * names.len();
    let mut data = vec![0; offset + 32 + body.len()];
    for (at, value) in [
        (0, 0x47),
        (4, 6),
        (8, 2),
        (12, 1),
        (784, 0),
        (788, names.len() as u32),
    ] {
        put(&mut data, at, value);
    }
    // Transparent index 0 is red; opaque index 1 is cyan. RGB is not the key.
    data[16..22].copy_from_slice(&[255, 0, 0, 0, 255, 255]);
    data[22..28].copy_from_slice(&[10, 20, 30, 40, 50, 60]);
    for (i, name) in names.iter().enumerate() {
        data[800 + i * 13..800 + i * 13 + name.len()].copy_from_slice(name.as_bytes());
        put(&mut data, 800 + 13 * names.len() + i * 4, offset as u32);
    }
    for (at, value) in [
        (0, body.len() as u32),
        (4, format),
        (8, 6),
        (12, 2),
        (16, 4),
        (20, 1),
        (24, 1),
        (28, 1),
    ] {
        put(&mut data, offset + at, value);
    }
    data[offset + 32..].copy_from_slice(body);
    data
}

#[test]
fn def_raw_and_row_rle_preserve_crop_palette_and_transparency() {
    let raw = synthetic(&["a.pcx"], 0, &[0, 1, 2, 3]);
    let rle = synthetic(&["a.pcx"], 1, &[4, 0, 0, 0, 255, 3, 0, 1, 2, 3]);
    for bytes in [raw, rle] {
        let def = DefFile::parse(bytes, ResourceLimits::default()).unwrap();
        let image = def.decode_named("A.PCX").unwrap();
        assert_eq!((image.width, image.height), (6, 2));
        assert_eq!(image.indices, [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 0]);
        let rgba = image.to_rgba();
        assert_eq!(&rgba.data[..4], &[255, 0, 0, 0]);
        assert_eq!(&rgba.data[8 * 4..9 * 4], &[0, 255, 255, 255]);
    }
    let run = DefFile::parse(
        synthetic(&["a.pcx"], 1, &[4, 0, 0, 0, 2, 3]),
        ResourceLimits::default(),
    )
    .unwrap();
    assert_eq!(
        &run.decode_named("a.pcx").unwrap().indices[7..11],
        &[2, 2, 2, 2]
    );
}

#[test]
fn def_rejects_bad_offsets_truncation_overruns_dimensions_and_variants() {
    let valid = synthetic(&["a.pcx"], 1, &[4, 0, 0, 0, 255, 3, 0, 1, 2, 3]);
    for end in [0, 16, 783, 800, 816] {
        assert!(DefFile::parse(valid[..end].to_vec(), ResourceLimits::default()).is_err());
    }
    for offset in [1, u32::MAX] {
        let mut bytes = valid.clone();
        put(&mut bytes, 813, offset);
        assert!(DefFile::parse(bytes, ResourceLimits::default()).is_err());
    }
    for body in [
        &[0, 0, 0, 0][..],
        &[4, 0, 0, 0, 255, 3, 1][..],
        &[4, 0, 0, 0, 2, 9][..],
    ] {
        let def =
            DefFile::parse(synthetic(&["a.pcx"], 1, body), ResourceLimits::default()).unwrap();
        assert!(def.decode_named("a.pcx").is_err());
    }
    for (at, value) in [
        (817 + 8, u32::MAX),
        (817 + 24, u32::MAX),
        (817 + 4, 2),
        (817 + 4, 3),
        (817, u32::MAX),
    ] {
        let mut bytes = valid.clone();
        put(&mut bytes, at, value);
        let def = DefFile::parse(bytes, ResourceLimits::default()).unwrap();
        assert!(def.decode_named("a.pcx").is_err());
    }
    let def = DefFile::parse(valid, ResourceLimits::default()).unwrap();
    assert!(def.decode_named("missing.pcx").is_err());
    let duplicate = DefFile::parse(
        synthetic(&["a.pcx", "A.PCX"], 0, &[0, 1, 2, 3]),
        ResourceLimits::default(),
    )
    .unwrap();
    assert!(duplicate.decode_named("a.pcx").is_err());
}

#[test]
fn creature_mapping_is_by_name_never_frame_position_or_hota_reuse() {
    assert_eq!(creature_frame_name(0).as_deref(), Some("CPrL000C.pcx"));
    assert_eq!(creature_frame_name(44).as_deref(), Some("CPrL044I.pcx"));
    assert_eq!(creature_frame_name(46).as_deref(), Some("CPrL046I.pcx"));
    for id in [122, 124, 126, 128, 145, 151, 153, 199, u32::MAX] {
        assert!(creature_frame_name(id).is_none());
    }
}

#[test]
fn local_twcrport_resolves_verified_standard_creatures() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("no asset root; skipping TwCrPort check");
        return;
    };
    let archive = LodArchive::open(std::path::PathBuf::from(root).join("H3sprite.lod")).unwrap();
    let bytes = archive
        .read_entry(archive.find_entry("TwCrPort.def").unwrap())
        .unwrap();
    let def = DefFile::parse(bytes, ResourceLimits::default()).unwrap();
    assert_eq!(def.frames().len(), 151);
    assert!(
        def.frames()
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case("CPrL046I.pcx"))
            .unwrap()
            < def
                .frames()
                .iter()
                .position(|f| f.name.eq_ignore_ascii_case("CPrL044I.pcx"))
                .unwrap()
    );
    let mut catalog = CreaturePortraits::load(&archive).unwrap();
    let mut resolved = 0;
    for id in 0..200 {
        if let Some(png) = catalog.png(id).unwrap() {
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
            resolved += 1;
        }
    }
    assert_eq!(resolved, 141);
    assert!(catalog.png(153).unwrap().is_none());
    let cached = catalog.png(44).unwrap().unwrap().to_vec();
    assert_eq!(catalog.png(44).unwrap().unwrap(), cached);
    // Small ignored visual proof: Pikeman, Archer, Griffin, Gog, Hell Hound, Ice Elemental.
    let mut sheet = RgbaImage {
        width: 174,
        height: 128,
        data: vec![0; 174 * 128 * 4],
    };
    for (tile, id) in [0, 2, 4, 44, 46, 123].into_iter().enumerate() {
        let image = def
            .decode_named(&creature_frame_name(id).unwrap())
            .unwrap()
            .to_rgba();
        assert_eq!((image.width, image.height), (58, 64));
        for y in 0..64 {
            for x in 0..58 {
                let source = (y * 58 + x) * 4;
                let dest = ((tile / 3 * 64 + y) * 174 + tile % 3 * 58 + x) * 4;
                let bg = if (x / 8 + y / 8) % 2 == 0 { 180 } else { 230 };
                for c in 0..3 {
                    sheet.data[dest + c] = if image.data[source + 3] == 0 {
                        bg
                    } else {
                        image.data[source + c]
                    };
                }
                sheet.data[dest + 3] = 255;
            }
        }
    }
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/fixtures/creature-portraits.png");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(&out, encode_png(&sheet)).unwrap();
    eprintln!(
        "TwCrPort: {resolved} verified portraits resolved; format 1; preview {}",
        out.display()
    );
}
