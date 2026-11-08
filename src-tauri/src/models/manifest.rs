use std::collections::HashSet;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use specta::Type;

/// A model file to download into the models directory.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelFile {
    /// Location within the models directory, using `/` as the separator.
    pub path: String,
    /// Where to download it from, always over HTTPS.
    pub url: String,
    /// Lowercase hexadecimal SHA-256 of the file.
    pub sha256: String,
    /// Size of the file in bytes.
    pub size: u64,
}

/// A voice of the speech synthesis model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Voice {
    /// Stable identifier to store in the config.
    pub id: String,
    pub name: String,
    /// The number the model knows the voice by.
    pub speaker: u32,
}

/// The files and voices the app needs.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub files: Vec<ModelFile>,
    /// The first voice is the default.
    pub voices: Vec<Voice>,
}

/// Failure to read a manifest.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("the manifest is not valid")]
    Parse(#[from] serde_json::Error),
    #[error("the manifest is not usable: {0}")]
    Invalid(String),
}

impl Manifest {
    /// The manifest shipped with the app.
    pub fn bundled() -> Self {
        Self::parse(include_str!("../../models.json"))
            .expect("the bundled manifest should be valid")
    }

    /// Reads a manifest and checks that every entry is safe to use.
    pub fn parse(json: &str) -> Result<Self, ManifestError> {
        let manifest: Self = serde_json::from_str(json)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// The total size of all the files in bytes.
    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|file| file.size).sum()
    }

    fn validate(&self) -> Result<(), ManifestError> {
        let invalid = |reason: String| Err(ManifestError::Invalid(reason));

        let mut paths = HashSet::new();
        for file in &self.files {
            if !is_contained_path(&file.path) {
                return invalid(format!(
                    "{} is not a path inside the models directory",
                    file.path
                ));
            }
            if !paths.insert(file.path.as_str()) {
                return invalid(format!("{} is listed twice", file.path));
            }
            if !file.url.starts_with("https://") {
                return invalid(format!("{} is not downloaded over HTTPS", file.path));
            }
            if !is_sha256(&file.sha256) {
                return invalid(format!("{} has no valid SHA-256", file.path));
            }
            if file.size == 0 {
                return invalid(format!("{} has no size", file.path));
            }
        }

        let mut ids = HashSet::new();
        if self.voices.is_empty() || !self.voices.iter().all(|voice| ids.insert(&voice.id)) {
            return invalid("the voices are missing or share an id".to_owned());
        }
        Ok(())
    }
}

/// Whether `path` stays inside the directory it is joined to.
fn is_contained_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':'])
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn manifest(path: &str, url: &str, sha256: &str, size: u64) -> String {
        format!(
            r#"{{
                "files": [{{ "path": "{path}", "url": "{url}", "sha256": "{sha256}", "size": {size} }}],
                "voices": [{{ "id": "a", "name": "A", "speaker": 0 }}]
            }}"#
        )
    }

    #[test]
    fn the_bundled_manifest_is_valid() {
        let manifest = Manifest::bundled();

        assert!(manifest.total_size() > 0);
    }

    #[test]
    fn entries_that_are_unsafe_or_malformed_are_rejected() {
        let url = "https://example.com/file";
        let invalid = [
            manifest("../model.onnx", url, DIGEST, 1),
            manifest("/model.onnx", url, DIGEST, 1),
            manifest("C:/model.onnx", url, DIGEST, 1),
            manifest("dir\\\\model.onnx", url, DIGEST, 1),
            manifest("", url, DIGEST, 1),
            manifest("model.onnx", "http://example.com/file", DIGEST, 1),
            manifest("model.onnx", url, &DIGEST.to_uppercase(), 1),
            manifest("model.onnx", url, "abc", 1),
            manifest("model.onnx", url, DIGEST, 0),
            r#"{ "files": [], "voices": [] }"#.to_owned(),
        ];

        assert!(Manifest::parse(&manifest("dir/model.onnx", url, DIGEST, 1)).is_ok());
        for json in invalid {
            assert!(Manifest::parse(&json).is_err(), "{json}");
        }
    }
}
