//! What the engine reports to the app, and who receives it.

use std::sync::Arc;

use serde::Serialize;
use specta::Type;

use super::EngineState;

/// Recognized speech of one utterance.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct Transcript {
    /// Partial transcripts of the same utterance replace each other until the final one.
    pub utterance_id: u32,
    pub text: String,
}

/// Something the engine reports to the app.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineEvent {
    StateChanged(EngineState),
    TranscriptPartial(Transcript),
    TranscriptFinal(Transcript),
}

/// Receives the events an engine produces.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: EngineEvent);
}

/// Passes every event to each of its sinks.
pub struct FanOutSink {
    sinks: Vec<Arc<dyn EventSink>>,
}

impl FanOutSink {
    pub fn new(sinks: Vec<Arc<dyn EventSink>>) -> Self {
        Self { sinks }
    }
}

impl EventSink for FanOutSink {
    fn emit(&self, event: EngineEvent) {
        for sink in &self.sinks {
            sink.emit(event.clone());
        }
    }
}

#[cfg(test)]
impl EventSink for std::sync::mpsc::Sender<EngineEvent> {
    fn emit(&self, event: EngineEvent) {
        let _ = self.send(event);
    }
}
