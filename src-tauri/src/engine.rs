//! The engine that turns speech from the microphone into speech on an audio output.

mod event;
#[cfg(test)]
mod fakes;
mod listener;
mod local;
#[cfg(any(test, feature = "mock-engine"))]
mod mock;
mod pipeline;
mod resources;
mod session;
mod sherpa;
mod speakable;
mod state;
mod stt;
mod tts;
mod vad;

pub use event::{EngineEvent, EventSink, FanOutSink, Transcript};
pub use listener::{Heard, Listener};
pub use local::LocalResources;
#[cfg(feature = "mock-engine")]
pub use mock::MockEngine;
pub use pipeline::PipelineEngine;
pub use resources::{Models, OutputTarget, Resources};
pub use sherpa::{SherpaTts, SileroVad, StreamingModel, StreamingStt};
pub use state::{EngineState, StateMachine};
pub use stt::Stt;
pub use tts::Tts;
pub use vad::Vad;

use crate::audio::devices::AudioError;

/// The sample rate of the audio given to a [`Vad`] and an [`Stt`].
pub const INPUT_SAMPLE_RATE: u32 = 16_000;

/// Failure to carry out a request made to the engine.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("the engine is not running")]
    NotRunning,
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error(transparent)]
    Speech(#[from] SpeechError),
}

/// Failure of a speech model.
#[derive(Debug, thiserror::Error)]
pub enum SpeechError {
    #[error("could not load the {0} model")]
    Load(&'static str),
    #[error("could not turn the text into speech")]
    Synthesis,
}

/// The controls the app has over an engine.
///
/// Changes happen in the background and are reported as [`EngineEvent::StateChanged`].
pub trait Engine: Send + Sync {
    /// Starts or stops the engine. Repeating the current request does nothing.
    fn set_enabled(&self, enabled: bool) -> Result<(), EngineError>;

    /// Speaks `text` as if it had been said into the microphone.
    fn speak_text(&self, text: &str) -> Result<(), EngineError>;

    fn state(&self) -> EngineState;

    /// Switches the engine off when it is on and on when it is off.
    fn toggle(&self) -> Result<(), EngineError> {
        self.set_enabled(!self.state().is_on())
    }
}
