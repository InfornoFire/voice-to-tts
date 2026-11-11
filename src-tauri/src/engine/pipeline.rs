//! The default engine, which runs the speech models on this machine.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use super::session::{Parts, Session, Settings};
use super::{
    Engine, EngineError, EngineState, EventSink, Listener, OutputTarget, Resources, StateMachine,
};
use crate::audio::fan_out::FanOutOutput;
use crate::config::Config;

/// How long the microphone and typed text are ignored after speech is cancelled.
const CANCEL_PAUSE: Duration = Duration::from_secs(3);

enum Command {
    Enable,
    Disable,
    Speak(String),
    Cancel,
    /// A running session could not continue.
    Fault {
        session: u32,
        error: EngineError,
    },
    Shutdown,
}

/// An [`Engine`] that listens to the microphone and speaks through the virtual microphone when
/// there is one, and through the default audio output otherwise or when asked to.
///
/// All state changes beyond speaking happen on one worker thread, which sleeps while the engine is
/// off. The microphone and the models are only held while the engine is on.
pub struct PipelineEngine {
    state: Arc<StateMachine>,
    commands: Sender<Command>,
}

impl PipelineEngine {
    pub fn new(
        sink: Arc<dyn EventSink>,
        resources: impl Resources + 'static,
        settings: impl Fn() -> Config + Send + Sync + 'static,
    ) -> Self {
        Self::with_cancel_pause(sink, resources, settings, CANCEL_PAUSE)
    }

    /// Like [`PipelineEngine::new`], with the pause after a cancel chosen, so tests stay short.
    fn with_cancel_pause(
        sink: Arc<dyn EventSink>,
        resources: impl Resources + 'static,
        settings: impl Fn() -> Config + Send + Sync + 'static,
        cancel_pause: Duration,
    ) -> Self {
        let state = Arc::new(StateMachine::new(Arc::clone(&sink)));
        let (commands, receiver) = mpsc::channel();
        let worker = Worker {
            state: Arc::clone(&state),
            sink,
            resources: Arc::new(resources),
            settings: Arc::new(settings),
            utterance_ids: Arc::new(AtomicU32::new(0)),
            cancel_pause,
            commands: commands.clone(),
            running: None,
            sessions_started: 0,
        };
        thread::spawn(move || worker.run(&receiver));
        Self { state, commands }
    }

    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

impl Engine for PipelineEngine {
    fn set_enabled(&self, enabled: bool) -> Result<(), EngineError> {
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

    fn cancel_speech(&self) {
        self.send(Command::Cancel);
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

/// The outputs to play speech through.
///
/// Speech goes to the virtual microphone when there is one, unless that is what is listened to,
/// because it would be heard again. It goes to the default audio output when the user wants to hear
/// it too, and whenever nothing else would receive it.
fn output_targets(
    cable_installed: bool,
    microphone_is_virtual: bool,
    hear_voice: bool,
) -> Vec<OutputTarget> {
    match (cable_installed && !microphone_is_virtual, hear_voice) {
        (true, false) => vec![OutputTarget::VirtualMicrophone],
        (true, true) => vec![OutputTarget::VirtualMicrophone, OutputTarget::DefaultOutput],
        (false, _) => vec![OutputTarget::DefaultOutput],
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
    cancel_pause: Duration,
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
                Command::Cancel => {
                    if let Some(running) = &self.running {
                        running.session.cancel();
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
        let targets = output_targets(
            self.resources.is_cable_installed(),
            self.resources
                .is_virtual_microphone(config.input_device.as_deref()),
            config.hear_voice,
        );
        let outputs = targets
            .into_iter()
            .map(|target| self.resources.open_output(target))
            .collect::<Result<Vec<_>, _>>()?;
        let models = self.resources.load_models(&config)?;
        let input = self.resources.open_input(config.input_device.as_deref())?;

        let commands = self.commands.clone();
        Ok(Parts {
            input,
            output: Box::new(FanOutOutput::new(outputs)),
            listener: Listener::new(models.vad, models.stt),
            tts: models.tts,
            state: Arc::clone(&self.state),
            sink: Arc::clone(&self.sink),
            settings: Arc::clone(&self.settings),
            utterance_ids: Arc::clone(&self.utterance_ids),
            cancel_pause: self.cancel_pause,
            on_fault: Box::new(move |error| {
                let _ = commands.send(Command::Fault { session: id, error });
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::engine::EngineEvent;
    use crate::engine::fakes::{FakeResources, next_event, state_changed, transcript};
    use crate::error::ErrorCode;

    fn speech() -> Vec<f32> {
        vec![1.0; 10]
    }

    fn silence() -> Vec<f32> {
        vec![0.0; 10]
    }

    fn engine_with(resources: &FakeResources) -> (PipelineEngine, Receiver<EngineEvent>) {
        engine_with_config(resources, Config::default())
    }

    fn engine_with_config(
        resources: &FakeResources,
        config: Config,
    ) -> (PipelineEngine, Receiver<EngineEvent>) {
        let (sender, events) = mpsc::channel();
        let engine =
            PipelineEngine::new(Arc::new(sender), resources.clone(), move || config.clone());
        (engine, events)
    }

    fn expect_events(
        events: &Receiver<EngineEvent>,
        expected: impl IntoIterator<Item = EngineEvent>,
    ) {
        for event in expected {
            assert_eq!(next_event(events), event);
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
                EngineEvent::TranscriptPartial(transcript(0, "Hello")),
                EngineEvent::TranscriptFinal(transcript(0, "Hello world")),
                state_changed(EngineState::Speaking),
                EngineEvent::SpeechStarted("Hello world".to_owned()),
                EngineEvent::SpeechEnded,
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
    fn speech_goes_to_the_virtual_microphone_only_when_it_exists_and_is_not_listened_to() {
        use OutputTarget::{DefaultOutput, VirtualMicrophone};
        let cases = [
            (true, false, false, vec![VirtualMicrophone]),
            (true, false, true, vec![VirtualMicrophone, DefaultOutput]),
            (true, true, true, vec![DefaultOutput]),
            (false, false, false, vec![DefaultOutput]),
        ];

        for (cable_installed, microphone_is_virtual, hear_voice, expected) in cases {
            let resources = FakeResources {
                cable_installed,
                microphone_is_virtual,
                ..FakeResources::default()
            };
            let config = Config {
                hear_voice,
                ..Config::default()
            };
            let (engine, events) = engine_with_config(&resources, config);

            engine.set_enabled(true).unwrap();

            expect_events(
                &events,
                [
                    state_changed(EngineState::Loading),
                    state_changed(EngineState::Listening),
                ],
            );
            assert_eq!(*resources.outputs.lock().unwrap(), expected);
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
                EngineEvent::SpeechStarted("hi".to_owned()),
                EngineEvent::SpeechEnded,
                state_changed(EngineState::Listening),
            ],
        );
        assert_eq!(*resources.played.lock().unwrap(), [vec![0.5; 2]]);
    }

    #[test]
    fn cancelling_stops_the_clip_and_ignores_input_for_a_moment() {
        let resources = FakeResources::default();
        resources.output_idle.store(false, Ordering::Relaxed);
        resources
            .heard
            .lock()
            .unwrap()
            .extend([speech(), speech(), silence()]);
        let (sender, events) = mpsc::channel();
        let engine = PipelineEngine::with_cancel_pause(
            Arc::new(sender),
            resources.clone(),
            Config::default,
            Duration::from_millis(300),
        );
        engine.set_enabled(true).unwrap();
        expect_events(
            &events,
            [
                state_changed(EngineState::Loading),
                state_changed(EngineState::Listening),
                EngineEvent::TranscriptPartial(transcript(0, "Hello")),
                EngineEvent::TranscriptFinal(transcript(0, "Hello world")),
                state_changed(EngineState::Speaking),
                EngineEvent::SpeechStarted("Hello world".to_owned()),
            ],
        );

        engine.cancel_speech();

        // The two threads report in either order.
        let reported = [next_event(&events), next_event(&events)];
        assert!(reported.contains(&EngineEvent::SpeechCancelled));
        assert!(reported.contains(&state_changed(EngineState::Paused)));
        assert!(matches!(
            engine.speak_text("hi"),
            Err(EngineError::NotRunning)
        ));
        expect_events(&events, [state_changed(EngineState::Listening)]);
        assert_eq!(resources.stops.load(Ordering::Relaxed), 1);
        assert_eq!(resources.played.lock().unwrap().len(), 1);
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
}
