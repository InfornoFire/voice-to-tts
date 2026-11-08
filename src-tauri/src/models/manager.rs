use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::Serialize;
use specta::Type;

use super::download::{Fetch, fetch_file};
use super::verify::{is_intact, is_present};
use super::{Manifest, ModelFile, ModelsError, Voice};
use crate::error::UserFacingError;

/// How often the progress of a download is reported.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Whether the model files are ready to use.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ModelsState {
    Ready,
    /// Some files are missing, or have the wrong size.
    Missing {
        /// What is left to download in bytes.
        #[specta(type = u32)]
        bytes: u64,
    },
    Downloading {
        #[specta(type = u32)]
        downloaded: u64,
        #[specta(type = u32)]
        total: u64,
    },
    Failed {
        error: UserFacingError,
    },
}

/// Receives every new [`ModelsState`].
pub type StateListener = Box<dyn Fn(&ModelsState) + Send + Sync>;

/// Keeps the model files in a directory complete and reports their [`ModelsState`].
pub struct ModelManager {
    dir: PathBuf,
    manifest: Manifest,
    fetch: Box<dyn Fetch>,
    state: Mutex<ModelsState>,
    on_change: StateListener,
    cancelled: AtomicBool,
}

impl ModelManager {
    /// Checks the files in `dir` by their size, without reading their contents.
    pub fn new(
        dir: PathBuf,
        manifest: Manifest,
        fetch: Box<dyn Fetch>,
        on_change: StateListener,
    ) -> Self {
        let state = scan(&dir, &manifest);
        Self {
            dir,
            manifest,
            fetch,
            state: Mutex::new(state),
            on_change,
            cancelled: AtomicBool::new(false),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn voices(&self) -> &[Voice] {
        &self.manifest.voices
    }

    pub fn state(&self) -> ModelsState {
        self.lock().clone()
    }

    /// Downloads every file that is missing or damaged, returning the state it ends in.
    ///
    /// Files are checked by their contents first, so this also repairs the models. It does
    /// nothing while another download runs.
    pub fn download(&self) -> ModelsState {
        if !self.begin() {
            return self.state();
        }
        let finished = match self.fetch_missing() {
            Ok(()) => ModelsState::Ready,
            Err(ModelsError::Cancelled) => scan(&self.dir, &self.manifest),
            Err(error) => {
                log::error!("could not download the models: {error:?}");
                ModelsState::Failed {
                    error: error.into(),
                }
            }
        };
        self.set(finished.clone());
        finished
    }

    /// Stops a running download after the bytes it is reading now.
    pub fn cancel(&self) {
        if matches!(*self.lock(), ModelsState::Downloading { .. }) {
            self.cancelled.store(true, Ordering::Relaxed);
        }
    }

    fn begin(&self) -> bool {
        let mut state = self.lock();
        if matches!(*state, ModelsState::Downloading { .. }) {
            return false;
        }
        self.cancelled.store(false, Ordering::Relaxed);
        *state = ModelsState::Downloading {
            downloaded: 0,
            total: self.manifest.total_size(),
        };
        (self.on_change)(&state);
        true
    }

    fn fetch_missing(&self) -> Result<(), ModelsError> {
        let missing: Vec<&ModelFile> = self
            .manifest
            .files
            .iter()
            .filter(|file| !is_intact(&self.dir.join(&file.path), file))
            .collect();
        let total = missing.iter().map(|file| file.size).sum();
        self.set(ModelsState::Downloading {
            downloaded: 0,
            total,
        });

        let mut downloaded = 0;
        let mut last_report = Instant::now();
        for file in missing {
            let mut report = |bytes: u64| {
                downloaded += bytes;
                if last_report.elapsed() >= PROGRESS_INTERVAL {
                    last_report = Instant::now();
                    self.set(ModelsState::Downloading { downloaded, total });
                }
            };
            fetch_file(
                self.fetch.as_ref(),
                &self.dir,
                file,
                &self.cancelled,
                &mut report,
            )?;
        }
        Ok(())
    }

    /// Moves to `next` and reports it.
    ///
    /// Events are sent under the lock, so they arrive in the order the states changed.
    fn set(&self, next: ModelsState) {
        let mut state = self.lock();
        *state = next;
        (self.on_change)(&state);
    }

    fn lock(&self) -> MutexGuard<'_, ModelsState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn scan(dir: &Path, manifest: &Manifest) -> ModelsState {
    let bytes = manifest
        .files
        .iter()
        .filter(|file| !is_present(&dir.join(&file.path), file))
        .map(|file| file.size)
        .sum();
    if bytes == 0 {
        ModelsState::Ready
    } else {
        ModelsState::Missing { bytes }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    use super::*;
    use crate::error::ErrorCode;
    use crate::models::HttpFetch;
    use crate::models::fakes::{FakeFetch, listed_file, manifest};

    fn manager_in(
        dir: &Path,
        files: Vec<ModelFile>,
        fetch: &FakeFetch,
    ) -> (ModelManager, Arc<Mutex<Vec<ModelsState>>>) {
        let states = Arc::new(Mutex::new(Vec::new()));
        let listener = {
            let states = Arc::clone(&states);
            Box::new(move |state: &ModelsState| states.lock().unwrap().push(state.clone()))
        };
        let manager = ModelManager::new(
            dir.to_owned(),
            manifest(files),
            Box::new(fetch.clone()),
            listener,
        );
        (manager, states)
    }

    #[test]
    fn download_fetches_only_the_files_that_are_missing_or_damaged() {
        let dir = tempfile::tempdir().unwrap();
        let intact = listed_file("intact.bin", b"intact");
        let damaged = listed_file("nested/damaged.bin", b"damaged");
        let missing = listed_file("missing.bin", b"missing");
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join(&intact.path), b"intact").unwrap();
        fs::write(dir.path().join(&damaged.path), b"DAMAGED").unwrap();
        let fetch = FakeFetch::default();
        fetch.serve(&damaged, b"damaged");
        fetch.serve(&missing, b"missing");
        let files = vec![intact, damaged.clone(), missing.clone()];
        let (manager, states) = manager_in(dir.path(), files.clone(), &fetch);

        // Damage that keeps the size is only found by reading the file.
        assert_eq!(
            manager.state(),
            ModelsState::Missing {
                bytes: missing.size
            }
        );

        let finished = manager.download();

        assert_eq!(finished, ModelsState::Ready);
        assert_eq!(
            fetch.requested(),
            [damaged.url.clone(), missing.url.clone()]
        );
        assert_eq!(
            fs::read(dir.path().join(&damaged.path)).unwrap(),
            b"damaged"
        );
        let states = states.lock().unwrap();
        let total = damaged.size + missing.size;
        assert!(states.contains(&ModelsState::Downloading {
            downloaded: 0,
            total
        }));
        assert_eq!(states.last(), Some(&ModelsState::Ready));
        assert_eq!(
            manager_in(dir.path(), files, &fetch).0.state(),
            ModelsState::Ready
        );
    }

    #[test]
    fn a_download_that_does_not_match_its_listing_is_not_kept() {
        let listed = listed_file("model.bin", b"expected");
        let bodies: [Option<&[u8]>; 4] = [
            Some(b"EXPECTED"),
            Some(b"expected and more"),
            Some(b"exp"),
            None,
        ];

        for body in bodies {
            let dir = tempfile::tempdir().unwrap();
            let fetch = FakeFetch::default();
            if let Some(body) = body {
                fetch.serve(&listed, body);
            }
            let (manager, _) = manager_in(dir.path(), vec![listed.clone()], &fetch);

            let finished = manager.download();

            let ModelsState::Failed { error } = &finished else {
                panic!("the download ended as {finished:?}");
            };
            assert_eq!(error.code, ErrorCode::ModelsDownloadFailed);
            assert_eq!(manager.state(), finished);
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
        }
    }

    #[test]
    #[ignore = "downloads all the models from the internet"]
    fn the_bundled_manifest_downloads_and_verifies() {
        let dir = tempfile::tempdir().unwrap();
        let manager = ModelManager::new(
            dir.path().to_owned(),
            Manifest::bundled(),
            Box::new(HttpFetch::new()),
            Box::new(|_| {}),
        );

        assert!(matches!(manager.state(), ModelsState::Missing { .. }));
        assert_eq!(manager.download(), ModelsState::Ready);
    }
}
