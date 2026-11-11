//! Installation persistence: remembers the manually selected game root
//! between launches, mirroring the open-dialog folder memory in
//! [`crate::dialog_dir`]. The file holds one line of lossy path text in the
//! OS config dir; every failure falls back to no stored selection.

use std::path::{Path, PathBuf};

use tauri::Manager;

const INSTALLATION_FILE_NAME: &str = "installation.txt";

pub fn installation_file_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    let config = app.path().app_config_dir().ok()?;
    Some(config.join(INSTALLATION_FILE_NAME))
}

pub fn read_persisted_root(file: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(file).ok()?;
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        let path = PathBuf::from(text);
        path.is_dir().then_some(path)
    }
}

pub fn remember_installation(file: &Path, root: &Path) {
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(file, root.to_string_lossy().as_bytes());
}
