//! State shared between Tauri commands.

use std::sync::Arc;

use crate::config::ConfigStore;
use crate::engine::Engine;
use crate::hotkey::Hotkeys;
use crate::models::ModelManager;
use crate::shell::Shell;
use crate::subtitles::Subtitles;

pub struct AppState {
    pub config: Arc<ConfigStore>,
    pub engine: Box<dyn Engine>,
    pub hotkeys: Box<dyn Hotkeys>,
    pub models: Arc<ModelManager>,
    pub shell: Box<dyn Shell>,
    pub subtitles: Arc<Subtitles>,
}
