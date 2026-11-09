//! Finds the speech in microphone audio.

use std::path::Path;

use sherpa_onnx::{SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};

use super::path_text;
use crate::engine::{INPUT_SAMPLE_RATE, SpeechError, Vad};

const MODEL_KIND: &str = "voice detection";
const THRESHOLD: f32 = 0.5;
const MIN_SPEECH_SECONDS: f32 = 0.2;
const WINDOW_SIZE: i32 = 512;

/// How long a pause lasts before the utterance ends.
///
/// A longer pause delays the voice, a shorter one cuts sentences in two more often.
const MIN_SILENCE_SECONDS: f32 = 0.6;

/// The longest stretch of speech the detector holds.
const BUFFER_SECONDS: f32 = 30.0;

/// Voice detection with the Silero model.
pub struct SileroVad {
    detector: VoiceActivityDetector,
}

impl SileroVad {
    /// Loads the model file at `model`.
    pub fn load(model: &Path) -> Result<Self, SpeechError> {
        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(path_text(model, MODEL_KIND)?),
                threshold: THRESHOLD,
                min_silence_duration: MIN_SILENCE_SECONDS,
                min_speech_duration: MIN_SPEECH_SECONDS,
                window_size: WINDOW_SIZE,
                max_speech_duration: BUFFER_SECONDS,
            },
            sample_rate: INPUT_SAMPLE_RATE as i32,
            num_threads: 1,
            ..Default::default()
        };
        let detector = VoiceActivityDetector::create(&config, BUFFER_SECONDS)
            .ok_or(SpeechError::Load(MODEL_KIND))?;
        Ok(Self { detector })
    }
}

impl Vad for SileroVad {
    fn accept(&mut self, samples: &[f32]) -> bool {
        self.detector.accept_waveform(samples);
        // The finished segments are not needed.
        self.detector.clear();
        self.detector.detected()
    }
}
