//! The engine that turns speech from the microphone into speech on the virtual microphone.

mod event;
mod mock;
mod state;

pub use event::{EngineEvent, EventSink, Transcript};
pub use mock::MockEngine;
pub use state::{EngineState, StateMachine};

/// Failure to carry out a request made to the engine.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("the engine is not running")]
    NotRunning,
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
}
