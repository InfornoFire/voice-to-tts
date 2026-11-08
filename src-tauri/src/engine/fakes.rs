//! Substitutes for the models and audio devices, so the engine can be tested without them.
//!
//! Audio is speech where its first sample is positive and silence where it is zero.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::{INPUT_SAMPLE_RATE, Models, Resources, SpeechError, Stt, Tts, Vad};
use crate::audio::devices::AudioError;
use crate::audio::{AudioInput, AudioOutput};
use crate::config::Config;

pub struct LoudnessVad;

impl Vad for LoudnessVad {
    fn accept(&mut self, samples: &[f32]) -> bool {
        samples[0] > 0.0
    }

    fn reset(&mut self) {}
}

/// Recognizes the same words in any audio and records what it is given.
#[derive(Default)]
pub struct RecordingStt {
    pub heard: Arc<Mutex<Vec<f32>>>,
}

impl Stt for RecordingStt {
    fn accept(&mut self, samples: &[f32]) -> String {
        self.heard.lock().unwrap().extend(samples);
        "Hello".to_owned()
    }

    fn finish(&mut self) -> String {
        self.heard.lock().unwrap().clear();
        " Hello  world".to_owned()
    }
}

/// Speaks one sample of audio for each letter.
pub struct LetterTts;

impl Tts for LetterTts {
    fn sample_rate(&self) -> u32 {
        INPUT_SAMPLE_RATE
    }

    fn synthesize(
        &mut self,
        text: &str,
        _voice: &str,
        _speed: f32,
    ) -> Result<Vec<f32>, SpeechError> {
        Ok(vec![0.5; text.len()])
    }
}

/// A script for the microphone and a record of what the virtual microphone played.
#[derive(Clone)]
pub struct FakeResources {
    pub cable_installed: bool,
    pub models_installed: bool,
    pub microphone_installed: bool,
    /// The audio the microphone will hear, one entry for each time it is read.
    pub heard: Arc<Mutex<VecDeque<Vec<f32>>>>,
    pub microphone_lost: Arc<AtomicBool>,
    pub microphone_open: Arc<AtomicBool>,
    pub played: Arc<Mutex<Vec<Vec<f32>>>>,
}

impl Default for FakeResources {
    fn default() -> Self {
        Self {
            cable_installed: true,
            models_installed: true,
            microphone_installed: true,
            heard: Arc::default(),
            microphone_lost: Arc::default(),
            microphone_open: Arc::default(),
            played: Arc::default(),
        }
    }
}

impl Resources for FakeResources {
    fn check_cable(&self) -> Result<(), AudioError> {
        if self.cable_installed {
            Ok(())
        } else {
            Err(AudioError::CableMissing)
        }
    }

    fn open_input(&self, _device_id: Option<&str>) -> Result<Box<dyn AudioInput>, AudioError> {
        if !self.microphone_installed {
            return Err(AudioError::MicUnavailable);
        }
        self.microphone_open.store(true, Ordering::Relaxed);
        Ok(Box::new(FakeInput {
            heard: Arc::clone(&self.heard),
            lost: Arc::clone(&self.microphone_lost),
            open: Arc::clone(&self.microphone_open),
        }))
    }

    fn open_output(&self) -> Result<Box<dyn AudioOutput>, AudioError> {
        self.check_cable()?;
        Ok(Box::new(FakeOutput {
            played: Arc::clone(&self.played),
        }))
    }

    fn load_models(&self, _config: &Config) -> Result<Models, SpeechError> {
        if !self.models_installed {
            return Err(SpeechError::Load("test"));
        }
        Ok(Models {
            vad: Box::new(LoudnessVad),
            stt: Box::new(RecordingStt::default()),
            tts: Box::new(LetterTts),
        })
    }
}

struct FakeInput {
    heard: Arc<Mutex<VecDeque<Vec<f32>>>>,
    lost: Arc<AtomicBool>,
    open: Arc<AtomicBool>,
}

impl AudioInput for FakeInput {
    fn read(&mut self, samples: &mut Vec<f32>) {
        if let Some(chunk) = self.heard.lock().unwrap().pop_front() {
            samples.extend(chunk);
        }
    }

    fn has_failed(&self) -> bool {
        self.lost.load(Ordering::Relaxed)
    }
}

impl Drop for FakeInput {
    fn drop(&mut self) {
        self.open.store(false, Ordering::Relaxed);
    }
}

struct FakeOutput {
    played: Arc<Mutex<Vec<Vec<f32>>>>,
}

impl AudioOutput for FakeOutput {
    fn play(&mut self, samples: &[f32], _sample_rate: u32) -> Result<(), AudioError> {
        self.played.lock().unwrap().push(samples.to_vec());
        Ok(())
    }

    fn is_idle(&self) -> bool {
        true
    }
}
