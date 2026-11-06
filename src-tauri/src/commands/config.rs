use tauri::State;

use crate::config::Config;
use crate::error::UserFacingError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.get()
}

/// Saves `config` and returns the stored value.
///
/// A new shortcut is registered first, so nothing is saved when it is unavailable.
#[tauri::command]
#[specta::specta]
pub async fn update_config(
    state: State<'_, AppState>,
    config: Config,
) -> Result<Config, UserFacingError> {
    let config = config.normalized();
    state
        .hotkeys
        .replace(&state.config.get().hotkey, &config.hotkey)?;
    Ok(state.config.update(config)?)
}
