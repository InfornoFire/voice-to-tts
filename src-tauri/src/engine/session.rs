//! The threads of an engine that is on, one listening and one speaking.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::{
    EngineError, EngineEvent, EngineState, EventSink, Heard, Listener, StateMachine, Transcript,
    Tts,
};
use crate::audio::devices::AudioError;
use crate::audio::{AudioInput, AudioOutput};
use crate::config::Config;

/// How often the threads look for new audio and for a request to stop.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Reads the current settings.
pub type Settings = Arc<dyn Fn() -> Config + Send + Sync>;

/// Called with the failure that ended a session.
pub type FaultHandler = Box<dyn Fn(EngineError) + Send + Sync>;

/// What a session is made of and reports to.
pub struct Parts {
    pub input: Box<dyn AudioInput>,
    pub output: Box<dyn AudioOutput>,
    pub listener: Listener,
    pub tts: Box<dyn Tts>,
    pub state: Arc<StateMachine>,
    pub sink: Arc<dyn EventSink>,
    pub settings: Settings,
    /// Where the ids of new utterances come from.
    pub utterance_ids: Arc<AtomicU32>,
    pub on_fault: FaultHandler,
}

/// What the two threads of a session share.
struct Shared {
    stopped: AtomicBool,
    state: Arc<StateMachine>,
    sink: Arc<dyn EventSink>,
    settings: Settings,
    utterance_ids: Arc<AtomicU32>,
    on_fault: FaultHandler,
}

impl Shared {
    fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    fn next_utterance_id(&self) -> u32 {
        self.utterance_ids.fetch_add(1, Ordering::Relaxed)
    }

    fn fail(&self, error: EngineError) {
        (self.on_fault)(error);
    }
}

/// The threads that listen to the microphone and speak, from the moment the engine is on.
///
/// Dropping the session stops them and releases the audio devices.
pub struct Session {
    shared: Arc<Shared>,
    requests: Option<Sender<String>>,
    threads: Vec<JoinHandle<()>>,
}

impl Session {
    pub fn start(parts: Parts) -> Self {
        let Parts {
            input,
            output,
            listener,
            tts,
            state,
            sink,
            settings,
            utterance_ids,
            on_fault,
        } = parts;
        let shared = Arc::new(Shared {
            stopped: AtomicBool::new(false),
            state,
            sink,
            settings,
            utterance_ids,
            on_fault,
        });
        let (requests, receiver) = mpsc::channel();

        let listening = thread::spawn({
            let shared = Arc::clone(&shared);
            let requests = requests.clone();
            move || listen(&shared, input, listener, &requests)
        });
        let speaking = thread::spawn({
            let shared = Arc::clone(&shared);
            move || speak(&shared, &receiver, tts, output)
        });

        Self {
            shared,
            requests: Some(requests),
            threads: vec![listening, speaking],
        }
    }

    /// Queues `text` to be spoken after what is already queued.
    pub fn say(&self, text: String) {
        if let Some(requests) = &self.requests {
            let _ = requests.send(text);
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Relaxed);
        self.requests = None;
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

/// Recognizes what the microphone hears and queues each finished utterance to be spoken.
fn listen(
    shared: &Shared,
    mut input: Box<dyn AudioInput>,
    mut listener: Listener,
    requests: &Sender<String>,
) {
    let mut samples = Vec::new();
    let mut utterance_id = None;

    while !shared.is_stopped() {
        thread::sleep(POLL_INTERVAL);
        if input.has_failed() {
            shared.fail(AudioError::MicUnavailable.into());
            return;
        }

        samples.clear();
        input.read(&mut samples);
        if samples.is_empty() {
            continue;
        }

        match listener.accept(&samples) {
            Some(Heard::Partial(text)) => {
                let utterance_id = *utterance_id.get_or_insert_with(|| shared.next_utterance_id());
                shared.sink.emit(EngineEvent::TranscriptPartial(Transcript {
                    utterance_id,
                    text,
                }));
            }
            Some(Heard::Final(text)) => {
                let started_id = utterance_id.take();
                if text.is_empty() {
                    continue;
                }
                let utterance_id = started_id.unwrap_or_else(|| shared.next_utterance_id());
                shared.sink.emit(EngineEvent::TranscriptFinal(Transcript {
                    utterance_id,
                    text: text.clone(),
                }));
                let _ = requests.send(text);
            }
            None => {}
        }
    }
}

/// Turns each requested text into speech on the output, one after the other.
fn speak(
    shared: &Shared,
    requests: &Receiver<String>,
    mut tts: Box<dyn Tts>,
    mut output: Box<dyn AudioOutput>,
) {
    for text in requests {
        if shared.is_stopped() {
            return;
        }

        shared.state.transition(EngineState::Speaking);
        let config = (shared.settings)();
        match tts.synthesize(&text, &config.tts_voice, config.tts_speed) {
            Ok(samples) => {
                if let Err(error) = output.play(&samples, tts.sample_rate()) {
                    shared.fail(error.into());
                    return;
                }
                while !output.is_idle() && !shared.is_stopped() {
                    thread::sleep(POLL_INTERVAL);
                }
            }
            Err(error) => log::warn!("could not turn text into speech: {error:?}"),
        }
        shared.state.transition(EngineState::Listening);
    }
}
