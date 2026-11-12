//! The thread that turns requested text into speech.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;

use super::{POLL_INTERVAL, Request, Shared};
use crate::audio::AudioOutput;
use crate::engine::{EngineEvent, EngineState, Tts};

/// Turns each requested text into speech on the output, one clip after the other.
///
/// The next text is synthesized while the previous clip plays, so streamed phrases follow each
/// other without a gap. Waiting requests that continue one utterance are spoken as one text.
///
/// A request that was queued before a cancel is skipped. A clip that is being generated or played
/// when speech is cancelled is dropped, and the state is left to the listening thread.
pub(super) fn speak(
    shared: &Shared,
    requests: &Receiver<Request>,
    tts: Box<dyn Tts>,
    output: Box<dyn AudioOutput>,
) {
    let mut speaker = Speaker {
        shared,
        tts,
        output,
        playing: None,
        speaking: false,
        held: None,
    };
    speaker.serve(requests);
    if speaker.playing.is_some() {
        shared.sink.emit(EngineEvent::SpeechEnded);
    }
}

struct Speaker<'a> {
    shared: &'a Shared,
    tts: Box<dyn Tts>,
    output: Box<dyn AudioOutput>,
    /// How many times speech had been cancelled when the clip being played was queued.
    playing: Option<u32>,
    /// Whether this thread moved the engine to [`EngineState::Speaking`] and has not moved it back.
    speaking: bool,
    /// A request that was taken from the queue but could not be spoken together with the one before.
    held: Option<Request>,
}

impl Speaker<'_> {
    fn serve(&mut self, requests: &Receiver<Request>) {
        while !self.shared.is_stopped() {
            let request = match self.held.take() {
                Some(request) => request,
                None => match requests.recv_timeout(POLL_INTERVAL) {
                    Ok(request) => request,
                    Err(RecvTimeoutError::Timeout) => {
                        self.rest();
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                },
            };
            if !self.handle(requests, request) {
                return;
            }
        }
    }

    /// Speaks `first`, and what is queued behind it that continues it. Returns whether the thread
    /// goes on.
    fn handle(&mut self, requests: &Receiver<Request>, first: Request) -> bool {
        let Request {
            mut text,
            continues,
            cancellations,
        } = first;
        if self.shared.is_cancelled(cancellations) {
            return true;
        }
        self.append_waiting(requests, &mut text, cancellations);
        if !self.enter_speaking() {
            return true;
        }

        let config = (self.shared.settings)();
        let samples = match self
            .tts
            .synthesize(&text, &config.tts_voice, config.tts_speed)
        {
            Ok(samples) => samples,
            Err(error) => {
                log::warn!("could not turn text into speech: {error:?}");
                return true;
            }
        };
        if self.shared.is_cancelled(cancellations) {
            return true;
        }

        self.wait_for_clip();
        if self.shared.is_stopped() {
            return false;
        }
        if self.shared.is_cancelled(cancellations) {
            return true;
        }

        if let Err(error) = self.output.play(&samples, self.tts.sample_rate()) {
            self.shared.fail(error.into());
            return false;
        }
        self.shared
            .sink
            .emit(EngineEvent::SpeechStarted { text, continues });
        self.playing = Some(cancellations);
        true
    }

    /// Adds the queued requests that continue `text` to it, and holds the first one that does not.
    fn append_waiting(
        &mut self,
        requests: &Receiver<Request>,
        text: &mut String,
        cancellations: u32,
    ) {
        while let Ok(next) = requests.try_recv() {
            if next.continues && next.cancellations == cancellations {
                text.push(' ');
                text.push_str(&next.text);
            } else {
                self.held = Some(next);
                break;
            }
        }
    }

    fn wait_for_clip(&mut self) {
        self.check_clip();
        while self.playing.is_some() && !self.shared.is_stopped() {
            thread::sleep(POLL_INTERVAL);
            self.check_clip();
        }
    }

    /// Moves the engine to [`EngineState::Speaking`], returning whether it is there.
    ///
    /// It is not when the engine is paused or no longer running.
    fn enter_speaking(&mut self) -> bool {
        let state = &self.shared.state;
        let entered =
            state.transition(EngineState::Speaking) || state.get() == EngineState::Speaking;
        self.speaking |= entered;
        entered
    }

    /// Looks at the clip being played, and reports it when it ended or was cancelled.
    fn check_clip(&mut self) {
        let Some(cancellations) = self.playing else {
            return;
        };
        if self.shared.is_cancelled(cancellations) {
            self.output.stop();
            self.shared.sink.emit(EngineEvent::SpeechCancelled);
            self.playing = None;
        } else if self.output.is_idle() {
            self.shared.sink.emit(EngineEvent::SpeechEnded);
            self.playing = None;
        }
    }

    /// Called while nothing is waiting to be spoken, and listens again once the last clip ended.
    ///
    /// After a cancel the listening thread decides when listening goes on.
    fn rest(&mut self) {
        self.check_clip();
        if self.speaking && self.playing.is_none() && !self.shared.is_paused() {
            self.speaking = false;
            self.shared
                .state
                .transition_from(&EngineState::Speaking, EngineState::Listening);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU32};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::config::Config;
    use crate::engine::fakes::{FakeResources, LetterTts, speech_started, state_changed};
    use crate::engine::{OutputTarget, Resources, StateMachine};

    #[test]
    fn waiting_requests_of_one_utterance_are_spoken_together_and_stale_ones_are_skipped() {
        let (sender, events) = mpsc::channel();
        let sink = Arc::new(sender);
        let state = Arc::new(StateMachine::new(sink.clone()));
        state.transition(EngineState::Loading);
        state.transition(EngineState::Listening);
        let shared = Shared {
            stopped: AtomicBool::new(false),
            cancellations: AtomicU32::new(1),
            resume_at: Mutex::new(Instant::now()),
            cancel_pause: Duration::ZERO,
            state,
            sink,
            settings: Arc::new(Config::default),
            utterance_ids: Arc::default(),
            on_fault: Box::new(|error| panic!("{error:?}")),
        };
        let resources = FakeResources::default();
        let output = resources.open_output(OutputTarget::DefaultOutput).unwrap();
        let tts = LetterTts {
            spoken: Arc::clone(&resources.spoken),
        };
        let (requests, receiver) = mpsc::channel();
        let queued = [
            ("old", false, 0),
            ("one", false, 1),
            ("two", true, 1),
            ("three", true, 1),
            ("typed", false, 1),
            ("four", true, 1),
        ];
        for (text, continues, cancellations) in queued {
            let text = text.to_owned();
            requests
                .send(Request {
                    text,
                    continues,
                    cancellations,
                })
                .unwrap();
        }
        drop(requests);

        speak(&shared, &receiver, Box::new(tts), output);

        assert_eq!(
            *resources.spoken.lock().unwrap(),
            ["one two three", "typed four"]
        );
        assert_eq!(
            events.try_iter().collect::<Vec<_>>(),
            [
                state_changed(EngineState::Loading),
                state_changed(EngineState::Listening),
                state_changed(EngineState::Speaking),
                speech_started("one two three", false),
                EngineEvent::SpeechEnded,
                speech_started("typed four", false),
                EngineEvent::SpeechEnded,
            ]
        );
    }
}
