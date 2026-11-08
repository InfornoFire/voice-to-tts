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

/// The audio devices and models a running engine uses.
pub trait Resources: Send + Sync {
    /// Succeeds when the virtual microphone is installed.
    fn check_cable(&self) -> Result<(), AudioError>;

    /// Opens the microphone with `device_id`, or the system default when `None`.
    ///
    /// The audio is mono at [`INPUT_SAMPLE_RATE`](super::INPUT_SAMPLE_RATE).
    fn open_input(&self, device_id: Option<&str>) -> Result<Box<dyn AudioInput>, AudioError>;

    /// Opens the virtual microphone, the only place speech is played.
    fn open_output(&self) -> Result<Box<dyn AudioOutput>, AudioError>;

    fn load_models(&self, config: &Config) -> Result<Models, SpeechError>;
}
