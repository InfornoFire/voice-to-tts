//! How the app lives in the tray while its main window is closed.

use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{AppHandle, Manager, Runtime, Webview, Window, WindowEvent};
use tauri_specta::Event;

use crate::config::CloseAction;
use crate::events::CloseRequested;
use crate::state::AppState;

const MAIN_WINDOW: &str = "main";

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Toggles the engine from outside the window. A failure opens the window to show why.
pub fn toggle_engine<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if state.engine.toggle().is_err() {
        show_main_window(app);
    }
}

/// Hides the main window, and the app keeps running in the tray.
pub fn hide_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.hide();
    }
}

pub fn quit<R: Runtime>(app: &AppHandle<R>) {
    app.exit(0);
}

/// Lets commands hide the main window or end the app.
pub trait Shell: Send + Sync {
    fn hide_main_window(&self);

    fn quit(&self);
}

/// Controls the windows and the process of the running app.
pub struct AppShell<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> AppShell<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> Shell for AppShell<R> {
    fn hide_main_window(&self) {
        hide_main_window(&self.app);
    }

    fn quit(&self) {
        quit(&self.app);
    }
}

/// Carries out the close action when the main window is closed.
///
/// The window is never destroyed here. Asking leaves it open for the frontend to prompt in.
pub fn handle_close_request<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if window.label() != MAIN_WINDOW {
        return;
    }
    api.prevent_close();

    let app = window.app_handle();
    let action = app
        .try_state::<AppState>()
        .map_or(CloseAction::Tray, |state| state.config.get().close_action);
    match action {
        CloseAction::Ask => {
            if CloseRequested.emit(app).is_err() {
                hide_main_window(app);
            }
        }
        CloseAction::Tray => hide_main_window(app),
        CloseAction::Quit => quit(app),
    }
}

/// Shows the main window once its page has loaded, unless the app starts minimized.
pub fn show_when_loaded<R: Runtime>(webview: &Webview<R>, payload: &PageLoadPayload<'_>) {
    if payload.event() != PageLoadEvent::Finished || webview.label() != MAIN_WINDOW {
        return;
    }
    let app = webview.app_handle();
    if let Some(state) = app.try_state::<AppState>()
        && !state.config.get().start_minimized
    {
        show_main_window(app);
    }
}
