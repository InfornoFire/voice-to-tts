use std::path::Path;

use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKittenModelConfig,
    OfflineTtsKokoroModelConfig, OfflineTtsModelConfig,
};

use super::path_text;
use crate::engine::{SpeechError, Tts};

const MODEL_KIND: &str = "speech synthesis";

/// Speech synthesis with a model that reads text through an English lexicon.
pub struct SherpaTts {
    tts: OfflineTts,
}

impl SherpaTts {
    /// Loads the Kokoro model in `model_dir`, using at most `threads` threads.
    pub fn kokoro(model_dir: &Path, lexicon: &Path, threads: u8) -> Result<Self, SpeechError> {
        let file = |name: &str| path_text(&model_dir.join(name), MODEL_KIND).map(Some);
        let model = OfflineTtsModelConfig {
            kokoro: OfflineTtsKokoroModelConfig {
                model: file("model.int8.onnx")?,
                voices: file("voices.bin")?,
                tokens: file("tokens.txt")?,
                lexicon: Some(path_text(lexicon, MODEL_KIND)?),
                ..Default::default()
            },
            num_threads: i32::from(threads),
            ..Default::default()
        };
        Self::create(model)
    }

    /// Loads the Kitten model in `model_dir`, using at most `threads` threads.
    pub fn kitten(model_dir: &Path, lexicon: &Path, threads: u8) -> Result<Self, SpeechError> {
        let file = |name: &str| path_text(&model_dir.join(name), MODEL_KIND).map(Some);
        let model = OfflineTtsModelConfig {
            kitten: OfflineTtsKittenModelConfig {
                model: file("model.int8.onnx")?,
                voices: file("voices.bin")?,
                tokens: file("tokens.txt")?,
                lexicon: Some(path_text(lexicon, MODEL_KIND)?),
                ..Default::default()
            },
            num_threads: i32::from(threads),
            ..Default::default()
        };
        Self::create(model)
    }

    fn create(model: OfflineTtsModelConfig) -> Result<Self, SpeechError> {
        let config = OfflineTtsConfig {
            model,
            ..Default::default()
        };
        let tts = OfflineTts::create(&config).ok_or(SpeechError::Load(MODEL_KIND))?;
        Ok(Self { tts })
    }
}

impl Tts for SherpaTts {
    fn sample_rate(&self) -> u32 {
        self.tts.sample_rate() as u32
    }

    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, SpeechError> {
        // C strings cannot hold a null character.
        let text = text.replace('\0', "");
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let config = GenerationConfig {
            speed,
            ..Default::default()
        };
        let audio = self
            .tts
            .generate_with_config(&text, &config, None::<fn(&[f32], f32) -> bool>)
            .ok_or(SpeechError::Synthesis)?;
        Ok(audio.samples().to_vec())
    }
}
