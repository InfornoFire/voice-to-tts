pub mod commands;
pub mod config;
pub mod error;
pub mod state;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands};

use crate::config::ConfigStore;
use crate::state::AppState;

/// The commands and events shared by the invoke handler and the TypeScript bindings.
fn specta_builder<R: tauri::Runtime>() -> Builder<R> {
    Builder::<R>::new()
        .disable_serde_phases()
        .commands(collect_commands![
            commands::config::get_config,
            commands::config::update_config,
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
            app.manage(AppState {
                config: ConfigStore::load(config_path),
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
