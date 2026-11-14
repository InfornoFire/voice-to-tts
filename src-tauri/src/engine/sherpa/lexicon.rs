//! The words the voice can pronounce.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::engine::SpeechError;
use crate::engine::speakable::ending_entries;

const WEAK_FORMS_FILE: &str = "weak-forms.txt";
const ENDINGS_FILE: &str = "endings.txt";

/// Pronunciations that replace the lexicon's, in its format.
///
/// The lexicon only lists the strong forms of "a", "the" and "to", which sound wrong in running
/// speech.
const WEAK_FORMS: &str = "a \u{250}\nthe \u{f0} \u{259}\nto t \u{259}\n";

/// The words a pronunciation lexicon has an entry for, with the last phoneme of each.
pub struct Lexicon {
    final_phonemes: HashMap<String, char>,
}

impl Lexicon {
    /// Reads the lexicon in `path`, with one `word phonemes` entry per line.
    pub fn load(path: &Path, model: &'static str) -> Result<Self, SpeechError> {
        let text = fs::read_to_string(path).map_err(|_| SpeechError::Load(model))?;
        Ok(Self::parse(&text))
    }

    fn parse(text: &str) -> Self {
        let final_phonemes = text
            .lines()
            .filter_map(|line| {
                let (word, phonemes) = line.split_once(' ')?;
                Some((word.to_owned(), phonemes.trim_end().chars().next_back()?))
            })
            .collect();
        Self { final_phonemes }
    }

    /// The last phoneme of `word`, in lowercase, or `None` when it has no entry.
    pub fn final_phoneme(&self, word: &str) -> Option<char> {
        self.final_phonemes.get(word).copied()
    }
}

/// Writes the replacement pronunciations into `directory` and returns the path of the file.
pub fn write_weak_forms(directory: &Path, model: &'static str) -> Result<PathBuf, SpeechError> {
    write_entries(&directory.join(WEAK_FORMS_FILE), WEAK_FORMS, model)
}

/// Writes the words that carry the sounds of inflected endings into `directory` and returns the
/// path of the file.
pub fn write_endings(directory: &Path, model: &'static str) -> Result<PathBuf, SpeechError> {
    write_entries(&directory.join(ENDINGS_FILE), &ending_entries(), model)
}

fn write_entries(path: &Path, entries: &str, model: &'static str) -> Result<PathBuf, SpeechError> {
    if !fs::read_to_string(path).is_ok_and(|text| text == entries) {
        fs::write(path, entries).map_err(|_| SpeechError::Load(model))?;
    }
    Ok(path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_phoneme_of_each_line_is_listed() {
        let lexicon = Lexicon::parse("hello h l O\nworld w r l d\n");
        assert_eq!(lexicon.final_phoneme("hello"), Some('O'));
        assert_eq!(lexicon.final_phoneme("world"), Some('d'));
        assert_eq!(lexicon.final_phoneme("h"), None);
        assert_eq!(lexicon.final_phoneme("hello world"), None);
    }
}
