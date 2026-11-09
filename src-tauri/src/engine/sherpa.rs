//! Speech models run by sherpa-onnx.

mod lexicon;
mod stt;
mod tts;
mod vad;

use std::path::Path;

use super::SpeechError;

pub use stt::{StreamingModel, StreamingStt};
pub use tts::SherpaTts;
pub use vad::SileroVad;

/// The text of `path`, which sherpa-onnx takes as a string, or a load failure of `model`.
fn path_text(path: &Path, model: &'static str) -> Result<String, SpeechError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or(SpeechError::Load(model))
}
