//! The global shortcuts that switch the engine on and off and cancel the speech.

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::config::Config;
use crate::shell;

/// Failure to register a global shortcut.
#[derive(Debug, thiserror::Error)]
pub enum HotkeyError {
    #[error("the shortcut is invalid or already used by another app")]
    Unavailable(#[source] tauri_plugin_global_shortcut::Error),
    #[error("both actions were given the same shortcut")]
    Duplicate,
}

/// What a global shortcut does when it is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    /// Switches the engine on or off.
    Toggle,
    /// Stops the voice and pauses listening for a moment.
    CancelSpeech,
}

/// Registers the global shortcuts.
pub trait Hotkeys: Send + Sync {
    /// Registers `next` for `action` in place of `previous`, where an empty shortcut means none.
    ///
    /// When `next` cannot be registered, `previous` stays registered.
    fn replace(&self, action: HotkeyAction, previous: &str, next: &str) -> Result<(), HotkeyError>;
}

/// Registers the shortcuts of `next` in place of those of `previous`.
///
/// The shortcuts being given up are released first, so one can move from an action to the other.
/// When one of the new shortcuts cannot be used, the previous ones are registered again.
pub fn replace_all(
    hotkeys: &dyn Hotkeys,
    previous: &Config,
    next: &Config,
) -> Result<(), HotkeyError> {
    if !next.hotkey.is_empty() && next.hotkey.eq_ignore_ascii_case(&next.cancel_hotkey) {
        return Err(HotkeyError::Duplicate);
    }

    for ((action, previous_shortcut), (_, next_shortcut)) in
        shortcuts(previous).into_iter().zip(shortcuts(next))
    {
        if previous_shortcut != next_shortcut {
            let _ = hotkeys.replace(action, previous_shortcut, "");
        }
    }
    let result = shortcuts(next)
        .into_iter()
        .try_for_each(|(action, shortcut)| hotkeys.replace(action, "", shortcut));
    if result.is_err() {
        for (action, shortcut) in shortcuts(next) {
            let _ = hotkeys.replace(action, shortcut, "");
        }
        for (action, shortcut) in shortcuts(previous) {
            let _ = hotkeys.replace(action, "", shortcut);
        }
    }
    result
}

/// Registers the shortcuts of `config`, when none is registered yet.
pub fn register_all(hotkeys: &dyn Hotkeys, config: &Config) -> Result<(), HotkeyError> {
    let none = Config {
        hotkey: String::new(),
        cancel_hotkey: String::new(),
        ..config.clone()
    };
    replace_all(hotkeys, &none, config)
}

/// The shortcut of each action in `config`.
pub fn shortcuts(config: &Config) -> [(HotkeyAction, &str); 2] {
    [
        (HotkeyAction::Toggle, &config.hotkey),
        (HotkeyAction::CancelSpeech, &config.cancel_hotkey),
    ]
}

/// Registers the shortcuts with the operating system.
pub struct GlobalHotkeys<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> GlobalHotkeys<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> Hotkeys for GlobalHotkeys<R> {
    fn replace(&self, action: HotkeyAction, previous: &str, next: &str) -> Result<(), HotkeyError> {
        let shortcuts = self.app.global_shortcut();
        if previous != next && !previous.is_empty() {
            let _ = shortcuts.unregister(previous);
        }
        if next.is_empty() || shortcuts.is_registered(next) {
            return Ok(());
        }

        shortcuts
            .on_shortcut(next, move |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    match action {
                        HotkeyAction::Toggle => shell::toggle_engine(app),
                        HotkeyAction::CancelSpeech => shell::cancel_speech(app),
                    }
                }
            })
            .map_err(|error| {
                if !previous.is_empty() && previous != next {
                    let _ = self.replace(action, "", previous);
                }
                HotkeyError::Unavailable(error)
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::HotkeyAction::{CancelSpeech, Toggle};
    use super::*;

    const REFUSED: &str = "Ctrl+Alt+Z";

    /// Keeps the registered shortcuts, and refuses one of them like the operating system can.
    #[derive(Default)]
    struct FakeHotkeys {
        registered: Mutex<Vec<(HotkeyAction, String)>>,
    }

    impl FakeHotkeys {
        /// The registered shortcuts in the order of their keys.
        fn registered(&self) -> Vec<(HotkeyAction, String)> {
            let mut registered = self.registered.lock().unwrap().clone();
            registered.sort_by(|first, second| first.1.cmp(&second.1));
            registered
        }
    }

    impl Hotkeys for FakeHotkeys {
        fn replace(
            &self,
            action: HotkeyAction,
            previous: &str,
            next: &str,
        ) -> Result<(), HotkeyError> {
            let mut registered = self.registered.lock().unwrap();
            if previous != next {
                registered.retain(|(_, shortcut)| shortcut != previous);
            }
            if next.is_empty() || registered.iter().any(|(_, shortcut)| shortcut == next) {
                return Ok(());
            }
            if next == REFUSED {
                let refused =
                    tauri_plugin_global_shortcut::Error::GlobalHotkey("refused".to_owned());
                return Err(HotkeyError::Unavailable(refused));
            }
            registered.push((action, next.to_owned()));
            Ok(())
        }
    }

    fn config(hotkey: &str, cancel_hotkey: &str) -> Config {
        Config {
            hotkey: hotkey.to_owned(),
            cancel_hotkey: cancel_hotkey.to_owned(),
            ..Config::default()
        }
    }

    fn registered_with(initial: &Config) -> FakeHotkeys {
        let hotkeys = FakeHotkeys::default();
        register_all(&hotkeys, initial).unwrap();
        hotkeys
    }

    #[test]
    fn a_shortcut_can_move_to_the_other_action() {
        let before = config("Ctrl+Alt+A", "Ctrl+Alt+B");
        let hotkeys = registered_with(&before);

        replace_all(&hotkeys, &before, &config("Ctrl+Alt+B", "Ctrl+Alt+A")).unwrap();

        assert_eq!(
            hotkeys.registered(),
            [
                (CancelSpeech, "Ctrl+Alt+A".to_owned()),
                (Toggle, "Ctrl+Alt+B".to_owned())
            ]
        );
    }

    #[test]
    fn shortcuts_that_cannot_be_used_leave_the_previous_ones_registered() {
        let before = config("Ctrl+Alt+A", "Ctrl+Alt+B");
        let refused = [
            config("Ctrl+Alt+C", REFUSED),
            config("Ctrl+Alt+C", "ctrl+alt+c"),
        ];

        for next in refused {
            let hotkeys = registered_with(&before);

            let result = replace_all(&hotkeys, &before, &next);

            assert!(result.is_err());
            assert_eq!(
                hotkeys.registered(),
                [
                    (Toggle, "Ctrl+Alt+A".to_owned()),
                    (CancelSpeech, "Ctrl+Alt+B".to_owned())
                ]
            );
        }
    }
}
