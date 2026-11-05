pub mod audio;
pub mod commands;
pub mod config;
pub mod engine;
pub mod error;
pub mod events;
pub mod state;

use std::sync::Arc;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::audio::devices::check_cable;
use crate::config::ConfigStore;
use crate::engine::MockEngine;
use crate::events::TauriEventSink;
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
    let builder = specta_builder::<tauri::Wry>();

    #[cfg(debug_assertions)]
    export_bindings(&builder);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let config_path = app.path().app_config_dir()?.join("config.json");
            let events = Arc::new(TauriEventSink::new(app.handle().clone()));
            app.manage(AppState {
                config: ConfigStore::load(config_path),
                engine: Box::new(MockEngine::new(events, check_cable)),
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
