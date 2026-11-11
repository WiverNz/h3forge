//! Open-dialog folder memory: where the file picker starts and how it
//! remembers the last used folder between launches.
//!
//! The state file holds a single line with the last picked folder as lossy
//! path text. It lives in the OS config dir, is written only by Rust (never
//! by the frontend), and every failure falls back silently to the
//! executable's directory.

use std::path::{Path, PathBuf};

use tauri::Manager;

/// File inside the OS config dir that stores the last picked folder.
const STATE_FILE_NAME: &str = "open_dialog_dir.txt";

/// Resolves the folder the open dialog should start in: the persisted last
/// folder while it still exists, otherwise the directory the executable
/// runs from.
pub fn resolve_initial_dir(state_file: &Path) -> PathBuf {
    read_persisted_dir(state_file)
        .filter(|dir| dir.is_dir())
        .unwrap_or_else(exe_dir)
}

/// Remembers the folder a file was picked from. Write failures are
/// non-fatal: the next dialog just falls back to the exe directory.
pub fn remember_picked_dir(state_file: &Path, picked: &Path) {
    let Some(dir) = picked.parent() else { return };
    if let Some(parent) = state_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(state_file, dir.to_string_lossy().as_bytes());
}

/// Location of the state file; `None` when the OS config dir is unavailable
/// (persistence is skipped and startup falls back to the exe directory).
pub fn state_file_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    let config = app.path().app_config_dir().ok()?;
    Some(config.join(STATE_FILE_NAME))
}

fn read_persisted_dir(state_file: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(state_file).ok()?;
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text))
    }
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}
