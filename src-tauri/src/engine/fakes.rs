//! Substitutes for the models and audio devices, so the engine can be tested without them.
//!
//! Audio is speech where its first sample is positive and silence where it is zero.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{
    EngineEvent, EngineState, INPUT_SAMPLE_RATE, Models, OutputTarget, Resources, SpeechError, Stt,
    Transcript, Tts, Vad,
};
use crate::audio::devices::AudioError;
use crate::audio::{AudioInput, AudioOutput};
use crate::config::Config;

/// The next event an engine reports, which should come soon.
pub fn next_event(events: &Receiver<EngineEvent>) -> EngineEvent {
    events
        .recv_timeout(Duration::from_secs(5))
        .expect("the engine should keep producing events")
}

pub fn state_changed(state: EngineState) -> EngineEvent {
    EngineEvent::StateChanged(state)
}

pub fn transcript(utterance_id: u32, text: &str) -> Transcript {
    Transcript {
        utterance_id,
        text: text.to_owned(),
    }
}

pub fn speech_started(text: &str, continues: bool) -> EngineEvent {
    EngineEvent::SpeechStarted {
        text: text.to_owned(),
        continues,
    }
}

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

    fn reset(&mut self) {
        self.heard.lock().unwrap().clear();
    }
}

/// Recognizes one more word of a sentence each time it is given audio, and all of them at the end.
pub struct SentenceStt {
    words: Vec<&'static str>,
    heard: usize,
}

impl SentenceStt {
    pub fn new(words: Vec<&'static str>) -> Self {
        Self { words, heard: 0 }
    }
}

impl Stt for SentenceStt {
    fn accept(&mut self, _samples: &[f32]) -> String {
        self.heard = (self.heard + 1).min(self.words.len());
        self.words[..self.heard].join(" ")
    }

    fn finish(&mut self) -> String {
        self.heard = 0;
        self.words.join(" ")
    }

    fn reset(&mut self) {
        self.heard = 0;
    }
}

/// Speaks one sample of audio for each letter, and records the texts it was given.
#[derive(Default)]
pub struct LetterTts {
    pub spoken: Arc<Mutex<Vec<String>>>,
}

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
        self.spoken.lock().unwrap().push(text.to_owned());
        Ok(vec![0.5; text.len()])
    }
}

/// A script for the microphone and a record of what the virtual microphone played.
#[derive(Clone)]
pub struct FakeResources {
    pub cable_installed: bool,
    pub models_installed: bool,
    pub microphone_installed: bool,
    /// Whether the microphone is the output of the virtual microphone.
    pub microphone_is_virtual: bool,
    /// The words of the sentence the recognizer hears, one more each time it is given audio. The
    /// recognizer always hears the same two words when this is empty.
    pub words: Vec<&'static str>,
    /// The texts the voice was asked to speak.
    pub spoken: Arc<Mutex<Vec<String>>>,
    /// The audio the microphone will hear, one entry for each time it is read.
    pub heard: Arc<Mutex<VecDeque<Vec<f32>>>>,
    pub microphone_lost: Arc<AtomicBool>,
    pub microphone_open: Arc<AtomicBool>,
    pub played: Arc<Mutex<Vec<Vec<f32>>>>,
    /// Whether the outputs report that they finished playing. Playing never finishes when unset.
    pub output_idle: Arc<AtomicBool>,
    /// How many times the outputs were stopped.
    pub stops: Arc<AtomicUsize>,
    /// Where speech was asked to be played, one entry for each output that was opened.
    pub outputs: Arc<Mutex<Vec<OutputTarget>>>,
}

impl Default for FakeResources {
    fn default() -> Self {
        Self {
            cable_installed: true,
            models_installed: true,
            microphone_installed: true,
            microphone_is_virtual: false,
            words: Vec::new(),
            spoken: Arc::default(),
            heard: Arc::default(),
            microphone_lost: Arc::default(),
            microphone_open: Arc::default(),
            played: Arc::default(),
            output_idle: Arc::new(AtomicBool::new(true)),
            stops: Arc::default(),
            outputs: Arc::default(),
        }
    }
}

impl Resources for FakeResources {
    fn is_cable_installed(&self) -> bool {
        self.cable_installed
    }

    fn is_virtual_microphone(&self, _device_id: Option<&str>) -> bool {
        self.microphone_is_virtual
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

    fn open_output(&self, target: OutputTarget) -> Result<Box<dyn AudioOutput>, AudioError> {
        if target == OutputTarget::VirtualMicrophone && !self.cable_installed {
            return Err(AudioError::CableMissing);
        }
        self.outputs.lock().unwrap().push(target);
        Ok(Box::new(FakeOutput {
            played: Arc::clone(&self.played),
            idle: Arc::clone(&self.output_idle),
            stops: Arc::clone(&self.stops),
        }))
    }

    fn load_models(&self, _config: &Config) -> Result<Models, SpeechError> {
        if !self.models_installed {
            return Err(SpeechError::Load("test"));
        }
        let stt: Box<dyn Stt> = if self.words.is_empty() {
            Box::new(RecordingStt::default())
        } else {
            Box::new(SentenceStt::new(self.words.clone()))
        };
        Ok(Models {
            vad: Box::new(LoudnessVad),
            stt,
            tts: Box::new(LetterTts {
                spoken: Arc::clone(&self.spoken),
            }),
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
    idle: Arc<AtomicBool>,
    stops: Arc<AtomicUsize>,
}

impl AudioOutput for FakeOutput {
    fn play(&mut self, samples: &[f32], _sample_rate: u32) -> Result<(), AudioError> {
        self.played.lock().unwrap().push(samples.to_vec());
        Ok(())
    }

    fn is_idle(&self) -> bool {
        self.idle.load(Ordering::Relaxed)
    }

    fn stop(&mut self) {
        self.stops.fetch_add(1, Ordering::Relaxed);
    }
}
