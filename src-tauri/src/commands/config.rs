//! Commands for the settings.

use tauri::State;

use crate::config::Config;
use crate::error::UserFacingError;
use crate::hotkey;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.get()
}

/// Saves `config` and returns the stored value.
///
/// New shortcuts are registered and the subtitles server is started or stopped first, so nothing
/// is saved when a shortcut or the server's port is unavailable.
#[tauri::command]
#[specta::specta]
pub async fn update_config(
    state: State<'_, AppState>,
    config: Config,
) -> Result<Config, UserFacingError> {
    let config = config.normalized();
    hotkey::replace_all(state.hotkeys.as_ref(), &state.config.get(), &config)?;
    state
        .subtitles
        .apply(config.subtitles_enabled, config.subtitles_port)?;
    Ok(state.config.update(config)?)
}
