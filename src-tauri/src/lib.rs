pub mod audio;
pub mod commands;
pub mod config;
pub mod engine;
pub mod error;
pub mod events;
pub mod hotkey;
pub mod models;
pub mod platform;
pub mod shell;
pub mod state;
pub mod tray;

use std::sync::Arc;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::config::ConfigStore;
use crate::engine::{Engine, EventSink, FanOutSink};
use crate::events::TauriEventSink;
use crate::hotkey::{GlobalHotkeys, Hotkeys};
use crate::models::{HttpFetch, Manifest, ModelManager};
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
            commands::models::cancel_models_download,
            commands::models::download_models,
            commands::models::get_models_state,
            commands::models::list_voices,
        ])
        .events(collect_events![
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
    Box::new(engine::MockEngine::new(events, audio::devices::check_cable))
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
        .on_window_event(shell::hide_on_close)
        .on_page_load(shell::show_when_loaded)
        .setup(move |app| {
            builder.mount_events(app);

            let config = Arc::new(ConfigStore::load(
                app.path().app_config_dir()?.join("config.json"),
            ));
            let hotkey = config.get().hotkey;
            let events = Arc::new(FanOutSink::new(vec![
                Arc::new(TauriEventSink::new(app.handle().clone())),
                Arc::new(tray::create(app.handle())?),
            ]));
            let hotkeys = GlobalHotkeys::new(app.handle().clone());

            // A shortcut that cannot be registered here can be replaced in the settings.
            let _ = hotkeys.replace("", &hotkey);

            let models = Arc::new(create_models(app)?);
            app.manage(AppState {
                engine: create_engine(events, &config, &models),
                config,
                hotkeys: Box::new(hotkeys),
                models,
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
