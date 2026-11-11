//! The threads of an engine that is on, one listening and one speaking.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
    /// How long the microphone and typed text are ignored after a cancel.
    pub cancel_pause: Duration,
    pub on_fault: FaultHandler,
}

/// A text to speak, and how many times speech had been cancelled when it was queued.
struct Request {
    text: String,
    cancellations: u32,
}

/// What the two threads of a session share.
struct Shared {
    stopped: AtomicBool,
    /// How many times speech was cancelled. A request queued before a cancel is never spoken.
    cancellations: AtomicU32,
    /// When the microphone is listened to again after the last cancel.
    resume_at: Mutex<Instant>,
    cancel_pause: Duration,
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

    fn request(&self, text: String) -> Request {
        Request {
            text,
            cancellations: self.cancellations.load(Ordering::Relaxed),
        }
    }

    /// Whether speech was cancelled since the counter read `queued_at`.
    fn is_cancelled(&self, queued_at: u32) -> bool {
        self.cancellations.load(Ordering::Relaxed) != queued_at
    }

    fn cancel(&self) {
        self.cancellations.fetch_add(1, Ordering::Relaxed);
        *self.lock_resume_at() = Instant::now() + self.cancel_pause;
    }

    fn is_paused(&self) -> bool {
        Instant::now() < *self.lock_resume_at()
    }

    fn lock_resume_at(&self) -> MutexGuard<'_, Instant> {
        self.resume_at
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// The threads that listen to the microphone and speak, from the moment the engine is on.
///
/// Dropping the session stops them and releases the audio devices.
pub struct Session {
    shared: Arc<Shared>,
    requests: Option<Sender<Request>>,
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
            cancel_pause,
            on_fault,
        } = parts;
        let shared = Arc::new(Shared {
            stopped: AtomicBool::new(false),
            cancellations: AtomicU32::new(0),
            resume_at: Mutex::new(Instant::now()),
            cancel_pause,
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
            let _ = requests.send(self.shared.request(text));
        }
    }

    /// Stops the voice, drops what is queued and the sentence being heard, and then ignores the
    /// microphone for a moment.
    ///
    /// The listening thread notices and moves the engine to [`EngineState::Paused`].
    pub fn cancel(&self) {
        self.shared.cancel();
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
///
/// This thread alone moves the engine into and out of [`EngineState::Paused`]. While paused the
/// microphone is still read, so no audio piles up, but it is thrown away.
fn listen(
    shared: &Shared,
    mut input: Box<dyn AudioInput>,
    mut listener: Listener,
    requests: &Sender<Request>,
) {
    let mut samples = Vec::new();
    let mut utterance_id = None;
    let mut was_paused = false;

    while !shared.is_stopped() {
        thread::sleep(POLL_INTERVAL);
        if input.has_failed() {
            shared.fail(AudioError::MicUnavailable.into());
            return;
        }

        samples.clear();
        input.read(&mut samples);

        let paused = shared.is_paused();
        if paused != was_paused {
            was_paused = paused;
            if paused {
                listener.reset();
                utterance_id = None;
                shared.state.transition(EngineState::Paused);
            } else {
                shared
                    .state
                    .transition_from(&EngineState::Paused, EngineState::Listening);
            }
        }
        if paused || samples.is_empty() {
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
                let _ = requests.send(shared.request(text));
            }
            None => {}
        }
    }
}

/// Turns each requested text into speech on the output, one after the other.
///
/// A request that was queued before a cancel is skipped. A clip that is being generated or played
/// when speech is cancelled is dropped, and the state is left to the listening thread.
fn speak(
    shared: &Shared,
    requests: &Receiver<Request>,
    mut tts: Box<dyn Tts>,
    mut output: Box<dyn AudioOutput>,
) {
    for Request {
        text,
        cancellations,
    } in requests
    {
        if shared.is_stopped() {
            return;
        }
        let is_cancelled = || shared.is_cancelled(cancellations);
        if is_cancelled() || !shared.state.transition(EngineState::Speaking) {
            continue;
        }

        let config = (shared.settings)();
        match tts.synthesize(&text, &config.tts_voice, config.tts_speed) {
            Ok(samples) => {
                if is_cancelled() {
                    continue;
                }
                if let Err(error) = output.play(&samples, tts.sample_rate()) {
                    shared.fail(error.into());
                    return;
                }
                shared.sink.emit(EngineEvent::SpeechStarted(text));
                while !output.is_idle() && !shared.is_stopped() && !is_cancelled() {
                    thread::sleep(POLL_INTERVAL);
                }
                if is_cancelled() {
                    output.stop();
                    shared.sink.emit(EngineEvent::SpeechCancelled);
                    continue;
                }
                shared.sink.emit(EngineEvent::SpeechEnded);
            }
            Err(error) => log::warn!("could not turn text into speech: {error:?}"),
        }
        shared
            .state
            .transition_from(&EngineState::Speaking, EngineState::Listening);
    }
}
