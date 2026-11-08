//! The speech model files, and getting them onto this machine.

#[cfg(test)]
mod fakes;
mod manager;
mod manifest;

use crate::download::DownloadError;

pub use manager::{ModelManager, ModelsState, StateListener};
pub use manifest::{Manifest, ManifestError, Voice};

/// Failure to get the model files.
#[derive(Debug, thiserror::Error)]
pub enum ModelsError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error("the download stopped unexpectedly")]
    Interrupted,
}
