//! How the app lives in the tray while its main window is closed.

use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{AppHandle, Manager, Runtime, Webview, Window, WindowEvent};

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

/// Hides the main window instead of closing it, so the app keeps running in the tray.
pub fn hide_on_close<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event
        && window.label() == MAIN_WINDOW
    {
        api.prevent_close();
        let _ = window.hide();
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
