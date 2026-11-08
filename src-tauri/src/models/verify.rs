use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::ModelFile;

/// The lowercase hexadecimal text of a finished hash.
pub fn hex_digest(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Whether `file` exists at `path` with the expected size, without reading its contents.
pub fn is_present(path: &Path, file: &ModelFile) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.len() == file.size)
}

/// Whether `file` exists at `path` with the expected size and contents.
pub fn is_intact(path: &Path, file: &ModelFile) -> bool {
    is_present(path, file) && hash_file(path).is_ok_and(|hash| hash == file.sha256)
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(hex_digest(hasher));
        }
        hasher.update(&buffer[..count]);
    }
}
