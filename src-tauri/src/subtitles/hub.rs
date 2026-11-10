//! Tells every connected subtitle page what the voice is saying.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// What a subtitle page is told.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// The voice started playing this text.
    Speaking(String),
    /// The voice stopped playing.
    Ended,
}

#[derive(Default)]
struct Inner {
    /// The text being played now.
    current: Option<String>,
    subscribers: Vec<Sender<Message>>,
}

/// Remembers what is being played and passes every change to the subscribers.
#[derive(Default)]
pub struct Hub {
    inner: Mutex<Inner>,
}

impl Hub {
    pub fn publish(&self, message: Message) {
        let mut inner = self.lock();
        inner.current = match &message {
            Message::Speaking(text) => Some(text.clone()),
            Message::Ended => None,
        };
        inner
            .subscribers
            .retain(|subscriber| subscriber.send(message.clone()).is_ok());
    }

    /// Returns a receiver of the changes from now on, which starts with the text being played.
    pub fn subscribe(&self) -> Receiver<Message> {
        let mut inner = self.lock();
        let (sender, receiver) = mpsc::channel();
        if let Some(text) = &inner.current {
            let _ = sender.send(Message::Speaking(text.clone()));
        }
        inner.subscribers.push(sender);
        receiver
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
