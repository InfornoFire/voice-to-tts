//! State shared between Tauri commands.

use std::sync::Arc;

use crate::config::ConfigStore;
use crate::engine::Engine;
use crate::hotkey::Hotkeys;

pub struct AppState {
    pub config: Arc<ConfigStore>,
    pub engine: Box<dyn Engine>,
    pub hotkeys: Box<dyn Hotkeys>,
}
