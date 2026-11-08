//! The speech model files, and getting them onto this machine.

mod download;
#[cfg(test)]
mod fakes;
mod manager;
mod manifest;
mod verify;

use std::io;

pub use download::{Fetch, HttpFetch};
pub use manager::{ModelManager, ModelsState, StateListener};
pub use manifest::{Manifest, ManifestError, ModelFile, Voice};

/// Failure to download the model files.
#[derive(Debug, thiserror::Error)]
pub enum ModelsError {
    #[error("could not download {path}")]
    Download {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("{path} does not have the expected contents")]
    Integrity { path: String },
    #[error("could not save {path}")]
    Write {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("the download was cancelled")]
    Cancelled,
    #[error("the download stopped unexpectedly")]
    Interrupted,
}
