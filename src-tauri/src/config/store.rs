//! Keeps the config in a JSON file.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::{Map, Value};

use super::Config;

/// Failure while saving the config.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not write the config file at {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not serialize the config")]
    Serialize(#[from] serde_json::Error),
}

/// Holds the current [`Config`] and keeps it in sync with a JSON file.
pub struct ConfigStore {
    path: PathBuf,
    current: Mutex<Config>,
}

impl ConfigStore {
    /// Loads the config at `path`, using defaults for anything missing.
    ///
    /// A file that cannot be parsed is moved to `<path>.bak`.
    pub fn load(path: PathBuf) -> Self {
        let config = read_or_default(&path);
        Self {
            path,
            current: Mutex::new(config),
        }
    }

    pub fn get(&self) -> Config {
        self.lock().clone()
    }

    /// Normalizes and saves `config`, returning the stored value.
    pub fn update(&self, config: Config) -> Result<Config, ConfigError> {
        let config = config.normalized();
        let mut current = self.lock();
        write_atomic(&self.path, &config)?;
        *current = config.clone();
        Ok(config)
    }

    fn lock(&self) -> MutexGuard<'_, Config> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn read_or_default(path: &Path) -> Config {
    let Ok(contents) = fs::read(path) else {
        return Config::default();
    };

    match parse(&contents) {
        Ok(config) => config.normalized(),
        Err(_) => {
            let _ = fs::rename(path, path.with_added_extension("bak"));
            Config::default()
        }
    }
}

fn parse(contents: &[u8]) -> Result<Config, serde_json::Error> {
    let stored: Map<String, Value> = serde_json::from_slice(contents)?;
    let mut merged = serde_json::to_value(Config::default())?;
    if let Value::Object(fields) = &mut merged {
        fields.extend(stored);
    }
    serde_json::from_value(merged)
}

fn write_atomic(path: &Path, config: &Config) -> Result<(), ConfigError> {
    let contents = serde_json::to_vec_pretty(config)?;
    let write_error = |source: io::Error| ConfigError::Write {
        path: path.to_owned(),
        source,
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(write_error)?;
    }

    let temporary_path = path.with_added_extension("tmp");
    fs::write(&temporary_path, contents).map_err(write_error)?;
    fs::rename(&temporary_path, path).map_err(|source| {
        let _ = fs::remove_file(&temporary_path);
        write_error(source)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MAX_THREADS;

    fn config_path(directory: &tempfile::TempDir) -> PathBuf {
        directory.path().join("config.json")
    }

    #[test]
    fn first_run_uses_defaults_and_creates_the_directory_on_save() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested").join("config.json");

        let store = ConfigStore::load(path.clone());
        assert_eq!(store.get(), Config::default());
        assert!(!path.exists());

        store.update(Config::default()).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn update_stores_the_normalized_config_across_reloads() {
        let directory = tempfile::tempdir().unwrap();
        let path = config_path(&directory);
        let store = ConfigStore::load(path.clone());

        let stored = store
            .update(Config {
                input_device: Some("wasapi:mic".to_owned()),
                hotkey: "Ctrl+Shift+T".to_owned(),
                tts_voice: "amy".to_owned(),
                tts_speed: 1.5,
                hear_voice: true,
                subtitles_enabled: true,
                subtitles_port: 9000,
                threads: 200,
                start_minimized: true,
                setup_guide_done: true,
            })
            .unwrap();

        assert_eq!(stored.threads, MAX_THREADS);
        assert_eq!(store.get(), stored);
        assert_eq!(ConfigStore::load(path.clone()).get(), stored);
        assert!(!path.with_added_extension("tmp").exists());
    }

    #[test]
    fn failed_update_keeps_the_previous_config() {
        let directory = tempfile::tempdir().unwrap();
        let blocker = directory.path().join("blocker");
        fs::write(&blocker, "not a directory").unwrap();
        let store = ConfigStore::load(blocker.join("config.json"));

        let result = store.update(Config {
            threads: 4,
            ..Config::default()
        });

        assert!(matches!(result, Err(ConfigError::Write { .. })));
        assert_eq!(store.get(), Config::default());
    }

    #[test]
    fn stored_file_is_merged_with_defaults_and_normalized() {
        let directory = tempfile::tempdir().unwrap();
        let path = config_path(&directory);
        fs::write(&path, r#"{ "removed_setting": true, "threads": 200 }"#).unwrap();

        let config = ConfigStore::load(path).get();

        assert_eq!(
            config,
            Config {
                threads: MAX_THREADS,
                ..Config::default()
            }
        );
    }

    #[test]
    fn unusable_file_is_backed_up_and_defaults_are_used() {
        let unusable = [
            "{ this is not json",
            "[1, 2, 3]",
            r#"{ "threads": "many" }"#,
        ];

        for contents in unusable {
            let directory = tempfile::tempdir().unwrap();
            let path = config_path(&directory);
            fs::write(&path, contents).unwrap();

            let store = ConfigStore::load(path.clone());

            assert_eq!(store.get(), Config::default(), "{contents}");
            assert!(!path.exists(), "{contents}");
            assert_eq!(
                fs::read_to_string(path.with_added_extension("bak")).unwrap(),
                contents
            );
        }
    }
}
