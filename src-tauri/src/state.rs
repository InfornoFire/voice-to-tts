//! State shared between Tauri commands.

use crate::config::ConfigStore;
use crate::engine::Engine;

pub struct AppState {
    pub config: ConfigStore,
    pub engine: Box<dyn Engine>,
}
