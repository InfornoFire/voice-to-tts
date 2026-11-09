//! The seam between the engine and the machine it runs on.

use super::{SpeechError, Stt, Tts, Vad};
use crate::audio::devices::AudioError;
use crate::audio::{AudioInput, AudioOutput};
use crate::config::Config;

/// The models that turn speech into text and text into speech.
pub struct Models {
    pub vad: Box<dyn Vad>,
    pub stt: Box<dyn Stt>,
    pub tts: Box<dyn Tts>,
}

/// An output that speech is played through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTarget {
    /// The virtual microphone, so a chat app hears it.
    VirtualMicrophone,
    /// The default audio output, which is never the virtual microphone.
    DefaultOutput,
}

/// The audio devices and models a running engine uses.
pub trait Resources: Send + Sync {
    /// Whether the virtual microphone is installed.
    fn is_cable_installed(&self) -> bool;

    /// Whether the microphone with `device_id`, or the system default when `None`, is the output of
    /// the virtual microphone.
    fn is_virtual_microphone(&self, device_id: Option<&str>) -> bool;

    /// Opens the microphone with `device_id`, or the system default when `None`.
    ///
    /// The audio is mono at [`INPUT_SAMPLE_RATE`](super::INPUT_SAMPLE_RATE).
    fn open_input(&self, device_id: Option<&str>) -> Result<Box<dyn AudioInput>, AudioError>;

    fn open_output(&self, target: OutputTarget) -> Result<Box<dyn AudioOutput>, AudioError>;

    fn load_models(&self, config: &Config) -> Result<Models, SpeechError>;
}
