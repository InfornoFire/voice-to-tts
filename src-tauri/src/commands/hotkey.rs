//! Commands for the global shortcuts.

use tauri::State;

use crate::error::UserFacingError;
use crate::hotkey::shortcuts;
use crate::state::AppState;

/// Stops the shortcuts from acting while a new one is being recorded.
#[tauri::command]
#[specta::specta]
pub async fn set_hotkey_paused(
    state: State<'_, AppState>,
    paused: bool,
) -> Result<(), UserFacingError> {
    let config = state.config.get();
    for (action, shortcut) in shortcuts(&config) {
        let (previous, next) = if paused {
            (shortcut, "")
        } else {
            ("", shortcut)
        };
        state.hotkeys.replace(action, previous, next)?;
    }
    Ok(())
}
