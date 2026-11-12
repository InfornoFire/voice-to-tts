//! The thread that recognizes what the microphone hears.

use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use super::{POLL_INTERVAL, Request, Shared};
use crate::audio::AudioInput;
use crate::audio::devices::AudioError;
use crate::engine::{EngineEvent, EngineState, Heard, Listener, Phrase, Streamer, Transcript};

/// Recognizes what the microphone hears and queues the words for the voice to speak.
///
/// While streaming, words are queued as they are finished and the rest when the utterance ends.
/// Otherwise the whole utterance is queued when it ends.
///
/// This thread alone moves the engine into and out of [`EngineState::Paused`]. While paused the
/// microphone is still read, so no audio piles up, but it is thrown away.
pub(super) fn listen(
    shared: &Shared,
    mut input: Box<dyn AudioInput>,
    mut listener: Listener,
    requests: &Sender<Request>,
) {
    let mut samples = Vec::new();
    let mut utterance_id = None;
    let mut streamer = Streamer::default();
    let mut stream_delay = current_stream_delay(shared);
    let mut was_paused = false;
    let queue = |phrase: Phrase| {
        let _ = requests.send(shared.request(phrase));
    };

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
                streamer.reset();
                utterance_id = None;
                shared.state.transition(EngineState::Paused);
            } else {
                shared
                    .state
                    .transition_from(&EngineState::Paused, EngineState::Listening);
            }
        }
        if paused {
            continue;
        }

        let heard = if samples.is_empty() {
            None
        } else {
            listener.accept(&samples)
        };
        match heard {
            Some(Heard::Partial(text)) => {
                streamer.partial(&text, Instant::now());
                stream_delay = current_stream_delay(shared);
                let utterance_id = *utterance_id.get_or_insert_with(|| shared.next_utterance_id());
                shared.sink.emit(EngineEvent::TranscriptPartial(Transcript {
                    utterance_id,
                    text,
                }));
            }
            Some(Heard::Final(text)) => {
                let started_id = utterance_id.take();
                let rest = streamer.finish(&text);
                if text.is_empty() {
                    continue;
                }
                let utterance_id = started_id.unwrap_or_else(|| shared.next_utterance_id());
                shared.sink.emit(EngineEvent::TranscriptFinal(Transcript {
                    utterance_id,
                    text,
                }));
                if let Some(phrase) = rest {
                    queue(phrase);
                }
            }
            None => {}
        }

        if let Some(phrase) = stream_delay.and_then(|delay| streamer.poll(Instant::now(), delay)) {
            queue(phrase);
        }
    }
}

/// How long a finished word waits before it is spoken, or `None` while whole utterances are spoken.
fn current_stream_delay(shared: &Shared) -> Option<Duration> {
    let config = (shared.settings)();
    config
        .stream_speech
        .then(|| Duration::try_from_secs_f32(config.stream_delay_seconds).unwrap_or_default())
}
