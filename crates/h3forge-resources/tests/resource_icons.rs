use h3forge_resources::{LodArchive, ResourceIcons};

#[test]
fn seven_original_resource_icons_and_unknown_placeholder() {
    let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") else {
        eprintln!("skipping private resource artwork: set H3FORGE_FIXTURE_ROOT");
        return;
    };
    let archive = LodArchive::open(std::path::PathBuf::from(root).join("H3sprite.lod")).unwrap();
    let mut icons = ResourceIcons::load(&archive).unwrap();
    for id in 0..7 {
        let first = icons.png(id).unwrap().unwrap().to_vec();
        assert_eq!(&first[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(first[16..20].try_into().unwrap()), 32);
        assert_eq!(u32::from_be_bytes(first[20..24].try_into().unwrap()), 32);
        assert_eq!(icons.png(id).unwrap().unwrap(), first);
    }
    assert!(icons.png(7).unwrap().is_none());
    assert!(icons.png(u32::MAX).unwrap().is_none());
}
