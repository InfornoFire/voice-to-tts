//! The text of the sentence that is being spoken, which grows by one phrase at a time.

/// The most characters kept from the earlier phrases of a sentence, so the text stays readable.
const MAX_CHARACTERS: usize = 150;

/// What a viewer reads while the voice speaks one sentence in several phrases.
#[derive(Default)]
pub struct Sentence {
    text: String,
}

impl Sentence {
    /// Adds a `phrase` that starts playing and returns the text to show.
    ///
    /// A phrase that `continues` the sentence is added after what is there, dropping the oldest
    /// words when it gets too long. Any other phrase replaces the text.
    pub fn add(&mut self, phrase: &str, continues: bool) -> &str {
        if continues {
            self.keep_at_most(MAX_CHARACTERS.saturating_sub(phrase.chars().count() + 1));
            if !self.text.is_empty() {
                self.text.push(' ');
            }
        } else {
            self.text.clear();
        }
        self.text.push_str(phrase);
        &self.text
    }

    /// Forgets the text, so the next phrase starts a new sentence.
    pub fn clear(&mut self) {
        self.text.clear();
    }

    /// Drops words from the start until at most `limit` characters are left.
    fn keep_at_most(&mut self, limit: usize) {
        while self.text.chars().count() > limit {
            let word_end = self
                .text
                .find(' ')
                .map_or(self.text.len(), |space| space + 1);
            self.text.drain(..word_end);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrases_that_continue_are_added_and_the_oldest_words_make_room() {
        let mut sentence = Sentence::default();

        assert_eq!(sentence.add("Hello there", false), "Hello there");
        assert_eq!(sentence.add("my friend", true), "Hello there my friend");

        for _ in 0..30 {
            sentence.add("alpha beta gamma", true);
        }
        let text = sentence.add("one two three", true);
        assert!(text.chars().count() <= MAX_CHARACTERS, "{text}");
        assert!(text.starts_with("alpha") || text.starts_with("beta") || text.starts_with("gamma"));
        assert!(text.ends_with("gamma one two three"));

        assert_eq!(sentence.add("A new sentence", false), "A new sentence");
    }

    #[test]
    fn a_phrase_is_kept_whole_and_a_cleared_sentence_starts_again() {
        let mut sentence = Sentence::default();
        let long = "word ".repeat(40).trim_end().to_owned();

        sentence.add("Hello there", false);
        assert_eq!(sentence.add(&long, true), long);

        sentence.clear();
        assert_eq!(sentence.add("my friend", true), "my friend");
    }
}
