//! An engine that replays a script, so the app can run without audio devices or models.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use super::{Engine, EngineError, EngineEvent, EngineState, EventSink, StateMachine, Transcript};
use crate::audio::devices::AudioError;

const SENTENCES: &[&str] = &[
    "Hello this is a test sentence.",
    "One, two, three, four, five.",
    "Something something something.",
];

const PACING: Pacing = Pacing {
    load: Duration::from_millis(1200),
    gap: Duration::from_millis(2500),
    word: Duration::from_millis(350),
    speak: Duration::from_millis(1500),
};

/// How long each part of the script takes.
#[derive(Clone, Copy)]
struct Pacing {
    load: Duration,
    /// Silence before an utterance starts.
    gap: Duration,
    /// Time between the words of an utterance.
    word: Duration,
    speak: Duration,
}

enum Command {
    Enable,
    Disable,
    Speak,
}

enum Action {
    Loaded,
    Partial(Transcript),
    Final(Transcript),
    Spoken,
}

/// An action that runs once `delay` has passed without a new command.
struct Step {
    delay: Duration,
    action: Action,
}

/// An [`Engine`] that loads, listens and speaks on a timer.
///
/// All state changes happen on one worker thread, which sleeps while the engine is off.
pub struct MockEngine {
    state: Arc<StateMachine>,
    commands: Sender<Command>,
    check_cable: Box<dyn Fn() -> Result<(), AudioError> + Send + Sync>,
}

impl MockEngine {
    pub fn new(
        sink: Arc<dyn EventSink>,
        check_cable: impl Fn() -> Result<(), AudioError> + Send + Sync + 'static,
    ) -> Self {
        Self::with_script(sink, check_cable, SENTENCES, PACING)
    }

    fn with_script(
        sink: Arc<dyn EventSink>,
        check_cable: impl Fn() -> Result<(), AudioError> + Send + Sync + 'static,
        sentences: &'static [&'static str],
        pacing: Pacing,
    ) -> Self {
        let state = Arc::new(StateMachine::new(Arc::clone(&sink)));
        let (commands, receiver) = mpsc::channel();
        let worker = Worker {
            state: Arc::clone(&state),
            sink,
            sentences,
            pacing,
            steps: VecDeque::new(),
            next_sentence: 0,
            next_utterance_id: 0,
        };
        thread::spawn(move || worker.run(&receiver));
        Self {
            state,
            commands,
            check_cable: Box::new(check_cable),
        }
    }

    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

impl Engine for MockEngine {
    fn set_enabled(&self, enabled: bool) -> Result<(), EngineError> {
        if enabled {
            (self.check_cable)()?;
        }
        self.send(if enabled {
            Command::Enable
        } else {
            Command::Disable
        });
        Ok(())
    }

    fn speak_text(&self, _text: &str) -> Result<(), EngineError> {
        if !self.state.get().is_running() {
            return Err(EngineError::NotRunning);
        }
        self.send(Command::Speak);
        Ok(())
    }

    fn state(&self) -> EngineState {
        self.state.get()
    }
}

struct Worker {
    state: Arc<StateMachine>,
    sink: Arc<dyn EventSink>,
    sentences: &'static [&'static str],
    pacing: Pacing,
    steps: VecDeque<Step>,
    next_sentence: usize,
    next_utterance_id: u32,
}

impl Worker {
    /// Runs until the engine is dropped.
    fn run(mut self, commands: &Receiver<Command>) {
        loop {
            let received = match self.steps.front() {
                Some(step) => commands.recv_timeout(step.delay),
                None => commands.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match received {
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => self.run_next_step(),
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Enable => {
                if self.state.transition(EngineState::Loading) {
                    self.queue(self.pacing.load, Action::Loaded);
                }
            }
            Command::Disable => {
                self.steps.clear();
                self.state.transition(EngineState::Off);
            }
            Command::Speak => {
                if self.state.get().is_running() {
                    self.state.transition(EngineState::Speaking);
                    self.steps.push_front(Step {
                        delay: self.pacing.speak,
                        action: Action::Spoken,
                    });
                }
            }
        }
    }

    fn run_next_step(&mut self) {
        let Some(step) = self.steps.pop_front() else {
            return;
        };
        match step.action {
            Action::Loaded => {
                self.state.transition(EngineState::Listening);
            }
            Action::Partial(transcript) => {
                self.sink.emit(EngineEvent::TranscriptPartial(transcript));
            }
            Action::Final(transcript) => {
                self.sink.emit(EngineEvent::TranscriptFinal(transcript));
                self.state.transition(EngineState::Speaking);
            }
            Action::Spoken => {
                self.state.transition(EngineState::Listening);
            }
        }

        if self.steps.is_empty() && self.state.get().is_running() {
            self.queue_next_utterance();
        }
    }

    /// Queues the partial transcripts, the final transcript and the speech of the next sentence.
    fn queue_next_utterance(&mut self) {
        if self.sentences.is_empty() {
            return;
        }
        let sentence = self.sentences[self.next_sentence % self.sentences.len()];
        let utterance_id = self.next_utterance_id;
        self.next_sentence += 1;
        self.next_utterance_id = self.next_utterance_id.wrapping_add(1);

        let words: Vec<&str> = sentence.split_whitespace().collect();
        for word_count in 1..=words.len() {
            let delay = if word_count == 1 {
                self.pacing.gap
            } else {
                self.pacing.word
            };
            let transcript = Transcript {
                utterance_id,
                text: words[..word_count].join(" "),
            };
            self.queue(delay, Action::Partial(transcript));
        }

        let transcript = Transcript {
            utterance_id,
            text: words.join(" "),
        };
        self.queue(self.pacing.word, Action::Final(transcript));
        self.queue(self.pacing.speak, Action::Spoken);
    }

    fn queue(&mut self, delay: Duration, action: Action) {
        self.steps.push_back(Step { delay, action });
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const FAST: Pacing = Pacing {
        load: Duration::from_millis(1),
        gap: Duration::from_millis(1),
        word: Duration::from_millis(1),
        speak: Duration::from_millis(1),
    };

    fn engine_with_script(
        sentences: &'static [&'static str],
    ) -> (MockEngine, Receiver<EngineEvent>) {
        let (sender, events) = mpsc::channel();
        let engine = MockEngine::with_script(Arc::new(sender), || Ok(()), sentences, FAST);
        (engine, events)
    }

    fn next_event(events: &Receiver<EngineEvent>) -> EngineEvent {
        events
            .recv_timeout(Duration::from_secs(5))
            .expect("the engine should keep producing events")
    }

    fn state_changed(state: EngineState) -> EngineEvent {
        EngineEvent::StateChanged(state)
    }

    fn transcript(utterance_id: u32, text: &str) -> Transcript {
        Transcript {
            utterance_id,
            text: text.to_owned(),
        }
    }

    #[test]
    fn enabling_plays_the_script_until_disabled() {
        let (engine, events) = engine_with_script(&["hello world"]);
        assert!(matches!(
            engine.speak_text("hi"),
            Err(EngineError::NotRunning)
        ));

        engine.set_enabled(true).unwrap();

        let expected = [
            state_changed(EngineState::Loading),
            state_changed(EngineState::Listening),
            EngineEvent::TranscriptPartial(transcript(0, "hello")),
            EngineEvent::TranscriptPartial(transcript(0, "hello world")),
            EngineEvent::TranscriptFinal(transcript(0, "hello world")),
            state_changed(EngineState::Speaking),
            state_changed(EngineState::Listening),
            EngineEvent::TranscriptPartial(transcript(1, "hello")),
        ];
        for event in expected {
            assert_eq!(next_event(&events), event);
        }

        engine.set_enabled(false).unwrap();
        while next_event(&events) != state_changed(EngineState::Off) {}
        assert!(events.recv_timeout(Duration::from_millis(50)).is_err());
        assert_eq!(engine.state(), EngineState::Off);
    }

    #[test]
    fn typed_text_is_spoken_while_listening() {
        let (engine, events) = engine_with_script(&[]);
        engine.set_enabled(true).unwrap();
        assert_eq!(next_event(&events), state_changed(EngineState::Loading));
        assert_eq!(next_event(&events), state_changed(EngineState::Listening));

        engine.speak_text("hi").unwrap();

        assert_eq!(next_event(&events), state_changed(EngineState::Speaking));
        assert_eq!(next_event(&events), state_changed(EngineState::Listening));
    }

    #[test]
    fn enabling_is_refused_without_the_virtual_microphone() {
        let (sender, events) = mpsc::channel();
        let missing = || Err(AudioError::CableMissing);
        let engine = MockEngine::with_script(Arc::new(sender), missing, &["hello"], FAST);

        let result = engine.set_enabled(true);

        assert!(matches!(
            result,
            Err(EngineError::Audio(AudioError::CableMissing))
        ));
        assert!(events.recv_timeout(Duration::from_millis(50)).is_err());
        assert_eq!(engine.state(), EngineState::Off);
    }
}
