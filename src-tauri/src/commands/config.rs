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
#[tauri::command]
#[specta::specta]
pub async fn update_config(
    state: State<'_, AppState>,
    config: Config,
) -> Result<Config, UserFacingError> {
    Ok(state.config.update(config)?)
}
