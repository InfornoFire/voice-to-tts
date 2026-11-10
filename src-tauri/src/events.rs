//! Events the backend pushes to the frontend.

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

use crate::engine::{EngineEvent, EngineState, EventSink, Transcript};
use crate::models::ModelsState;

#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct StateChanged(pub EngineState);

#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct ModelsStateChanged(pub ModelsState);

#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct TranscriptPartial(pub Transcript);

#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct TranscriptFinal(pub Transcript);

/// Delivers each new state of the models to the frontend.
pub fn models_state_listener<R: Runtime>(
    app: AppHandle<R>,
) -> impl Fn(&ModelsState) + Send + Sync + 'static {
    move |state| {
        // Nothing can be done about a webview that is gone, and the download goes on.
        let _ = ModelsStateChanged(state.clone()).emit(&app);
    }
}

/// Delivers engine events to the frontend.
pub struct TauriEventSink<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriEventSink<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> EventSink for TauriEventSink<R> {
    fn emit(&self, event: EngineEvent) {
        let delivered = match event {
            EngineEvent::StateChanged(state) => StateChanged(state).emit(&self.app),
            EngineEvent::TranscriptPartial(transcript) => {
                TranscriptPartial(transcript).emit(&self.app)
            }
            EngineEvent::TranscriptFinal(transcript) => TranscriptFinal(transcript).emit(&self.app),
            EngineEvent::SpeechStarted(_) | EngineEvent::SpeechEnded => Ok(()),
        };
        // Nothing can be done about a webview that is gone, and the engine keeps running.
        let _ = delivered;
    }
}
