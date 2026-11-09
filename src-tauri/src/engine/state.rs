//! The state of the engine and the moves allowed between states.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use specta::Type;

use super::{EngineEvent, EventSink};
use crate::error::UserFacingError;

/// What the engine is doing right now.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "status", content = "error", rename_all = "snake_case")]
pub enum EngineState {
    Off,
    /// Opening the microphone and loading the models.
    Loading,
    Listening,
    /// Playing synthesized speech to the virtual microphone.
    Speaking,
    Error(UserFacingError),
}

impl EngineState {
    /// Whether the engine is starting or up.
    pub fn is_on(&self) -> bool {
        matches!(self, Self::Loading | Self::Listening | Self::Speaking)
    }

    /// Whether the engine is up and can take text to speak.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Listening | Self::Speaking)
    }

    fn can_become(&self, next: &Self) -> bool {
        matches!(
            (self, next),
            (Self::Off | Self::Error(_), Self::Loading)
                | (Self::Loading, Self::Listening)
                | (Self::Listening, Self::Speaking)
                | (Self::Speaking, Self::Listening)
                | (
                    Self::Loading | Self::Listening | Self::Speaking,
                    Self::Error(_)
                )
                | (
                    Self::Loading | Self::Listening | Self::Speaking | Self::Error(_),
                    Self::Off
                )
        )
    }
}

/// Holds the [`EngineState`], allows only valid transitions and reports each change.
pub struct StateMachine {
    current: Mutex<EngineState>,
    sink: Arc<dyn EventSink>,
}

impl StateMachine {
    pub fn new(sink: Arc<dyn EventSink>) -> Self {
        Self {
            current: Mutex::new(EngineState::Off),
            sink,
        }
    }

    pub fn get(&self) -> EngineState {
        self.lock().clone()
    }

    /// Moves to `next` and reports it, returning whether the move was valid.
    ///
    /// Events are emitted under the lock, so they arrive in the order the states changed.
    pub fn transition(&self, next: EngineState) -> bool {
        let mut current = self.lock();
        if !current.can_become(&next) {
            return false;
        }
        *current = next.clone();
        self.sink.emit(EngineEvent::StateChanged(next));
        true
    }

    fn lock(&self) -> MutexGuard<'_, EngineState> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::engine::EngineError;

    #[test]
    fn only_valid_transitions_are_applied_and_reported() {
        let (sender, events) = mpsc::channel();
        let machine = StateMachine::new(Arc::new(sender));
        let failure = EngineState::Error(EngineError::NotRunning.into());
        let attempts = [
            (EngineState::Listening, false),
            (EngineState::Loading, true),
            (EngineState::Listening, true),
            (EngineState::Listening, false),
            (failure, true),
            (EngineState::Off, true),
        ];

        for (next, expected_valid) in &attempts {
            assert_eq!(machine.transition(next.clone()), *expected_valid);
        }

        let expected_events: Vec<_> = attempts
            .iter()
            .filter(|(_, valid)| *valid)
            .map(|(next, _)| EngineEvent::StateChanged(next.clone()))
            .collect();
        assert_eq!(events.try_iter().collect::<Vec<_>>(), expected_events);
        assert_eq!(machine.get(), EngineState::Off);
    }
}
