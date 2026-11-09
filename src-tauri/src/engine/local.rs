use std::path::PathBuf;

use super::{
    INPUT_SAMPLE_RATE, Models, OutputTarget, Resources, SherpaTts, SileroVad, SpeechError,
    StreamingModel, StreamingStt,
};
use crate::audio::devices::{AudioError, check_cable, is_virtual_microphone};
use crate::audio::microphone::Microphone;
use crate::audio::playback::Playback;
use crate::audio::{AudioInput, AudioOutput};
use crate::config::Config;
use crate::models::Voice;

/// The directory of the speech recognition model within the models directory.
pub(super) const RECOGNIZER_DIR: &str = "sherpa-onnx-nemotron-speech-streaming-en-0.6b-560ms-int8";

pub(super) const RECOGNIZER_MODEL: StreamingModel = StreamingModel {
    encoder: "encoder.int8.onnx",
    decoder: "decoder.int8.onnx",
    joiner: "joiner.int8.onnx",
    tokens: "tokens.txt",
    tail_padding_seconds: 0.3,
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

    fn is_virtual_microphone(&self, device_id: Option<&str>) -> bool {
        is_virtual_microphone(device_id)
    }

    fn open_input(&self, device_id: Option<&str>) -> Result<Box<dyn AudioInput>, AudioError> {
        Ok(Box::new(Microphone::open(device_id, INPUT_SAMPLE_RATE)?))
    }

    fn open_output(&self, target: OutputTarget) -> Result<Box<dyn AudioOutput>, AudioError> {
        let playback = match target {
            OutputTarget::VirtualMicrophone => Playback::virtual_microphone()?,
            OutputTarget::DefaultOutput => Playback::default_output()?,
        };
        Ok(Box::new(playback))
    }

    fn load_models(&self, config: &Config) -> Result<Models, SpeechError> {
        let models = &self.models_dir;
        let recognizer = models.join(RECOGNIZER_DIR);
        let voice = models.join("kitten-nano-en-v0_8-int8");
        let lexicon = models.join("lexicon-en-us.txt");
        Ok(Models {
            vad: Box::new(SileroVad::load(&models.join("silero_vad.onnx"))?),
            stt: Box::new(StreamingStt::load(
                &recognizer,
                RECOGNIZER_MODEL,
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
