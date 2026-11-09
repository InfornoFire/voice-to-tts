//! Commands for the audio devices.

use crate::audio::devices::{self, InputDevice};
use crate::error::UserFacingError;

/// Lists the microphones the user can capture from.
#[tauri::command]
#[specta::specta]
pub async fn list_input_devices() -> Result<Vec<InputDevice>, UserFacingError> {
    Ok(devices::list_input_devices()?)
}

/// Whether the virtual microphone is installed.
#[tauri::command]
#[specta::specta]
pub async fn is_cable_installed() -> bool {
    devices::is_cable_installed()
}
