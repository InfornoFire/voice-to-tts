//! State shared between Tauri commands.

use crate::config::ConfigStore;
use crate::engine::Engine;
use crate::hotkey::Hotkeys;

pub struct AppState {
    pub config: ConfigStore,
    pub engine: Box<dyn Engine>,
    pub hotkeys: Box<dyn Hotkeys>,
}
