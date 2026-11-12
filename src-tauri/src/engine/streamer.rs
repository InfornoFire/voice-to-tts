//! Decides which recognized words the voice may speak while a sentence is still being said.

use std::time::{Duration, Instant};

/// Words for the voice to speak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phrase {
    pub text: String,
    /// Whether earlier words of the same utterance were already spoken.
    pub continues: bool,
}

struct Word {
    text: String,
    /// When the next word first appeared.
    finished_at: Option<Instant>,
}

impl Word {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            finished_at: None,
        }
    }
}

/// Follows the partial text of one utterance and hands out its words once they are safe to speak.
///
/// A word is safe once the next word has been there for a chosen delay, because the last word of a
/// partial text is often unfinished. Spoken words cannot be taken back, so a later revision of one
/// is ignored.
#[derive(Default)]
pub struct Streamer {
    words: Vec<Word>,
    spoken: usize,
}

impl Streamer {
    /// Takes the latest partial `text` of the utterance, heard at `now`.
    pub fn partial(&mut self, text: &str, now: Instant) {
        let heard: Vec<&str> = text.split_whitespace().collect();
        let unchanged = self
            .words
            .iter()
            .zip(&heard)
            .take_while(|(known, word)| known.text == **word)
            .count();
        self.words.truncate(unchanged);
        self.words
            .extend(heard[unchanged..].iter().map(|word| Word::new(word)));

        let finished = self.words.len().saturating_sub(1);
        for word in &mut self.words[..finished] {
            word.finished_at.get_or_insert(now);
        }
    }

    /// Returns the words that were not spoken yet and have been finished for at least `delay`.
    pub fn poll(&mut self, now: Instant, delay: Duration) -> Option<Phrase> {
        let waiting = self.words.get(self.spoken..)?;
        let ready = waiting
            .iter()
            .take_while(|word| {
                word.finished_at
                    .is_some_and(|finished_at| now.duration_since(finished_at) >= delay)
            })
            .count();
        if ready == 0 {
            return None;
        }
        let phrase = Phrase {
            text: join(waiting[..ready].iter().map(|word| word.text.as_str())),
            continues: self.spoken > 0,
        };
        self.spoken += ready;
        Some(phrase)
    }

    /// Ends the utterance, returning the words of its final `text` that were not spoken yet.
    ///
    /// Words are matched by position, so a word that was revised after it was spoken is not spoken
    /// again.
    pub fn finish(&mut self, text: &str) -> Option<Phrase> {
        let words: Vec<_> = text.split_whitespace().collect();
        let rest = words.get(self.spoken..).unwrap_or_default();
        let phrase = (!rest.is_empty()).then(|| Phrase {
            text: join(rest.iter().copied()),
            continues: self.spoken > 0,
        });
        self.reset();
        phrase
    }

    /// Forgets the utterance, without speaking the rest of it.
    pub fn reset(&mut self) {
        self.words.clear();
        self.spoken = 0;
    }
}

fn join<'a>(words: impl Iterator<Item = &'a str>) -> String {
    words.collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DELAY: Duration = Duration::from_millis(500);

    fn phrase(text: &str, continues: bool) -> Option<Phrase> {
        Some(Phrase {
            text: text.to_owned(),
            continues,
        })
    }

    #[test]
    fn a_word_is_spoken_once_it_is_finished_and_has_waited_for_the_delay() {
        let start = Instant::now();
        let at = |milliseconds| start + Duration::from_millis(milliseconds);
        let mut streamer = Streamer::default();

        streamer.partial("Hello", at(0));
        assert_eq!(streamer.poll(at(5000), DELAY), None);

        streamer.partial("Hello there", at(600));
        assert_eq!(streamer.poll(at(1099), DELAY), None);
        assert_eq!(streamer.poll(at(1100), DELAY), phrase("Hello", false));
        assert_eq!(streamer.poll(at(5000), DELAY), None);

        streamer.partial("Hello there my friend", at(1200));
        assert_eq!(streamer.poll(at(1699), DELAY), None);
        assert_eq!(streamer.poll(at(1700), DELAY), phrase("there my", true));
    }

    #[test]
    fn a_revised_word_and_the_words_after_it_wait_again() {
        let start = Instant::now();
        let at = |milliseconds| start + Duration::from_millis(milliseconds);
        let mut streamer = Streamer::default();
        streamer.partial("Hello there you", at(0));

        streamer.partial("Hello where you are", at(100));

        assert_eq!(streamer.poll(at(500), DELAY), phrase("Hello", false));
        assert_eq!(streamer.poll(at(599), DELAY), None);
        assert_eq!(streamer.poll(at(600), DELAY), phrase("where you", true));
    }

    #[test]
    fn the_final_text_speaks_only_what_was_not_spoken_yet() {
        let start = Instant::now();
        let mut streamer = Streamer::default();
        streamer.partial("Hello there my", start);
        assert_eq!(
            streamer.poll(start + DELAY, DELAY),
            phrase("Hello there", false)
        );

        assert_eq!(
            streamer.finish("Hello there my friend"),
            phrase("my friend", true)
        );
        assert_eq!(streamer.finish("Another one"), phrase("Another one", false));
        assert_eq!(streamer.finish(""), None);

        streamer.partial("Hello there", start);
        streamer.poll(start + DELAY, DELAY);
        streamer.reset();
        assert_eq!(streamer.finish("Hello there"), phrase("Hello there", false));
    }
}
