pub mod audio;
pub mod commands;
pub mod config;
pub mod engine;
pub mod error;
pub mod events;
pub mod hotkey;
pub mod platform;
pub mod shell;
pub mod state;
pub mod tray;

use std::sync::Arc;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::audio::devices::check_cable;
use crate::config::ConfigStore;
use crate::engine::{FanOutSink, MockEngine};
use crate::events::TauriEventSink;
use crate::hotkey::{GlobalHotkeys, Hotkeys};
use crate::state::AppState;

/// The commands and events shared by the invoke handler and the TypeScript bindings.
fn specta_builder<R: tauri::Runtime>() -> Builder<R> {
    Builder::<R>::new()
        .disable_serde_phases()
        .commands(collect_commands![
            commands::config::get_config,
            commands::config::update_config,
            commands::devices::check_cable,
            commands::devices::list_input_devices,
            commands::engine::get_engine_state,
            commands::engine::set_enabled,
            commands::engine::speak_text,
            commands::hotkey::set_hotkey_paused,
        ])
        .events(collect_events![
            events::StateChanged,
            events::TranscriptFinal,
            events::TranscriptPartial,
        ])
}

#[cfg(any(debug_assertions, test))]
fn export_bindings<R: tauri::Runtime>(builder: &Builder<R>) {
    builder
        .export(
            specta_typescript::Typescript::default(),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../src/ipc/bindings.ts"),
        )
        .expect("failed to export the TypeScript bindings");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    platform::lower_priority();

    let builder = specta_builder::<tauri::Wry>();

    #[cfg(debug_assertions)]
    export_bindings(&builder);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            shell::show_main_window(app);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .on_window_event(shell::hide_on_close)
        .on_page_load(shell::show_when_loaded)
        .setup(move |app| {
            builder.mount_events(app);

            let config = ConfigStore::load(app.path().app_config_dir()?.join("config.json"));
            let hotkey = config.get().hotkey;
            let events = Arc::new(FanOutSink::new(vec![
                Arc::new(TauriEventSink::new(app.handle().clone())),
                Arc::new(tray::create(app.handle())?),
            ]));
            let hotkeys = GlobalHotkeys::new(app.handle().clone());

            // A shortcut that cannot be registered here can be replaced in the settings.
            let _ = hotkeys.replace("", &hotkey);

            app.manage(AppState {
                config,
                engine: Box::new(MockEngine::new(events, check_cable)),
                hotkeys: Box::new(hotkeys),
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use tauri::test::MockRuntime;

    use super::{export_bindings, specta_builder};

    #[test]
    fn export_bindings_to_disk() {
        export_bindings(&specta_builder::<MockRuntime>());
    }
}
