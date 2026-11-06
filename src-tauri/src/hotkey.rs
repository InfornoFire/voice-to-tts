//! The global shortcut that switches the engine on and off.

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::shell;

/// Failure to register a global shortcut.
#[derive(Debug, thiserror::Error)]
pub enum HotkeyError {
    #[error("the shortcut is invalid or already used by another app")]
    Unavailable(#[source] tauri_plugin_global_shortcut::Error),
}

/// Registers the global shortcut.
pub trait Hotkeys: Send + Sync {
    /// Registers `next` in place of `previous`, where an empty shortcut means none.
    ///
    /// When `next` cannot be registered, `previous` stays registered.
    fn replace(&self, previous: &str, next: &str) -> Result<(), HotkeyError>;
}

/// Registers the shortcut with the operating system.
pub struct GlobalHotkeys<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> GlobalHotkeys<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> Hotkeys for GlobalHotkeys<R> {
    fn replace(&self, previous: &str, next: &str) -> Result<(), HotkeyError> {
        let shortcuts = self.app.global_shortcut();
        if previous != next && !previous.is_empty() {
            let _ = shortcuts.unregister(previous);
        }
        if next.is_empty() || shortcuts.is_registered(next) {
            return Ok(());
        }

        shortcuts
            .on_shortcut(next, |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    shell::toggle_engine(app);
                }
            })
            .map_err(|error| {
                if !previous.is_empty() && previous != next {
                    let _ = self.replace("", previous);
                }
                HotkeyError::Unavailable(error)
            })
    }
}
