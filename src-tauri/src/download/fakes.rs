//! Substitutes for the network, so downloads can be tested without it.

use std::collections::HashMap;
use std::io::{self, Cursor, Read};
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

use super::verify::hex_digest;
use super::{Fetch, RemoteFile};

/// Serves fixed bodies by URL and remembers which URLs were asked for.
#[derive(Clone, Default)]
pub struct FakeFetch {
    bodies: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    requested: Arc<Mutex<Vec<String>>>,
}

impl FakeFetch {
    /// Makes `body` the answer for the URL of `file`.
    pub fn serve(&self, file: &RemoteFile, body: &[u8]) {
        self.bodies
            .lock()
            .unwrap()
            .insert(file.url.clone(), body.to_vec());
    }

    pub fn requested(&self) -> Vec<String> {
        self.requested.lock().unwrap().clone()
    }
}

impl Fetch for FakeFetch {
    fn open(&self, url: &str) -> io::Result<Box<dyn Read + Send>> {
        self.requested.lock().unwrap().push(url.to_owned());
        match self.bodies.lock().unwrap().get(url) {
            Some(body) => Ok(Box::new(Cursor::new(body.clone()))),
            None => Err(io::ErrorKind::NotFound.into()),
        }
    }
}

/// A listing for a file with `contents`.
pub fn listed_file(path: &str, contents: &[u8]) -> RemoteFile {
    let mut hasher = Sha256::new();
    hasher.update(contents);
    RemoteFile {
        path: path.to_owned(),
        url: format!("https://example.com/{path}"),
        sha256: hex_digest(hasher),
        size: contents.len() as u64,
    }
}
