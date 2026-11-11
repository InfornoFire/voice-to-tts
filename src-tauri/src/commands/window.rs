//! Commands for the main window.

use tauri::State;

use crate::config::{CloseChoice, Config};
use crate::state::AppState;

/// Carries out the answer to the close prompt, and remembers it when asked to.
#[tauri::command]
#[specta::specta]
pub fn answer_close_request(state: State<'_, AppState>, choice: CloseChoice, remember: bool) {
    if remember {
        let config = Config {
            close_action: choice.into(),
            ..state.config.get()
        };
        // Not remembering only means the user is asked again.
        if let Err(error) = state.config.update(config) {
            log::warn!("the close choice was not remembered: {error}");
        }
    }

    match choice {
        CloseChoice::Tray => state.shell.hide_main_window(),
        CloseChoice::Quit => state.shell.quit(),
    }
}
