//! The notification area icon that shows the engine state and offers quick actions.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Runtime};

use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::shell;

const OPEN_ID: &str = "open";
const TOGGLE_ID: &str = "toggle";
const QUIT_ID: &str = "quit";

/// Keeps the tray icon in step with the engine state.
pub struct TraySink<R: Runtime> {
    tray: TrayIcon<R>,
    toggle_item: MenuItem<R>,
}

impl<R: Runtime> EventSink for TraySink<R> {
    fn emit(&self, event: EngineEvent) {
        if let EngineEvent::StateChanged(state) = event {
            let toggle_label = if state.is_on() { "Turn off" } else { "Turn on" };
            let _ = self.toggle_item.set_text(toggle_label);
            let _ = self.tray.set_tooltip(Some(tooltip(&state)));
        }
    }
}

/// Adds the tray icon to the notification area.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TraySink<R>> {
    let open_item = MenuItem::with_id(app, OPEN_ID, "Open", true, None::<&str>)?;
    let toggle_item = MenuItem::with_id(app, TOGGLE_ID, "Turn on", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, QUIT_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &toggle_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(tooltip(&EngineState::Off))
        .on_menu_event(|app, event| match event.id.as_ref() {
            OPEN_ID => shell::show_main_window(app),
            TOGGLE_ID => shell::toggle_engine(app),
            QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                shell::show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    Ok(TraySink {
        tray: builder.build(app)?,
        toggle_item,
    })
}

fn tooltip(state: &EngineState) -> &'static str {
    match state {
        EngineState::Off => "voice-to-tts: Off",
        EngineState::Loading => "voice-to-tts: Getting ready",
        EngineState::Listening => "voice-to-tts: Listening",
        EngineState::Speaking => "voice-to-tts: Speaking",
        EngineState::Error(_) => "voice-to-tts: Needs attention",
    }
}
