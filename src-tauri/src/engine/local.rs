use std::path::PathBuf;

use super::{
    INPUT_SAMPLE_RATE, Models, Resources, SherpaTts, SileroVad, SpeechError, ZipformerStt,
};
use crate::audio::devices::{AudioError, check_cable};
use crate::audio::microphone::Microphone;
use crate::audio::playback::CableOutput;
use crate::audio::{AudioInput, AudioOutput};
use crate::config::Config;

/// The audio devices of this machine and the models in a directory.
pub struct LocalResources {
    models_dir: PathBuf,
}

impl LocalResources {
    pub fn new(models_dir: PathBuf) -> Self {
        Self { models_dir }
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
        let recognizer = models.join("sherpa-onnx-streaming-zipformer-en-20M-2023-02-17");
        let voice = models.join("kitten-nano-en-v0_8-int8");
        let lexicon = models.join("lexicon-en-us.txt");
        Ok(Models {
            vad: Box::new(SileroVad::load(&models.join("silero_vad.onnx"))?),
            stt: Box::new(ZipformerStt::load(&recognizer, config.threads)?),
            tts: Box::new(SherpaTts::kitten(&voice, &lexicon, config.threads)?),
        })
    }
}
