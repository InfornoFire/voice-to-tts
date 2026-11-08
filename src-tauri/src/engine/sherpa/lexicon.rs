use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::engine::SpeechError;

/// The delimiter between a word and its phonemes in a lexicon line.
const DELIMITER: &str = " || ";

/// The words a pronunciation lexicon has an entry for.
pub struct Lexicon {
    words: HashSet<String>,
}

impl Lexicon {
    /// Reads the lexicon in `path`, with one `word || phonemes` entry per line.
    pub fn load(path: &Path, model: &'static str) -> Result<Self, SpeechError> {
        let text = fs::read_to_string(path).map_err(|_| SpeechError::Load(model))?;
        Ok(Self::parse(&text))
    }

    fn parse(text: &str) -> Self {
        let words = text
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_once(DELIMITER))
            .map(|(word, _)| word)
            .filter(|word| !word.contains(' '))
            .map(str::to_lowercase)
            .collect();
        Self { words }
    }

    /// Whether `word`, in lowercase, has an entry.
    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_single_words_are_listed() {
        let lexicon = Lexicon::parse(
            "# Lexicon\nhello || h e l o\nWorld || w o r l d\nhello world || h e l o w o r l d\n",
        );
        assert!(lexicon.contains("hello") && lexicon.contains("world"));
        assert!(!lexicon.contains("hello world") && !lexicon.contains("# lexicon"));
    }
}
