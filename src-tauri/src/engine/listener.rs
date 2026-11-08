use std::collections::VecDeque;
use std::mem;

use super::{INPUT_SAMPLE_RATE, Stt, Vad};

/// Audio kept from before speech is detected, because detection lags behind the speech.
const LEAD_IN_SAMPLES: usize = INPUT_SAMPLE_RATE as usize;

/// What the listener understood from the audio so far.
#[derive(Debug, PartialEq)]
pub enum Heard {
    /// The words of the current utterance, when they changed.
    Partial(String),
    /// The words of an utterance that has ended.
    Final(String),
}

/// Finds the speech in microphone audio and recognizes it.
pub struct Listener {
    vad: Box<dyn Vad>,
    stt: Box<dyn Stt>,
    lead_in: VecDeque<f32>,
    speaking: bool,
    partial_text: String,
}

impl Listener {
    pub fn new(vad: Box<dyn Vad>, stt: Box<dyn Stt>) -> Self {
        Self {
            vad,
            stt,
            lead_in: VecDeque::new(),
            speaking: false,
            partial_text: String::new(),
        }
    }

    /// Feeds the next `samples` of the microphone, which are mono at [`INPUT_SAMPLE_RATE`].
    pub fn accept(&mut self, samples: &[f32]) -> Option<Heard> {
        let speaking = self.vad.accept(samples);
        let was_speaking = mem::replace(&mut self.speaking, speaking);

        if speaking {
            if !was_speaking {
                let mut lead_in = mem::take(&mut self.lead_in);
                self.stt.accept(lead_in.make_contiguous());
            }
            return self.partial(samples);
        }

        let heard = was_speaking.then(|| {
            self.partial_text.clear();
            Heard::Final(tidy(&self.stt.finish()))
        });
        self.keep_lead_in(samples);
        heard
    }

    fn partial(&mut self, samples: &[f32]) -> Option<Heard> {
        let text = tidy(&self.stt.accept(samples));
        if text.is_empty() || text == self.partial_text {
            return None;
        }
        self.partial_text.clone_from(&text);
        Some(Heard::Partial(text))
    }

    fn keep_lead_in(&mut self, samples: &[f32]) {
        self.lead_in.extend(samples);
        let excess = self.lead_in.len().saturating_sub(LEAD_IN_SAMPLES);
        self.lead_in.drain(..excess);
    }
}

/// Trims the text and collapses the spaces between its words.
fn tidy(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::engine::fakes::{LoudnessVad, RecordingStt};

    const CHUNK_SAMPLES: usize = LEAD_IN_SAMPLES / 2;

    fn listener() -> (Listener, Arc<Mutex<Vec<f32>>>) {
        let stt = RecordingStt::default();
        let heard = Arc::clone(&stt.heard);
        (Listener::new(Box::new(LoudnessVad), Box::new(stt)), heard)
    }

    fn silence() -> Vec<f32> {
        vec![0.0; CHUNK_SAMPLES]
    }

    fn speech() -> Vec<f32> {
        vec![1.0; CHUNK_SAMPLES]
    }

    fn count(audio: &Mutex<Vec<f32>>, level: f32) -> usize {
        audio
            .lock()
            .unwrap()
            .iter()
            .filter(|&&s| s == level)
            .count()
    }

    #[test]
    fn an_utterance_is_reported_as_partial_text_then_final_text() {
        let (mut listener, _) = listener();

        let results: Vec<_> = [silence(), speech(), speech(), silence(), silence()]
            .iter()
            .map(|chunk| listener.accept(chunk))
            .collect();

        let partial = Heard::Partial("Hello".to_owned());
        let last = Heard::Final("Hello world".to_owned());
        assert_eq!(results, [None, Some(partial), None, Some(last), None]);
    }

    #[test]
    fn the_recognizer_hears_the_audio_before_the_speech_but_not_older_speech() {
        let cases = [
            (
                vec![silence(), silence(), silence(), speech()],
                LEAD_IN_SAMPLES,
            ),
            (vec![speech(), silence(), speech()], CHUNK_SAMPLES),
        ];

        for (chunks, expected_silence) in cases {
            let (mut listener, heard) = listener();
            for chunk in chunks {
                listener.accept(&chunk);
            }

            assert_eq!(count(&heard, 0.0), expected_silence);
            assert_eq!(count(&heard, 1.0), CHUNK_SAMPLES);
        }
    }

    #[test]
    fn recognized_text_is_trimmed_and_keeps_its_case() {
        let cases = [("", ""), ("  Hello,   World ", "Hello, World")];

        for (recognized, expected) in cases {
            assert_eq!(tidy(recognized), expected);
        }
    }
}
