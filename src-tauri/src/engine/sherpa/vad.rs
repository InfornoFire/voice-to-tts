use std::path::Path;

use sherpa_onnx::{SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};

use super::{INPUT_SAMPLE_RATE, path_text};
use crate::engine::{SpeechError, Vad};

const MODEL_KIND: &str = "voice detection";
const BUFFER_SECONDS: f32 = 30.0;

/// Voice detection with the Silero model.
pub struct SileroVad {
    detector: VoiceActivityDetector,
}

impl SileroVad {
    pub fn load(model: &Path) -> Result<Self, SpeechError> {
        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(path_text(model, MODEL_KIND)?),
                threshold: 0.5,
                min_silence_duration: 0.4,
                min_speech_duration: 0.2,
                window_size: 512,
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

    fn reset(&mut self) {
        self.detector.reset();
    }
}
