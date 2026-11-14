//! Regular inflected forms of known words.
//!
//! A pronunciation lexicon lists the base words but only some of their plurals, past tenses and
//! participles. A form it lacks is written as its stem, a hyphen and a word that only has the
//! sound of the ending. The voice says the parts of a hyphenated word without a pause, so they
//! are heard as one word.

/// The shortest stem of an inflected word.
const MIN_STEM_LENGTH: usize = 3;

/// Consonants that are doubled before an ending, as in "stopped".
const DOUBLED_CONSONANTS: &str = "bcdgklmnprstvxz";

/// Phonemes before which a plural is said with an unvoiced s.
const VOICELESS_BEFORE_S: &str = "ptkf\u{3b8}";

/// Phonemes before which a plural is said with an extra vowel.
const SIBILANTS: &str = "sz\u{283}\u{292}\u{2a7}\u{2a4}";

/// Phonemes before which a past tense is said with an unvoiced t.
const VOICELESS_BEFORE_ED: &str = "pkf\u{3b8}\u{283}s\u{2a7}";

/// The sound of an ending, carried by a word that is not an English one.
#[derive(Clone, Copy)]
enum Ending {
    S,
    Z,
    Iz,
    T,
    D,
    Id,
    Ing,
}

impl Ending {
    const ALL: [Self; 7] = [
        Self::S,
        Self::Z,
        Self::Iz,
        Self::T,
        Self::D,
        Self::Id,
        Self::Ing,
    ];

    /// The word that carries the sound and the phonemes of that word.
    fn entry(self) -> (&'static str, &'static str) {
        match self {
            Self::S => ("sfxs", "s"),
            Self::Z => ("sfxz", "z"),
            Self::Iz => ("sfxiz", "\u{1d7b} z"),
            Self::T => ("sfxt", "t"),
            Self::D => ("sfxd", "d"),
            Self::Id => ("sfxid", "\u{1d7b} d"),
            Self::Ing => ("sfxing", "\u{26a} \u{14b}"),
        }
    }
}

/// The lexicon entries of the ending words, one per line, which an inflected word relies on.
pub fn ending_entries() -> String {
    Ending::ALL
        .iter()
        .map(|ending| {
            let (word, phonemes) = ending.entry();
            format!("{word} {phonemes}\n")
        })
        .collect()
}

/// Writes `word`, in lowercase, as a known stem and the sound of its ending.
///
/// `final_phoneme` gives the last phoneme of a known word. Returns `None` when `word` is not a
/// regular plural, past tense or participle of a known word.
pub fn inflected(word: &str, final_phoneme: &impl Fn(&str) -> Option<char>) -> Option<String> {
    let (stem, ending) = plural(word, final_phoneme)
        .or_else(|| past(word, final_phoneme))
        .or_else(|| participle(word, final_phoneme))?;
    Some(format!("{stem}-{}", ending.entry().0))
}

fn plural(word: &str, final_phoneme: &impl Fn(&str) -> Option<char>) -> Option<(String, Ending)> {
    let stems = [
        word.strip_suffix('s')
            .filter(|stem| !stem.ends_with('s'))
            .map(str::to_owned),
        word.strip_suffix("es").map(str::to_owned),
        word.strip_suffix("ies").map(|stem| format!("{stem}y")),
    ];
    let (stem, last) = known_stem(stems, final_phoneme)?;
    let ending = if VOICELESS_BEFORE_S.contains(last) {
        Ending::S
    } else if SIBILANTS.contains(last) {
        Ending::Iz
    } else {
        Ending::Z
    };
    Some((stem, ending))
}

fn past(word: &str, final_phoneme: &impl Fn(&str) -> Option<char>) -> Option<(String, Ending)> {
    let stems = [
        word.strip_suffix('d')
            .filter(|stem| !stem.ends_with('d'))
            .map(str::to_owned),
        word.strip_suffix("ed")
            .filter(|stem| !stem.ends_with('e'))
            .map(str::to_owned),
        word.strip_suffix("ied").map(|stem| format!("{stem}y")),
        word.strip_suffix("ed").and_then(undoubled),
    ];
    let (stem, last) = known_stem(stems, final_phoneme)?;
    let ending = if VOICELESS_BEFORE_ED.contains(last) {
        Ending::T
    } else if last == 't' || last == 'd' {
        Ending::Id
    } else {
        Ending::D
    };
    Some((stem, ending))
}

fn participle(
    word: &str,
    final_phoneme: &impl Fn(&str) -> Option<char>,
) -> Option<(String, Ending)> {
    let stem = word.strip_suffix("ing")?;
    let stems = [
        Some(stem.to_owned()),
        Some(format!("{stem}e")),
        undoubled(stem),
    ];
    let (stem, _) = known_stem(stems, final_phoneme)?;
    Some((stem, Ending::Ing))
}

/// The first of `stems` that is long enough and known, with its last phoneme.
fn known_stem(
    stems: impl IntoIterator<Item = Option<String>>,
    final_phoneme: &impl Fn(&str) -> Option<char>,
) -> Option<(String, char)> {
    stems
        .into_iter()
        .flatten()
        .filter(|stem| stem.chars().count() >= MIN_STEM_LENGTH)
        .find_map(|stem| final_phoneme(&stem).map(|last| (stem, last)))
}

/// `stem` without its last letter when that letter is doubled, as in "stopp".
fn undoubled(stem: &str) -> Option<String> {
    let mut letters = stem.chars().rev();
    let last = letters.next()?;
    (letters.next() == Some(last) && DOUBLED_CONSONANTS.contains(last))
        .then(|| stem.chars().take(stem.chars().count() - 1).collect())
}
