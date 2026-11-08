use std::path::PathBuf;

use super::{
    INPUT_SAMPLE_RATE, Models, Resources, SherpaTts, SileroVad, SpeechError, ZipformerFiles,
    ZipformerStt,
};
use crate::audio::devices::{AudioError, check_cable};
use crate::audio::microphone::Microphone;
use crate::audio::playback::CableOutput;
use crate::audio::{AudioInput, AudioOutput};
use crate::config::Config;
use crate::models::Voice;

/// The directory of the speech recognition model within the models directory.
pub(super) const RECOGNIZER_DIR: &str = "sherpa-onnx-streaming-zipformer-en-kroko-2025-08-06";

pub(super) const RECOGNIZER_FILES: ZipformerFiles = ZipformerFiles {
    encoder: "encoder.onnx",
    decoder: "decoder.onnx",
    joiner: "joiner.onnx",
    tokens: "tokens.txt",
};

/// The audio devices of this machine and the models in a directory.
pub struct LocalResources {
    models_dir: PathBuf,
    voices: Vec<Voice>,
}

impl LocalResources {
    /// The first of the `voices` is the default.
    pub fn new(models_dir: PathBuf, voices: Vec<Voice>) -> Self {
        Self { models_dir, voices }
    }
}

impl Resources for LocalResources {
    fn check_cable(&self) -> Result<(), AudioError> {
        check_cable()
    }

    fn open_input(&self, device_id: Option<&str>) -> Result<Box<dyn AudioInput>, AudioError> {
        Ok(Box::new(Microphone::open(device_id, INPUT_SAMPLE_RATE)?))
    }

    fn open_output(&self) -> Result<Box<dyn AudioOutput>, AudioError> {
        Ok(Box::new(CableOutput::open()?))
    }

    fn load_models(&self, config: &Config) -> Result<Models, SpeechError> {
        let models = &self.models_dir;
        let recognizer = models.join(RECOGNIZER_DIR);
        let voice = models.join("kitten-nano-en-v0_8-int8");
        let lexicon = models.join("lexicon-en-us.txt");
        Ok(Models {
            vad: Box::new(SileroVad::load(&models.join("silero_vad.onnx"))?),
            stt: Box::new(ZipformerStt::load(
                &recognizer,
                RECOGNIZER_FILES,
                config.threads,
            )?),
            tts: Box::new(SherpaTts::kitten(
                &voice,
                &lexicon,
                self.voices.clone(),
                config.threads,
            )?),
        })
    }
}
