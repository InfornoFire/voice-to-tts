//! The words the voice can pronounce.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::engine::SpeechError;

const WEAK_FORMS_FILE: &str = "weak-forms.txt";

/// Pronunciations that replace the lexicon's, in its format.
///
/// The lexicon only lists the strong forms of "a", "the" and "to", which sound wrong in running
/// speech.
const WEAK_FORMS: &str = "a ɐ\nthe ð ə\nto t ə\n";

/// The words a pronunciation lexicon has an entry for.
pub struct Lexicon {
    words: HashSet<String>,
}

impl Lexicon {
    /// Reads the lexicon in `path`, with one `word phonemes` entry per line.
    pub fn load(path: &Path, model: &'static str) -> Result<Self, SpeechError> {
        let text = fs::read_to_string(path).map_err(|_| SpeechError::Load(model))?;
        Ok(Self::parse(&text))
    }

    fn parse(text: &str) -> Self {
        let words = text
            .lines()
            .filter_map(|line| line.split_once(' '))
            .map(|(word, _)| word.to_owned())
            .collect();
        Self { words }
    }

    /// Whether `word`, in lowercase, has an entry.
    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(word)
    }
}

/// Writes the replacement pronunciations into `directory` and returns the path of the file.
pub fn write_weak_forms(directory: &Path, model: &'static str) -> Result<PathBuf, SpeechError> {
    let path = directory.join(WEAK_FORMS_FILE);
    if fs::read_to_string(&path).is_ok_and(|text| text == WEAK_FORMS) {
        return Ok(path);
    }
    fs::write(&path, WEAK_FORMS).map_err(|_| SpeechError::Load(model))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_word_of_each_line_is_listed() {
        let lexicon = Lexicon::parse("hello h ə l \u{2c8} O\nworld w \u{2c8} ɜ ɹ l d\n");
        assert!(lexicon.contains("hello") && lexicon.contains("world"));
        assert!(!lexicon.contains("h") && !lexicon.contains("hello world"));
    }
}
