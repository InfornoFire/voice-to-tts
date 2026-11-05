use tauri::State;

use crate::engine::EngineState;
use crate::error::UserFacingError;
use crate::state::AppState;

/// Turns the engine on or off. Progress is reported through state events.
#[tauri::command]
#[specta::specta]
pub async fn set_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), UserFacingError> {
    Ok(state.engine.set_enabled(enabled)?)
}

/// Speaks `text` through the virtual microphone.
#[tauri::command]
#[specta::specta]
pub async fn speak_text(state: State<'_, AppState>, text: String) -> Result<(), UserFacingError> {
    Ok(state.engine.speak_text(&text)?)
}

#[tauri::command]
#[specta::specta]
pub fn get_engine_state(state: State<'_, AppState>) -> EngineState {
    state.engine.state()
}
