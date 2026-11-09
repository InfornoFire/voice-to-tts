//! Commands for the global shortcut.

use tauri::State;

use crate::error::UserFacingError;
use crate::state::AppState;

/// Stops the shortcut from switching the engine while a new one is being recorded.
#[tauri::command]
#[specta::specta]
pub async fn set_hotkey_paused(
    state: State<'_, AppState>,
    paused: bool,
) -> Result<(), UserFacingError> {
    let shortcut = state.config.get().hotkey;
    let (previous, next) = if paused {
        (shortcut.as_str(), "")
    } else {
        ("", shortcut.as_str())
    };
    Ok(state.hotkeys.replace(previous, next)?)
}
