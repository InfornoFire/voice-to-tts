//! Turns text into speech.

use std::path::Path;

use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKittenModelConfig,
    OfflineTtsModelConfig,
};

use super::lexicon::Lexicon;
use super::path_text;
use crate::engine::speakable::speakable;
use crate::engine::{SpeechError, Tts};
use crate::models::Voice;

const MODEL_KIND: &str = "speech synthesis";

/// Speech synthesis with the Kitten model, which reads text through an English lexicon.
///
/// Words the lexicon lacks are spelled out or read as the words they are made of, because the
/// model would skip them.
pub struct SherpaTts {
    tts: OfflineTts,
    lexicon: Lexicon,
    voices: Vec<Voice>,
}

impl SherpaTts {
    /// Loads the Kitten model in `model_dir`, using at most `threads` threads.
    ///
    /// The first of the `voices` is the default.
    pub fn kitten(
        model_dir: &Path,
        lexicon: &Path,
        voices: Vec<Voice>,
        threads: u8,
    ) -> Result<Self, SpeechError> {
        let file = |name: &str| path_text(&model_dir.join(name), MODEL_KIND).map(Some);
        let config = OfflineTtsConfig {
            model: OfflineTtsModelConfig {
                kitten: OfflineTtsKittenModelConfig {
                    model: file("model.int8.onnx")?,
                    voices: file("voices.bin")?,
                    tokens: file("tokens.txt")?,
                    lexicon: Some(path_text(lexicon, MODEL_KIND)?),
                    ..Default::default()
                },
                num_threads: i32::from(threads),
                ..Default::default()
            },
            ..Default::default()
        };
        Ok(Self {
            tts: OfflineTts::create(&config).ok_or(SpeechError::Load(MODEL_KIND))?,
            lexicon: Lexicon::load(lexicon, MODEL_KIND)?,
            voices,
        })
    }

    fn speaker(&self, voice: &str) -> i32 {
        let known = self.voices.iter().find(|known| known.id == voice);
        known
            .or(self.voices.first())
            .map_or(0, |voice| voice.speaker as i32)
    }
}

impl Tts for SherpaTts {
    fn sample_rate(&self) -> u32 {
        self.tts.sample_rate() as u32
    }

    fn synthesize(&mut self, text: &str, voice: &str, speed: f32) -> Result<Vec<f32>, SpeechError> {
        // C strings cannot hold a null character.
        let text = speakable(&text.replace('\0', ""), |word| self.lexicon.contains(word));
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let config = GenerationConfig {
            speed,
            sid: self.speaker(voice),
            ..Default::default()
        };
        let audio = self
            .tts
            .generate_with_config(&text, &config, None::<fn(&[f32], f32) -> bool>)
            .ok_or(SpeechError::Synthesis)?;
        Ok(audio.samples().to_vec())
    }
}
