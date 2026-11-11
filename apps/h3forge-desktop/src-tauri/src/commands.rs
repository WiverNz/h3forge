//! Tauri command surface: thin wrappers over [`crate::session::AppState`].
//!
//! Every command takes the shared state lock, delegates to a plain method
//! (unit-testable without a Tauri runtime) and maps lock poisoning to a
//! structured error. No command accepts paths or write instructions.
//!
//! Commands that show a native dialog are `#[tauri::command(async)]`: they
//! run on the thread pool, never on the main thread. A modal dialog spins a
//! nested message loop, and doing that inside the WebView2 IPC callback
//! re-enters WebView2 and trips a fatal check in `EmbeddedBrowserWebView.dll`
//! (STATUS_BREAKPOINT 0x80000003). The state lock is never held while a
//! dialog is open, so main-thread commands cannot block behind it.

use std::sync::{Mutex, MutexGuard};

use tauri::State;

use crate::dto::{
    AppErrorDto, AppInfo, HeroDetailDto, HeroEditRequestDto, HeroEditResponseDto, HeroListItemDto,
    PendingEditDto, SaveGrantDto, SaveSummaryDto,
};
use crate::session::AppState;

#[tauri::command]
pub fn get_ui_languages() -> Vec<String> {
    sys_locale::get_locales().take(32).collect()
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "H3Forge",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
    }
}

/// Runs the native picker and, on selection, stores the path as the single
/// openable grant. Returns `None` when the user cancels. The dialog starts
/// in the last used folder (falling back to the exe directory) and
/// remembers the folder picked from for next time.
#[tauri::command(async)]
pub fn choose_save(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, Mutex<AppState>>,
    labels: crate::localization::DialogLabels,
) -> Result<Option<SaveGrantDto>, AppErrorDto> {
    labels.validate()?;
    pick_save_path(&app, &window, &state, &labels)
}

#[cfg(windows)]
fn pick_save_path(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    state: &State<'_, Mutex<AppState>>,
    labels: &crate::localization::DialogLabels,
) -> Result<Option<SaveGrantDto>, AppErrorDto> {
    let state_file = crate::dialog_dir::state_file_path(app);
    let initial = state_file
        .as_deref()
        .map(crate::dialog_dir::resolve_initial_dir)
        .unwrap_or_else(crate::dialog_dir::exe_dir);
    let path = rfd::FileDialog::new()
        .set_parent(window)
        .set_title(&labels.title)
        .add_filter(&labels.file_type, &["gm1", "gm2", "sav"])
        .add_filter(&labels.all_files, &["*"])
        .set_directory(&initial)
        .pick_file();
    let Some(path) = path else { return Ok(None) };
    if let Some(file) = state_file.as_deref() {
        crate::dialog_dir::remember_picked_dir(file, &path);
    }
    Ok(Some(lock(state)?.issue_grant(path)))
}

#[cfg(not(windows))]
fn pick_save_path(
    _app: &tauri::AppHandle,
    _window: &tauri::WebviewWindow,
    _state: &State<'_, Mutex<AppState>>,
    labels: &crate::localization::DialogLabels,
) -> Result<Option<SaveGrantDto>, AppErrorDto> {
    Err(AppErrorDto::picker_unsupported())
}

#[tauri::command]
pub fn open_granted_save(
    state: State<'_, Mutex<AppState>>,
    grant_id: String,
) -> Result<SaveSummaryDto, AppErrorDto> {
    lock(&state)?.open_granted_save(&grant_id)
}

#[tauri::command]
pub fn list_heroes(state: State<'_, Mutex<AppState>>) -> Result<Vec<HeroListItemDto>, AppErrorDto> {
    lock(&state)?.list_heroes()
}

#[tauri::command]
pub fn get_hero(
    state: State<'_, Mutex<AppState>>,
    id: usize,
) -> Result<HeroDetailDto, AppErrorDto> {
    lock(&state)?.get_hero(id)
}

/// Current installation description, auto-discovering on first use
/// (portable exe directory, debug development root, stored selection).
#[tauri::command]
pub fn get_installation_status(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<crate::dto::InstallationDto, AppErrorDto> {
    let mut guard = lock(&state)?;
    if guard.stored_installation_root().is_none()
        && let Some(file) = crate::installation::installation_file_path(&app)
        && let Some(root) = crate::installation::read_persisted_root(&file)
    {
        guard.set_stored_installation_root(root);
    }
    let dto = {
        let install = guard.installation()?;
        crate::dto::InstallationDto::new(install.picked_root(), install)
    };
    Ok(dto)
}

/// Native folder picker for the game installation. `None` when cancelled.
#[tauri::command(async)]
pub fn choose_installation(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, Mutex<AppState>>,
    labels: crate::localization::DialogLabels,
) -> Result<Option<crate::dto::InstallationDto>, AppErrorDto> {
    labels.validate()?;
    let initial = lock(&state)?
        .installation()
        .map(|install| install.picked_root().to_path_buf())
        .unwrap_or_else(|_| crate::dialog_dir::exe_dir());
    let Some(path) = rfd::FileDialog::new()
        .set_parent(&window)
        .set_title(&labels.title)
        .set_directory(&initial)
        .pick_folder()
    else {
        return Ok(None);
    };
    let mut guard = lock(&state)?;
    guard.set_manual_installation(path.clone())?;
    if let Some(file) = crate::installation::installation_file_path(&app) {
        crate::installation::remember_installation(&file, &path);
    }
    let dto = {
        let install = guard.installation()?;
        crate::dto::InstallationDto::new(install.picked_root(), install)
    };
    Ok(Some(dto))
}

/// PNG portrait for one hero; `size` is "large" or "small".
#[tauri::command]
pub fn get_hero_portrait(
    state: State<'_, Mutex<AppState>>,
    id: usize,
    size: String,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    let size = portrait_size(&size)?;
    lock(&state)?.hero_portrait(id, size)
}

/// Batched small portraits for the hero list, in the same order as `ids`.
#[tauri::command]
pub fn get_hero_portraits(
    state: State<'_, Mutex<AppState>>,
    ids: Vec<usize>,
    size: String,
) -> Result<Vec<crate::dto::PortraitDto>, AppErrorDto> {
    let size = portrait_size(&size)?;
    let mut guard = lock(&state)?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(guard.hero_portrait(id, size)?);
    }
    Ok(out)
}

fn portrait_size(size: &str) -> Result<crate::session::PortraitSize, AppErrorDto> {
    match size {
        "large" => Ok(crate::session::PortraitSize::Large),
        "small" => Ok(crate::session::PortraitSize::Small),
        _ => Err(AppErrorDto {
            cause: None,
            params: std::collections::BTreeMap::new(),
            code: "bad-request",
            message: format!("unknown portrait size {size:?}"),
        }),
    }
}

// ------------------------------------------------------ hero editing (M3) --

/// Applies one batch of semantic edits to one hero and returns its effective
/// editable state. Values are absolute; the backend keeps pending edits per
/// hero and re-plans patches against the immutable source payload.
#[tauri::command]
pub fn apply_hero_edit(
    state: State<'_, Mutex<AppState>>,
    id: usize,
    request: HeroEditRequestDto,
) -> Result<HeroEditResponseDto, AppErrorDto> {
    lock(&state)?.apply_hero_edit(id, request.to_edits(), Vec::new())
}

/// Effective editable state for one hero (base values plus pending edits).
#[tauri::command]
pub fn get_hero_edit_state(
    state: State<'_, Mutex<AppState>>,
    id: usize,
) -> Result<HeroEditResponseDto, AppErrorDto> {
    lock(&state)?.hero_edit_state(id)
}

/// Unsaved-changes summary across all heroes.
#[tauri::command]
pub fn get_pending_edits(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<PendingEditDto>, AppErrorDto> {
    lock(&state)?.pending_edits()
}

/// Discards every pending edit of one hero.
#[tauri::command]
pub fn discard_hero_edits(
    state: State<'_, Mutex<AppState>>,
    id: usize,
) -> Result<Vec<PendingEditDto>, AppErrorDto> {
    lock(&state)?.discard_hero_edits(id)
}

/// Marks or unmarks one hero for dismissal (H3F-026); written only by Save.
#[tauri::command]
pub fn set_hero_dismissal(
    state: State<'_, Mutex<AppState>>,
    id: usize,
    dismissed: bool,
) -> Result<HeroEditResponseDto, AppErrorDto> {
    lock(&state)?.set_hero_dismissal(id, dismissed)
}

// ------------------------------------------------------ town commands (H3F-013) --

/// Compact town list with pending garrison overrides applied.
#[tauri::command]
pub fn list_towns(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<crate::dto::TownListItemDto>, AppErrorDto> {
    lock(&state)?.list_towns()
}

/// Read-only town detail with initial-map facts attached.
#[tauri::command]
pub fn get_town(
    state: State<'_, Mutex<AppState>>,
    id: usize,
) -> Result<crate::dto::TownDetailDto, AppErrorDto> {
    lock(&state)?.get_town(id)
}

/// Applies one batch of semantic edits to one town and returns its effective
/// editable state; values are absolute and nothing is written to disk.
#[tauri::command]
pub fn apply_town_edit(
    state: State<'_, Mutex<AppState>>,
    id: usize,
    request: crate::dto::TownEditRequestDto,
) -> Result<crate::dto::TownEditResponseDto, AppErrorDto> {
    lock(&state)?.apply_town_request(id, &request)
}

/// Effective editable state for one town (base values plus pending edits).
#[tauri::command]
pub fn get_town_edit_state(
    state: State<'_, Mutex<AppState>>,
    id: usize,
) -> Result<crate::dto::TownEditResponseDto, AppErrorDto> {
    lock(&state)?.get_town_edit_state(id)
}

/// Discards every pending edit of one town.
#[tauri::command]
pub fn discard_town_edits(
    state: State<'_, Mutex<AppState>>,
    id: usize,
) -> Result<Vec<PendingEditDto>, AppErrorDto> {
    lock(&state)?.discard_town_edits(id)
}

/// Discards all pending edits across heroes.
#[tauri::command]
pub fn discard_all_edits(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<PendingEditDto>, AppErrorDto> {
    lock(&state)?.discard_all_edits()
}

/// Native Save As picker plus the safe write. `None` when the user cancels.
/// The destination comes from the Rust-owned dialog only; the write is a
/// verified temporary file plus a no-clobber commit, and the original file
/// is never modified.
#[tauri::command(async)]
pub fn save_save_as(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, Mutex<AppState>>,
    labels: crate::localization::DialogLabels,
) -> Result<Option<SaveSummaryDto>, AppErrorDto> {
    labels.validate()?;
    save_save_as_inner(&app, &window, &state, &labels)
}

#[cfg(windows)]
fn save_save_as_inner(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    state: &State<'_, Mutex<AppState>>,
    labels: &crate::localization::DialogLabels,
) -> Result<Option<SaveSummaryDto>, AppErrorDto> {
    // Copy what the picker needs and release the lock before it opens.
    let (initial, default_name) = {
        let guard = lock(state)?;
        let session = guard.session_ref()?;
        let initial = session
            .source_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(crate::dialog_dir::exe_dir);
        (initial, edited_file_name(&session.file_name))
    };
    let picker = rfd::FileDialog::new()
        .set_parent(window)
        .set_title(&labels.title)
        .add_filter(&labels.file_type, &["gm2", "gm1", "sav"])
        .set_directory(&initial)
        .set_file_name(&default_name);
    let Some(path) = picker.save_file() else {
        return Ok(None);
    };
    let result = lock(state)?.save_save_as(&path);
    if let Some(file) = crate::dialog_dir::state_file_path(app) {
        crate::dialog_dir::remember_picked_dir(&file, &path);
    }
    result.map(Some)
}

/// "SaAndVla.GM2" -> "SaAndVla (edited).GM2".
#[cfg(windows)]
fn edited_file_name(name: &str) -> String {
    match name.rfind('.') {
        Some(index) if index > 0 => format!("{} (edited){}", &name[..index], &name[index..]),
        _ => format!("{name} (edited)"),
    }
}

#[cfg(not(windows))]
fn save_save_as_inner(
    _app: &tauri::AppHandle,
    _window: &tauri::WebviewWindow,
    _state: &State<'_, Mutex<AppState>>,
    labels: &crate::localization::DialogLabels,
) -> Result<Option<SaveSummaryDto>, AppErrorDto> {
    Err(AppErrorDto::picker_unsupported())
}

fn lock<'a>(
    state: &'a State<'_, Mutex<AppState>>,
) -> Result<MutexGuard<'a, AppState>, AppErrorDto> {
    state
        .inner()
        .lock()
        .map_err(|_| AppErrorDto::state_locked())
}

/// Direct Save uses only the session's already-open source path.
#[tauri::command]
pub fn save(state: State<'_, Mutex<AppState>>) -> Result<SaveSummaryDto, AppErrorDto> {
    lock(&state)?.save()
}

#[tauri::command]
pub fn get_creature_portrait(
    state: State<'_, Mutex<AppState>>,
    id: u32,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    lock(&state)?.creature_portrait(id)
}

/// Static town icon by town type; `fort` picks the fort or village picture
/// and `large` the 58x64 or 48x32 family.
#[tauri::command]
pub fn get_town_icon(
    state: State<'_, Mutex<AppState>>,
    town_type: u8,
    fort: bool,
    large: bool,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    lock(&state)?.town_icon(town_type, fort, large)
}

/// Town Hall picture of one building by town type and building ID.
#[tauri::command]
pub fn get_building_icon(
    state: State<'_, Mutex<AppState>>,
    town_type: u8,
    building: u8,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    lock(&state)?.building_icon(town_type, building)
}

#[tauri::command]
pub fn get_spell_icon(
    state: State<'_, Mutex<AppState>>,
    id: u32,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    lock(&state)?.spell_icon(id)
}

#[tauri::command]
pub fn get_skill_icon(
    state: State<'_, Mutex<AppState>>,
    id: u32,
    mastery: u8,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    lock(&state)?.skill_icon(id, mastery)
}

#[tauri::command]
pub fn get_artifact_icon(
    state: State<'_, Mutex<AppState>>,
    id: u32,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    lock(&state)?.artifact_icon(id)
}

// ------------------------------------------------------------ map link --

/// Link between the open save and its source `.h3m` (H3F-021).
#[tauri::command]
pub fn get_map_link(
    state: State<'_, Mutex<AppState>>,
) -> Result<crate::map_link::MapLinkDto, AppErrorDto> {
    lock(&state)?.map_link()
}

/// Re-runs bounded map discovery for the open save.
#[tauri::command]
pub fn rescan_maps(
    state: State<'_, Mutex<AppState>>,
) -> Result<crate::map_link::MapLinkDto, AppErrorDto> {
    lock(&state)?.rescan_maps()
}

/// Links one candidate of an ambiguous discovery result.
#[tauri::command]
pub fn select_map_candidate(
    state: State<'_, Mutex<AppState>>,
    index: usize,
) -> Result<crate::map_link::MapLinkDto, AppErrorDto> {
    lock(&state)?.select_map_candidate(index)
}

/// Native picker for the source map. `None` when cancelled. The path never
/// reaches the frontend; the chosen file is only ever read.
#[tauri::command(async)]
pub fn choose_map_file(
    window: tauri::WebviewWindow,
    state: State<'_, Mutex<AppState>>,
    labels: crate::localization::DialogLabels,
) -> Result<Option<crate::map_link::MapLinkDto>, AppErrorDto> {
    labels.validate()?;
    pick_map_file(&window, &state, &labels)
}

#[cfg(windows)]
fn pick_map_file(
    window: &tauri::WebviewWindow,
    state: &State<'_, Mutex<AppState>>,
    labels: &crate::localization::DialogLabels,
) -> Result<Option<crate::map_link::MapLinkDto>, AppErrorDto> {
    let initial = lock(state)?
        .map_picker_dir()
        .unwrap_or_else(crate::dialog_dir::exe_dir);
    let Some(path) = rfd::FileDialog::new()
        .set_parent(window)
        .set_title(&labels.title)
        .add_filter(&labels.file_type, &["h3m"])
        .set_directory(&initial)
        .pick_file()
    else {
        return Ok(None);
    };
    lock(state)?.set_manual_map(&path).map(Some)
}

#[cfg(not(windows))]
fn pick_map_file(
    _window: &tauri::WebviewWindow,
    _state: &State<'_, Mutex<AppState>>,
    labels: &crate::localization::DialogLabels,
) -> Result<Option<crate::map_link::MapLinkDto>, AppErrorDto> {
    Err(AppErrorDto::picker_unsupported())
}

#[tauri::command]
pub fn list_players(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<crate::dto::PlayerDto>, AppErrorDto> {
    lock(&state)?.list_players()
}

#[tauri::command]
pub fn apply_player_edit(
    state: State<'_, Mutex<AppState>>,
    id: usize,
    resource: u8,
    value: i64,
) -> Result<Vec<crate::dto::PlayerDto>, AppErrorDto> {
    lock(&state)?.apply_player_edit(id, resource, value)
}

#[tauri::command]
pub fn discard_player_edits(
    state: State<'_, Mutex<AppState>>,
    id: usize,
) -> Result<Vec<crate::dto::PlayerDto>, AppErrorDto> {
    lock(&state)?.discard_player_edits(id)
}

#[tauri::command]
pub fn get_resource_icon(
    state: State<'_, Mutex<AppState>>,
    id: u32,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    lock(&state)?.resource_icon(id)
}

#[tauri::command]
pub fn get_player_flag(
    state: State<'_, Mutex<AppState>>,
    color: u32,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    lock(&state)?.player_flag(color)
}

#[tauri::command]
pub fn get_player_starting_portrait(
    state: State<'_, Mutex<AppState>>,
    id: usize,
) -> Result<crate::dto::PortraitDto, AppErrorDto> {
    lock(&state)?.player_starting_portrait(id)
}
#[tauri::command]
pub fn list_neutral_armies(
    state: State<'_, Mutex<AppState>>,
) -> Result<crate::neutral::NeutralListDto, AppErrorDto> {
    lock(&state)?.list_neutral_armies()
}

#[tauri::command]
pub fn apply_neutral_edit(
    state: State<'_, Mutex<AppState>>,
    id: usize,
    army: crate::dto::ArmyEditDto,
) -> Result<crate::neutral::NeutralListDto, AppErrorDto> {
    lock(&state)?.apply_neutral_edit(id, army)
}

#[tauri::command]
pub fn discard_neutral_edits(
    state: State<'_, Mutex<AppState>>,
    id: usize,
) -> Result<crate::neutral::NeutralListDto, AppErrorDto> {
    lock(&state)?.discard_neutral_edits(id)
}
