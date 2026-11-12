//! Shows the text the voice speaks in an OBS browser source.

mod hub;
mod sentence;
mod server;

use std::io;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use hub::{Hub, Message};
use sentence::Sentence;
use server::Server;

use crate::engine::{EngineEvent, EventSink};

/// Failure to serve the subtitles.
#[derive(Debug, thiserror::Error)]
pub enum SubtitlesError {
    #[error("the subtitles server cannot listen on that port")]
    Unavailable(#[source] io::Error),
}

/// Passes the text that is being spoken to the subtitles page, while the server runs.
#[derive(Default)]
pub struct Subtitles {
    hub: Arc<Hub>,
    sentence: Mutex<Sentence>,
    server: Mutex<Option<Server>>,
}

impl Subtitles {
    /// Runs the server on `port` when `enabled` and stops it otherwise.
    ///
    /// When the port cannot be used, a server that is already running stays up.
    pub fn apply(&self, enabled: bool, port: u16) -> Result<(), SubtitlesError> {
        let mut server = self.lock_server();
        if !enabled {
            *server = None;
            return Ok(());
        }
        if server
            .as_ref()
            .is_some_and(|running| running.port() == port)
        {
            return Ok(());
        }

        let started =
            Server::start(port, Arc::clone(&self.hub)).map_err(SubtitlesError::Unavailable)?;
        *server = Some(started);
        Ok(())
    }

    fn lock_server(&self) -> MutexGuard<'_, Option<Server>> {
        self.server.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_sentence(&self) -> MutexGuard<'_, Sentence> {
        self.sentence.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl EventSink for Subtitles {
    fn emit(&self, event: EngineEvent) {
        match event {
            EngineEvent::SpeechStarted { text, continues } => {
                let shown = self.lock_sentence().add(&text, continues).to_owned();
                self.hub.publish(Message::Speaking(shown));
            }
            EngineEvent::SpeechEnded => self.hub.publish(Message::Ended),
            EngineEvent::SpeechCancelled => {
                self.lock_sentence().clear();
                self.hub.publish(Message::Cancelled);
            }
            EngineEvent::StateChanged(_)
            | EngineEvent::TranscriptPartial(_)
            | EngineEvent::TranscriptFinal(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, TcpListener, TcpStream};

    use super::*;
    use crate::engine::EngineState;

    fn free_port() -> u16 {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.local_addr().unwrap().port()
    }

    fn is_listening(port: u16) -> bool {
        TcpStream::connect((Ipv4Addr::LOCALHOST, port)).is_ok()
    }

    #[test]
    fn the_server_follows_the_setting_and_a_port_in_use_keeps_the_running_one() {
        let subtitles = Subtitles::default();
        let port = free_port();
        let taken = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let taken_port = taken.local_addr().unwrap().port();

        subtitles.apply(true, port).unwrap();
        assert!(is_listening(port));

        let result = subtitles.apply(true, taken_port);
        assert!(matches!(result, Err(SubtitlesError::Unavailable(_))));
        assert!(is_listening(port));

        subtitles.apply(false, port).unwrap();
        assert!(!is_listening(port));
    }

    #[test]
    fn speech_events_are_published_and_other_events_are_ignored() {
        let subtitles = Subtitles::default();
        let messages = subtitles.hub.subscribe();

        subtitles.emit(EngineEvent::StateChanged(EngineState::Listening));
        for (text, continues) in [("Hello", false), ("world", true)] {
            subtitles.emit(EngineEvent::SpeechStarted {
                text: text.to_owned(),
                continues,
            });
            subtitles.emit(EngineEvent::SpeechEnded);
        }
        subtitles.emit(EngineEvent::SpeechCancelled);
        subtitles.emit(EngineEvent::SpeechStarted {
            text: "again".to_owned(),
            continues: true,
        });

        let speaking = |text: &str| Ok(Message::Speaking(text.to_owned()));
        assert_eq!(messages.try_recv(), speaking("Hello"));
        assert_eq!(messages.try_recv(), Ok(Message::Ended));
        assert_eq!(messages.try_recv(), speaking("Hello world"));
        assert_eq!(messages.try_recv(), Ok(Message::Ended));
        assert_eq!(messages.try_recv(), Ok(Message::Cancelled));
        assert_eq!(messages.try_recv(), speaking("again"));
        assert!(messages.try_recv().is_err());
    }
}
