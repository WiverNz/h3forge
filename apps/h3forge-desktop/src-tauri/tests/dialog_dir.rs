//! Tests for the open-dialog folder memory: initial directory resolution
//! and persistence of the last picked folder. All state lives under unique
//! OS-temp dirs so parallel tests never share files.

use std::fs;
use std::path::PathBuf;

use h3forge_desktop::dialog_dir;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "h3forge-dialog-dir-{}-{}-{}",
        tag,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn missing_state_file_falls_back_to_existing_dir() {
    let root = temp_dir("missing");
    let initial = dialog_dir::resolve_initial_dir(&root.join("nope.txt"));
    assert!(initial.is_dir());
    assert_ne!(initial, root.join("nope.txt"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn persisted_folder_is_used_while_it_exists() {
    let root = temp_dir("persisted");
    let wanted = root.join("saves");
    fs::create_dir_all(&wanted).unwrap();
    let state = root.join("state.txt");
    fs::write(&state, wanted.to_string_lossy().as_bytes()).unwrap();
    assert_eq!(dialog_dir::resolve_initial_dir(&state), wanted);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn stale_persisted_folder_falls_back() {
    let root = temp_dir("stale");
    let deleted = root.join("deleted");
    let state = root.join("state.txt");
    fs::write(&state, deleted.to_string_lossy().as_bytes()).unwrap();
    let initial = dialog_dir::resolve_initial_dir(&state);
    assert_ne!(initial, deleted);
    assert!(initial.is_dir());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn remember_roundtrips_parent_folder() {
    let root = temp_dir("roundtrip");
    let folder = root.join("games");
    fs::create_dir_all(&folder).unwrap();
    let picked = folder.join("SaAndVla.GM2");
    fs::write(&picked, b"synthetic").unwrap();
    // Nested state dir also exercises remember's create_dir_all.
    let state = root.join("nested").join("state.txt");
    dialog_dir::remember_picked_dir(&state, &picked);
    assert_eq!(dialog_dir::resolve_initial_dir(&state), folder);
    let _ = fs::remove_dir_all(&root);
}
