//! Commands for the speech models.

use std::sync::Arc;

use tauri::State;

use crate::error::UserFacingError;
use crate::models::{ModelsError, ModelsState, Voice};
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn get_models_state(state: State<'_, AppState>) -> ModelsState {
    state.models.state()
}

/// Downloads the missing or damaged model files, resolving with the state it ends in.
///
/// Progress is reported through state events.
#[tauri::command]
#[specta::specta]
pub async fn download_models(state: State<'_, AppState>) -> Result<ModelsState, UserFacingError> {
    let models = Arc::clone(&state.models);
    tauri::async_runtime::spawn_blocking(move || models.download())
        .await
        .map_err(|_| ModelsError::Interrupted.into())
}

/// Stops a running download.
#[tauri::command]
#[specta::specta]
pub fn cancel_models_download(state: State<'_, AppState>) {
    state.models.cancel();
}

/// Lists the voices the speech can be spoken in, the first being the default.
#[tauri::command]
#[specta::specta]
pub fn list_voices(state: State<'_, AppState>) -> Vec<Voice> {
    state.models.voices().to_vec()
}
