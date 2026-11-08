//! Downloading files over HTTPS and keeping only what matches its listing.

#[cfg(test)]
pub mod fakes;
mod verify;

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use verify::hex_digest;
pub use verify::{is_intact, is_present};

const BUFFER_SIZE: usize = 64 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const BODY_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// A file to download, and what it must look like once it is here.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteFile {
    /// Location within the directory it is downloaded into, using `/` as the separator.
    pub path: String,
    /// Where to download it from, always over HTTPS.
    pub url: String,
    /// Lowercase hexadecimal SHA-256 of the file.
    pub sha256: String,
    /// Size of the file in bytes.
    pub size: u64,
}

/// Failure to download a file.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
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
}

/// Where files are downloaded from.
pub trait Fetch: Send + Sync {
    /// Starts downloading `url` and returns its body.
    fn open(&self, url: &str) -> io::Result<Box<dyn Read + Send>>;
}

/// Downloads over HTTPS.
pub struct HttpFetch {
    agent: ureq::Agent,
}

impl HttpFetch {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .https_only(true)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_recv_response(Some(RESPONSE_TIMEOUT))
            .timeout_recv_body(Some(BODY_TIMEOUT))
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl Default for HttpFetch {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetch for HttpFetch {
    fn open(&self, url: &str) -> io::Result<Box<dyn Read + Send>> {
        let response = self.agent.get(url).call().map_err(ureq::Error::into_io)?;
        Ok(Box::new(response.into_body().into_reader()))
    }
}

/// Downloads `file` into `dir`, keeping it only when its size and contents are as listed.
///
/// The bytes are written next to the target and replace it once checked, so a failed
/// download leaves nothing behind. `on_bytes` is told how many bytes each read added.
pub fn fetch_file(
    fetch: &dyn Fetch,
    dir: &Path,
    file: &RemoteFile,
    cancelled: &AtomicBool,
    on_bytes: &mut dyn FnMut(u64),
) -> Result<(), DownloadError> {
    let target = dir.join(&file.path);
    let partial = target.with_added_extension("part");

    let result = save(fetch, &partial, file, cancelled, on_bytes)
        .and_then(|()| fs::rename(&partial, &target).map_err(|source| write_error(file, source)));
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

fn save(
    fetch: &dyn Fetch,
    partial: &Path,
    file: &RemoteFile,
    cancelled: &AtomicBool,
    on_bytes: &mut dyn FnMut(u64),
) -> Result<(), DownloadError> {
    let download_error = |source| DownloadError::Download {
        path: file.path.clone(),
        source,
    };

    if let Some(parent) = partial.parent() {
        fs::create_dir_all(parent).map_err(|source| write_error(file, source))?;
    }
    // One byte beyond the listed size shows that the body is longer than it should be.
    let mut body = fetch
        .open(&file.url)
        .map_err(download_error)?
        .take(file.size + 1);
    let mut output = File::create(partial).map_err(|source| write_error(file, source))?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0; BUFFER_SIZE];
    let mut received = 0;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }
        let count = body.read(&mut buffer).map_err(download_error)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        output
            .write_all(&buffer[..count])
            .map_err(|source| write_error(file, source))?;
        received += count as u64;
        on_bytes(count as u64);
    }

    if received != file.size || hex_digest(hasher) != file.sha256 {
        return Err(DownloadError::Integrity {
            path: file.path.clone(),
        });
    }
    Ok(())
}

fn write_error(file: &RemoteFile, source: io::Error) -> DownloadError {
    DownloadError::Write {
        path: file.path.clone(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::fakes::{FakeFetch, listed_file};

    #[test]
    fn cancelling_leaves_no_partial_file_behind() {
        let contents = vec![7; BUFFER_SIZE * 2];
        let listed = listed_file("model.bin", &contents);
        let fetch = FakeFetch::default();
        fetch.serve(&listed, &contents);
        let dir = tempfile::tempdir().unwrap();
        let cancelled = AtomicBool::new(false);

        let result = fetch_file(&fetch, dir.path(), &listed, &cancelled, &mut |_| {
            cancelled.store(true, Ordering::Relaxed);
        });

        assert!(matches!(result, Err(DownloadError::Cancelled)));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
