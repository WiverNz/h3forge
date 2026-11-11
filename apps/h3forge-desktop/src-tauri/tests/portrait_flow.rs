//! End-to-end portrait flow through the plain session state (no Tauri
//! runtime): sample save open → installation set → hero portrait PNG with
//! resolved/unresolved outcomes. Opt-in against the local `Data/` directory
//! and the private sample copy; skips cleanly when either is absent.

use std::path::{Path, PathBuf};

use h3forge_desktop::session::{AppState, PortraitSize};

fn data_root() -> Option<PathBuf> {
    // The fixture root names the directory holding the private saves; it is
    // only used for artwork when it validates as an installation itself
    // (e.g. a repo root with a Data/ directory). Otherwise fall back to the
    // repository Data/ next to this crate.
    if let Some(root) = std::env::var_os("H3FORGE_FIXTURE_ROOT") {
        let root = PathBuf::from(root);
        if root.is_dir() && h3forge_resources::validate(&root).is_ok() {
            return Some(root);
        }
    }
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../Data");
    match data.canonicalize() {
        Ok(path) if path.is_dir() => Some(path),
        _ => None,
    }
}

fn sample_copy() -> Option<PathBuf> {
    let root = std::env::var_os("H3FORGE_FIXTURE_ROOT").map(PathBuf::from)?;
    let source = root.join("SaAndVla.GM2");
    if !source.is_file() {
        return None;
    }
    let destination = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.local/fixtures");
    std::fs::create_dir_all(&destination).ok()?;
    let destination = destination.join("SaAndVla.GM2");
    std::fs::copy(&source, &destination).ok()?;
    Some(destination)
}

#[test]
fn sample_heroes_resolve_and_placeholder_correctly() {
    let Some(root) = data_root() else {
        eprintln!("no data root; skipping portrait flow test");
        return;
    };
    let Some(copy) = sample_copy() else {
        eprintln!("no private sample copy; skipping portrait flow test");
        return;
    };

    let mut state = AppState::new();
    state
        .set_manual_installation(root.clone())
        .expect("installation validates");
    let grant = state.issue_grant(copy);
    state
        .open_granted_save(&grant.grant_id)
        .expect("sample opens");

    // Record 0 (Orrin): portrait 0 resolves to a real PNG from H3bitmap.lod.
    let portrait = state
        .hero_portrait(0, PortraitSize::Large)
        .expect("portrait command succeeds");
    assert!(portrait.resolved, "got {:?}", portrait.reason);
    let png = portrait.png_base64.expect("resolved portrait has data");
    assert!(!png.is_empty());
    assert_eq!(
        portrait.identity.map(|id| id.portrait_id),
        Some(0),
        "identity travels with the image"
    );

    // Cached second fetch resolves identically without error.
    let again = state.hero_portrait(0, PortraitSize::Large).expect("cached");
    assert!(again.resolved);

    // Renamed heroes keep their identity (evidence-backed mapping).
    let renamed = state
        .hero_portrait(4, PortraitSize::Small)
        .expect("renamed hero");
    assert!(renamed.resolved, "got {:?}", renamed.reason);
    assert_eq!(renamed.identity.map(|id| id.hero_type), Some(4));

    // A map-customized portrait from the HotA table (H3F-021): record 28
    // carries portrait 213, which resolves from HotA.lod by entry ID.
    let custom = state
        .hero_portrait(28, PortraitSize::Large)
        .expect("custom portrait");
    assert!(custom.resolved, "got {:?}", custom.reason);
    assert_eq!(custom.identity.map(|id| id.portrait_id), Some(213));
}

#[test]
fn creature_portrait_command_resolves_hota_and_keeps_placeholder_for_unknown_id() {
    let Some(root) = data_root() else {
        eprintln!("no artwork; skipping creature portrait command");
        return;
    };
    let mut state = AppState::new();
    state.set_manual_installation(root).unwrap();
    let first = state.creature_portrait(44).unwrap();
    assert!(first.resolved);
    assert!(first.png_base64.as_ref().unwrap().starts_with("iVBOR"));
    assert_eq!(
        state.creature_portrait(44).unwrap().png_base64,
        first.png_base64
    );
    // HotA-exclusive ID 153 resolves from the HotA.lod creature container.
    let hota = state.creature_portrait(153).unwrap();
    assert!(hota.resolved, "got {:?}", hota.reason);
    assert!(hota.png_base64.as_ref().unwrap().starts_with("iVBOR"));
    assert_ne!(hota.png_base64, first.png_base64);
    // An ID beyond the container's range (0-199) keeps the placeholder.
    let missing = state.creature_portrait(200).unwrap();
    assert!(!missing.resolved);
    assert!(missing.png_base64.is_none());
    assert!(missing.reason.unwrap().contains("HotA"));
}
