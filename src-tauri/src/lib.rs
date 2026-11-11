//! A desktop app that speaks what is said into the microphone, optionally through a virtual
//! microphone.

// The scripted engine leaves the real engine and the audio code unused.
#![cfg_attr(feature = "mock-engine", allow(dead_code, unused_imports))]

mod audio;
mod commands;
mod config;
mod download;
mod engine;
mod error;
mod events;
mod hotkey;
mod models;
mod platform;
mod shell;
mod state;
mod subtitles;
mod tray;

use std::sync::Arc;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::config::ConfigStore;
use crate::download::HttpFetch;
use crate::engine::{Engine, EventSink, FanOutSink};
use crate::events::TauriEventSink;
use crate::hotkey::{GlobalHotkeys, Hotkeys};
use crate::models::{Manifest, ModelManager};
use crate::shell::AppShell;
use crate::state::AppState;
use crate::subtitles::Subtitles;

/// The commands and events shared by the invoke handler and the TypeScript bindings.
fn specta_builder<R: tauri::Runtime>() -> Builder<R> {
    Builder::<R>::new()
        .disable_serde_phases()
        .commands(collect_commands![
            commands::config::get_config,
            commands::config::update_config,
            commands::devices::is_cable_installed,
            commands::devices::list_input_devices,
            commands::engine::get_engine_state,
            commands::engine::set_enabled,
            commands::engine::speak_text,
            commands::hotkey::set_hotkey_paused,
            commands::models::cancel_models_download,
            commands::models::download_models,
            commands::models::get_models_state,
            commands::models::list_voices,
            commands::window::answer_close_request,
        ])
        .events(collect_events![
            events::CloseRequested,
            events::ModelsStateChanged,
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

/// Creates the manager of the model files in the app's data directory.
fn create_models(app: &tauri::App) -> tauri::Result<ModelManager> {
    let dir = app.path().app_local_data_dir()?.join("models");
    let manifest = Manifest::bundled();
    // The scripted engine needs no model files.
    #[cfg(feature = "mock-engine")]
    let manifest = Manifest {
        files: Vec::new(),
        ..manifest
    };
    Ok(ModelManager::new(
        dir,
        manifest,
        Box::new(HttpFetch::new()),
        Box::new(events::models_state_listener(app.handle().clone())),
    ))
}

/// Creates the engine that runs the speech models on this machine.
#[cfg(not(feature = "mock-engine"))]
fn create_engine(
    events: Arc<dyn EventSink>,
    config: &Arc<ConfigStore>,
    models: &ModelManager,
) -> Box<dyn Engine> {
    let config = Arc::clone(config);
    Box::new(engine::PipelineEngine::new(
        events,
        engine::LocalResources::new(models.dir().to_owned(), models.voices().to_vec()),
        move || config.get(),
    ))
}

/// Creates the engine that replays a script, without audio devices or models.
#[cfg(feature = "mock-engine")]
fn create_engine(
    events: Arc<dyn EventSink>,
    _config: &Arc<ConfigStore>,
    _models: &ModelManager,
) -> Box<dyn Engine> {
    Box::new(engine::MockEngine::new(events))
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
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .on_window_event(shell::handle_close_request)
        .on_page_load(shell::show_when_loaded)
        .setup(move |app| {
            builder.mount_events(app);

            let config = Arc::new(ConfigStore::load(
                app.path().app_config_dir()?.join("config.json"),
            ));
            let stored = config.get();
            let subtitles = Arc::new(Subtitles::default());
            let events = Arc::new(FanOutSink::new(vec![
                Arc::new(TauriEventSink::new(app.handle().clone())),
                Arc::new(tray::create(app.handle())?),
                Arc::clone(&subtitles) as Arc<dyn EventSink>,
            ]));
            let hotkeys = GlobalHotkeys::new(app.handle().clone());

            // A shortcut that cannot be registered here can be replaced in the settings.
            let _ = hotkeys.replace("", &stored.hotkey);
            // A port in use is reported when the user next saves the settings.
            if let Err(error) = subtitles.apply(stored.subtitles_enabled, stored.subtitles_port) {
                log::warn!("the subtitles server did not start: {error}");
            }

            let models = Arc::new(create_models(app)?);
            app.manage(AppState {
                engine: create_engine(events, &config, &models),
                config,
                hotkeys: Box::new(hotkeys),
                models,
                shell: Box::new(AppShell::new(app.handle().clone())),
                subtitles,
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
