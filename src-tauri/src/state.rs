//! State shared between Tauri commands.

use crate::config::ConfigStore;

pub struct AppState {
    pub config: ConfigStore,
}
