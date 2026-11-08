//! The engine that turns microphone speech into speech on the virtual microphone.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use super::session::{Parts, Session, Settings};
use super::{
    Engine, EngineError, EngineState, EventSink, Listener, OutputTarget, Resources, StateMachine,
};
use crate::config::Config;

enum Command {
    Enable,
    Disable,
    Speak(String),
    /// A running session could not continue.
    Fault {
        session: u32,
        error: EngineError,
    },
    Shutdown,
}

/// An [`Engine`] that listens to the microphone and speaks through the virtual microphone, or
/// through the default audio output when the virtual microphone is what it listens to.
///
/// All state changes beyond speaking happen on one worker thread, which sleeps while the engine is
/// off. The microphone and the models are only held while the engine is on.
pub struct PipelineEngine {
    state: Arc<StateMachine>,
    resources: Arc<dyn Resources>,
    commands: Sender<Command>,
}

impl PipelineEngine {
    pub fn new(
        sink: Arc<dyn EventSink>,
        resources: impl Resources + 'static,
        settings: impl Fn() -> Config + Send + Sync + 'static,
    ) -> Self {
        let state = Arc::new(StateMachine::new(Arc::clone(&sink)));
        let resources: Arc<dyn Resources> = Arc::new(resources);
        let (commands, receiver) = mpsc::channel();
        let worker = Worker {
            state: Arc::clone(&state),
            sink,
            resources: Arc::clone(&resources),
            settings: Arc::new(settings),
            utterance_ids: Arc::new(AtomicU32::new(0)),
            commands: commands.clone(),
            running: None,
            sessions_started: 0,
        };
        thread::spawn(move || worker.run(&receiver));
        Self {
            state,
            resources,
            commands,
        }
    }

    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

impl Engine for PipelineEngine {
    fn set_enabled(&self, enabled: bool) -> Result<(), EngineError> {
        if enabled {
            self.resources.check_cable()?;
        }
        self.send(if enabled {
            Command::Enable
        } else {
            Command::Disable
        });
        Ok(())
    }

    fn speak_text(&self, text: &str) -> Result<(), EngineError> {
        if !self.state.get().is_running() {
            return Err(EngineError::NotRunning);
        }
        self.send(Command::Speak(text.to_owned()));
        Ok(())
    }

    fn state(&self) -> EngineState {
        self.state.get()
    }
}

impl Drop for PipelineEngine {
    fn drop(&mut self) {
        self.send(Command::Shutdown);
    }
}

struct Running {
    id: u32,
    session: Session,
}

struct Worker {
    state: Arc<StateMachine>,
    sink: Arc<dyn EventSink>,
    resources: Arc<dyn Resources>,
    settings: Settings,
    utterance_ids: Arc<AtomicU32>,
    commands: Sender<Command>,
    running: Option<Running>,
    sessions_started: u32,
}

impl Worker {
    fn run(mut self, commands: &Receiver<Command>) {
        for command in commands {
            match command {
                Command::Enable => self.enable(),
                Command::Disable => self.disable(),
                Command::Speak(text) => {
                    if let Some(running) = &self.running {
                        running.session.say(text);
                    }
                }
                Command::Fault { session, error } => {
                    if self
                        .running
                        .as_ref()
                        .is_some_and(|running| running.id == session)
                    {
                        self.fail(error);
                    }
                }
                Command::Shutdown => return,
            }
        }
    }

    fn enable(&mut self) {
        if !self.state.transition(EngineState::Loading) {
            return;
        }

        let id = self.sessions_started;
        self.sessions_started = self.sessions_started.wrapping_add(1);
        match self.load(id) {
            Ok(parts) => {
                self.state.transition(EngineState::Listening);
                self.running = Some(Running {
                    id,
                    session: Session::start(parts),
                });
            }
            Err(error) => self.fail(error),
        }
    }

    fn disable(&mut self) {
        self.running = None;
        self.state.transition(EngineState::Off);
    }

    fn fail(&mut self, error: EngineError) {
        log::error!("the engine stopped: {error:?}");
        self.running = None;
        self.state.transition(EngineState::Error(error.into()));
    }

    /// Opens the devices and loads the models for the session `id`.
    ///
    /// The microphone is opened last, so it does not collect audio while the models load.
    fn load(&self, id: u32) -> Result<Parts, EngineError> {
        let config = (self.settings)();
        // Speech played into the virtual microphone while listening to it would be heard again.
        let target = if self
            .resources
            .is_virtual_microphone(config.input_device.as_deref())
        {
            OutputTarget::DefaultOutput
        } else {
            OutputTarget::VirtualMicrophone
        };
        let output = self.resources.open_output(target)?;
        let models = self.resources.load_models(&config)?;
        let input = self.resources.open_input(config.input_device.as_deref())?;

        let commands = self.commands.clone();
        Ok(Parts {
            input,
            output,
            listener: Listener::new(models.vad, models.stt),
            tts: models.tts,
            state: Arc::clone(&self.state),
            sink: Arc::clone(&self.sink),
            settings: Arc::clone(&self.settings),
            utterance_ids: Arc::clone(&self.utterance_ids),
            on_fault: Box::new(move |error| {
                let _ = commands.send(Command::Fault { session: id, error });
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use super::*;
    use crate::audio::devices::AudioError;
    use crate::audio::resample::resample_clip;
    use crate::download::HttpFetch;
    use crate::engine::fakes::FakeResources;
    use crate::engine::{EngineEvent, Heard, INPUT_SAMPLE_RATE, LocalResources, Transcript};
    use crate::error::ErrorCode;
    use crate::models::{Manifest, ModelManager, ModelsState};

    fn speech() -> Vec<f32> {
        vec![1.0; 10]
    }

    fn silence() -> Vec<f32> {
        vec![0.0; 10]
    }

    fn engine_with(resources: &FakeResources) -> (PipelineEngine, Receiver<EngineEvent>) {
        let (sender, events) = mpsc::channel();
        let engine = PipelineEngine::new(Arc::new(sender), resources.clone(), Config::default);
        (engine, events)
    }

    fn next_event(events: &Receiver<EngineEvent>) -> EngineEvent {
        events
            .recv_timeout(Duration::from_secs(5))
            .expect("the engine should keep producing events")
    }

    fn expect_events(
        events: &Receiver<EngineEvent>,
        expected: impl IntoIterator<Item = EngineEvent>,
    ) {
        for event in expected {
            assert_eq!(next_event(events), event);
        }
    }

    fn state_changed(state: EngineState) -> EngineEvent {
        EngineEvent::StateChanged(state)
    }

    fn transcript(text: &str) -> Transcript {
        Transcript {
            utterance_id: 0,
            text: text.to_owned(),
        }
    }

    fn error_code(event: EngineEvent) -> ErrorCode {
        match event {
            EngineEvent::StateChanged(EngineState::Error(error)) => error.code,
            other => panic!("expected an error state, got {other:?}"),
        }
    }

    #[test]
    fn heard_speech_is_transcribed_then_spoken_until_disabled() {
        let resources = FakeResources::default();
        resources
            .heard
            .lock()
            .unwrap()
            .extend([speech(), speech(), silence()]);
        let (engine, events) = engine_with(&resources);

        engine.set_enabled(true).unwrap();

        expect_events(
            &events,
            [
                state_changed(EngineState::Loading),
                state_changed(EngineState::Listening),
                EngineEvent::TranscriptPartial(transcript("Hello")),
                EngineEvent::TranscriptFinal(transcript("Hello world")),
                state_changed(EngineState::Speaking),
                state_changed(EngineState::Listening),
            ],
        );
        assert_eq!(
            *resources.played.lock().unwrap(),
            [vec![0.5; "Hello world".len()]]
        );

        engine.set_enabled(false).unwrap();
        expect_events(&events, [state_changed(EngineState::Off)]);
        assert!(!resources.microphone_open.load(Ordering::Relaxed));
    }

    #[test]
    fn speech_is_never_played_into_the_virtual_microphone_that_is_listened_to() {
        let cases = [
            (false, OutputTarget::VirtualMicrophone),
            (true, OutputTarget::DefaultOutput),
        ];

        for (microphone_is_virtual, expected) in cases {
            let resources = FakeResources {
                microphone_is_virtual,
                ..FakeResources::default()
            };
            let (engine, events) = engine_with(&resources);

            engine.set_enabled(true).unwrap();

            expect_events(
                &events,
                [
                    state_changed(EngineState::Loading),
                    state_changed(EngineState::Listening),
                ],
            );
            assert_eq!(*resources.outputs.lock().unwrap(), [expected]);
        }
    }

    #[test]
    fn typed_text_is_spoken_only_while_running() {
        let resources = FakeResources::default();
        let (engine, events) = engine_with(&resources);
        assert!(matches!(
            engine.speak_text("hi"),
            Err(EngineError::NotRunning)
        ));
        engine.set_enabled(true).unwrap();
        expect_events(
            &events,
            [
                state_changed(EngineState::Loading),
                state_changed(EngineState::Listening),
            ],
        );

        engine.speak_text("hi").unwrap();

        expect_events(
            &events,
            [
                state_changed(EngineState::Speaking),
                state_changed(EngineState::Listening),
            ],
        );
        assert_eq!(*resources.played.lock().unwrap(), [vec![0.5; 2]]);
    }

    #[test]
    fn missing_models_or_microphone_end_loading_with_an_error() {
        let cases = [
            (
                FakeResources {
                    models_installed: false,
                    ..FakeResources::default()
                },
                ErrorCode::ModelsUnavailable,
            ),
            (
                FakeResources {
                    microphone_installed: false,
                    ..FakeResources::default()
                },
                ErrorCode::MicUnavailable,
            ),
        ];

        for (resources, expected) in cases {
            let (engine, events) = engine_with(&resources);

            engine.set_enabled(true).unwrap();

            assert_eq!(next_event(&events), state_changed(EngineState::Loading));
            assert_eq!(error_code(next_event(&events)), expected);
            assert!(!resources.microphone_open.load(Ordering::Relaxed));
        }
    }

    #[test]
    #[ignore = "downloads the models to the models directory"]
    fn downloaded_models_recognize_their_own_speech() {
        let local_data = std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA should be set");
        let models_dir = std::path::PathBuf::from(local_data)
            .join("com.inforno.voice-to-tts")
            .join("models");
        let manager = ModelManager::new(
            models_dir,
            Manifest::bundled(),
            Box::new(HttpFetch::new()),
            Box::new(|_| {}),
        );
        assert_eq!(manager.download(), ModelsState::Ready);
        let mut models = LocalResources::new(manager.dir().to_owned(), manager.voices().to_vec())
            .load_models(&Config::default())
            .unwrap();

        let sentence = "Hello there, this is a test of the voice proxy.";
        let first_voice = manager.voices().first().unwrap();
        let last_voice = manager.voices().last().unwrap();
        let speech = models
            .tts
            .synthesize(sentence, &first_voice.id, 1.0)
            .unwrap();
        let other_speech = models
            .tts
            .synthesize(sentence, &last_voice.id, 1.0)
            .unwrap();
        assert_ne!(speech, other_speech);
        let speech = resample_clip(&speech, models.tts.sample_rate(), INPUT_SAMPLE_RATE).unwrap();
        let mut audio = vec![0.0; INPUT_SAMPLE_RATE as usize];
        audio.extend(speech);
        audio.extend(vec![0.0; 2 * INPUT_SAMPLE_RATE as usize]);

        let mut listener = Listener::new(models.vad, models.stt);
        let heard: Vec<_> = audio
            .chunks(INPUT_SAMPLE_RATE as usize / 50)
            .filter_map(|chunk| listener.accept(chunk))
            .collect();

        let Some(Heard::Final(text)) = heard.last() else {
            panic!("no utterance ended: {heard:?}");
        };
        let text = text.to_lowercase();
        assert!(text.contains("test") && text.contains("voice"), "{text}");
    }

    #[test]
    fn losing_the_microphone_stops_the_engine_with_an_error() {
        let resources = FakeResources::default();
        let (engine, events) = engine_with(&resources);
        engine.set_enabled(true).unwrap();
        expect_events(
            &events,
            [
                state_changed(EngineState::Loading),
                state_changed(EngineState::Listening),
            ],
        );

        resources.microphone_lost.store(true, Ordering::Relaxed);

        assert_eq!(error_code(next_event(&events)), ErrorCode::MicUnavailable);
        assert!(!resources.microphone_open.load(Ordering::Relaxed));
        assert!(matches!(
            engine.speak_text("hi"),
            Err(EngineError::NotRunning)
        ));
    }

    #[test]
    fn enabling_is_refused_without_the_virtual_microphone() {
        let resources = FakeResources {
            cable_installed: false,
            ..FakeResources::default()
        };
        let (engine, events) = engine_with(&resources);

        let result = engine.set_enabled(true);

        assert!(matches!(
            result,
            Err(EngineError::Audio(AudioError::CableMissing))
        ));
        assert!(events.recv_timeout(Duration::from_millis(50)).is_err());
        assert_eq!(engine.state(), EngineState::Off);
    }
}
