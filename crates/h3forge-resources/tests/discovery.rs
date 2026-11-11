//! Installation discovery against synthetic directories in the OS temp dir.
//! All fixtures are built in memory; nothing under any real installation is
//! touched, and every temporary directory is removed on exit.

use std::fs;
use std::path::PathBuf;

use h3forge_resources::{ArchiveState, DiscoveryError, Installation, KNOWN_ARCHIVES, validate};

fn u32le(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// Minimal valid empty LOD (92-byte header, zero entries).
fn empty_lod() -> Vec<u8> {
    let mut out = vec![0u8; 92];
    out[0..4].copy_from_slice(b"LOD\0");
    out[4..8].copy_from_slice(&u32le(200));
    out
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        let mut path = std::env::temp_dir();
        path.push(format!("h3forge-discovery-{}-{}", std::process::id(), tag));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp dir creates");
        TempDir(path)
    }

    fn path(&self) -> &PathBuf {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn validate_accepts_archives_in_root_or_data_child() {
    let temp = TempDir::new("valid");
    fs::write(temp.path().join("H3bitmap.lod"), empty_lod()).unwrap();
    let install = validate(temp.path()).expect("root with archives validates");
    assert_eq!(install.root, *temp.path());
    assert!(install.has_base_artwork());
    assert_eq!(
        install.resource_sets(),
        Vec::<&str>::new(),
        "sprite archive absent"
    );

    let temp = TempDir::new("datachild");
    let data = temp.path().join("Data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("H3bitmap.lod"), empty_lod()).unwrap();
    fs::write(data.join("H3sprite.lod"), empty_lod()).unwrap();
    let install = validate(temp.path()).expect("Data child validates");
    assert_eq!(install.root, data);
    assert_eq!(
        install.resource_sets(),
        vec!["Heroes III: Complete (SoD) artwork"]
    );
}

#[test]
fn validate_rejects_empty_and_archiveless_dirs() {
    let temp = TempDir::new("empty");
    let error = validate(temp.path()).unwrap_err();
    assert!(matches!(error, DiscoveryError::NoArchives { .. }));

    let temp = TempDir::new("garbage");
    fs::write(temp.path().join("readme.txt"), b"not a game").unwrap();
    assert!(matches!(
        validate(temp.path()).unwrap_err(),
        DiscoveryError::NoArchives { .. }
    ));
}

#[test]
fn per_archive_status_distinguishes_failure_kinds() {
    let temp = TempDir::new("status");
    fs::write(temp.path().join("H3bitmap.lod"), empty_lod()).unwrap();
    fs::write(temp.path().join("HotA.lod"), empty_lod()).unwrap();
    fs::write(temp.path().join("h3ab_bmp.lod"), []).unwrap(); // zero bytes
    // Unknown names are never probed; garbage under a known name is unreadable.
    fs::write(temp.path().join("HotA_lng.lod"), b"garbage").unwrap();

    let install = validate(temp.path()).expect("validates on the readable one");
    let state = |name: &str| {
        install
            .archives
            .iter()
            .find(|status| status.name == name)
            .map(|status| status.state.clone())
    };
    assert!(matches!(
        state("H3bitmap.lod"),
        Some(ArchiveState::Readable { entries: 0 })
    ));
    assert!(
        state("HotA_l_ext.lod").is_none(),
        "missing archives are not reported"
    );
    assert!(matches!(
        state("h3ab_bmp.lod"),
        Some(ArchiveState::Unreadable { .. })
    ));
    assert!(matches!(
        state("HotA_lng.lod"),
        Some(ArchiveState::Unreadable { .. })
    ));
    assert!(
        state("junk.lod").is_none(),
        "unknown names are never probed"
    );
    assert_eq!(install.archives.len(), 4);
}

#[test]
fn discover_reports_not_found_without_candidates() {
    // The test binary's directory holds no LODs and no dev root is set in
    // the test environment; discovery must fail rather than guess. Skip when
    // a dev root happens to be configured for this shell.
    if std::env::var_os("H3FORGE_DEV_DATA_ROOT").is_some() {
        eprintln!("H3FORGE_DEV_DATA_ROOT is set; skipping not-found discovery test");
        return;
    }
    let error = h3forge_resources::discover(None).unwrap_err();
    match error {
        DiscoveryError::NotFound { searched } => assert!(!searched.is_empty()),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn stored_selection_is_revalidated_before_use() {
    let temp = TempDir::new("stored");
    let error = h3forge_resources::discover(Some(temp.path())).unwrap_err();
    assert!(matches!(error, DiscoveryError::NotFound { .. }));

    fs::write(temp.path().join("H3bitmap.lod"), empty_lod()).unwrap();
    let install: Installation =
        h3forge_resources::discover(Some(temp.path())).expect("stored dir now validates");
    assert!(install.has_base_artwork());
}

#[test]
fn known_archives_list_is_bounded() {
    assert!(KNOWN_ARCHIVES.contains(&"H3bitmap.lod"));
    assert!(KNOWN_ARCHIVES.contains(&"HotA.lod"));
    assert!(KNOWN_ARCHIVES.len() <= 8);
}
