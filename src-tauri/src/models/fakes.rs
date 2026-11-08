//! Substitutes for the manifest, so the manager can be tested without the real one.

use super::{Manifest, Voice};
use crate::download::RemoteFile;

pub fn manifest(files: Vec<RemoteFile>) -> Manifest {
    Manifest {
        files,
        voices: vec![Voice {
            id: "a".to_owned(),
            name: "A".to_owned(),
            speaker: 0,
        }],
    }
}
