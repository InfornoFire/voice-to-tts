//! Text rewritten so that a voice reading it from a lexicon can say all of it.
//!
//! Such a voice skips every word it has no entry for, and cannot read digits at all.

mod numbers;

/// The shortest word that is a piece of a longer one.
const MIN_PIECE_LENGTH: usize = 3;

/// Unknown words of up to this many letters are spelled out.
const MAX_SPELLED_LENGTH: usize = 3;

const ORDINAL_SUFFIXES: [&str; 4] = ["st", "nd", "rd", "th"];

/// Rewrites `text` so that the words `is_known` rejects are spoken anyway.
///
/// Numbers become words. An unknown word is read as the known words it is made of, or spelled
/// out letter by letter when it is an acronym, has no vowel, or has no such parts. Everything
/// else, including the punctuation, stays as it was.
pub fn speakable(text: &str, is_known: impl Fn(&str) -> bool) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut spoken = String::with_capacity(text.len());
    let mut position = 0;
    while position < characters.len() {
        let character = characters[position];
        let before_digit = characters
            .get(position + 1)
            .is_some_and(char::is_ascii_digit);
        position = if character.is_ascii_digit() || (character == '$' && before_digit) {
            push_number(&characters, position, &mut spoken)
        } else if character.is_alphabetic() {
            push_word(&characters, position, &is_known, &mut spoken)
        } else {
            spoken.push(character);
            position + 1
        };
    }
    spoken
}

/// Appends the number that starts at `start` and returns the position after it.
fn push_number(characters: &[char], start: usize, spoken: &mut String) -> usize {
    let is_currency = characters[start] == '$';
    let number_start = start + usize::from(is_currency);
    let mut end = number_start;
    let mut has_fraction = false;
    while end < characters.len() {
        let character = characters[end];
        let continues = character.is_ascii_digit()
            || (character == '.' && !has_fraction && is_digit_at(characters, end + 1))
            || (character == ',' && is_group_of_three(characters, end + 1));
        if !continues {
            break;
        }
        has_fraction |= character == '.';
        end += 1;
    }
    let number: String = characters[number_start..end]
        .iter()
        .filter(|character| **character != ',')
        .collect();

    let suffix: String = characters[end..]
        .iter()
        .take_while(|character| character.is_ascii_alphabetic())
        .collect();
    let is_ordinal = !has_fraction && ORDINAL_SUFFIXES.contains(&suffix.to_lowercase().as_str());
    if is_ordinal {
        spoken.push_str(&numbers::ordinal(&number));
        end += suffix.len();
    } else {
        spoken.push_str(&numbers::cardinal(&number));
    }

    if is_currency {
        spoken.push_str(" dollars");
    }
    if characters.get(end) == Some(&'%') {
        spoken.push_str(" percent");
        end += 1;
    }
    if characters
        .get(end)
        .is_some_and(|character| character.is_alphanumeric())
    {
        spoken.push(' ');
    }
    end
}

/// Appends the word that starts at `start` and returns the position after it.
fn push_word(
    characters: &[char],
    start: usize,
    is_known: &impl Fn(&str) -> bool,
    spoken: &mut String,
) -> usize {
    let mut end = start;
    while end < characters.len() {
        let character = characters[end];
        let joins_letters = matches!(character, '\'' | '\u{2019}' | '-')
            && characters
                .get(end + 1)
                .is_some_and(|next| next.is_alphabetic());
        if !(character.is_alphanumeric() || joins_letters) {
            break;
        }
        end += 1;
    }
    let word: String = characters[start..end].iter().collect();
    spoken.push_str(&speakable_word(&word, is_known));
    end
}

fn speakable_word(word: &str, is_known: &impl Fn(&str) -> bool) -> String {
    let word = word.replace('\u{2019}', "'");
    if is_known(&word.to_lowercase()) {
        return word;
    }
    word.split('-')
        .map(|part| speakable_part(part, is_known))
        .collect::<Vec<_>>()
        .join(" ")
}

fn speakable_part(part: &str, is_known: &impl Fn(&str) -> bool) -> String {
    let lowercase = part.to_lowercase();
    if is_known(&lowercase) {
        return part.to_owned();
    }
    if part.chars().any(|character| character.is_ascii_digit()) {
        return spell(part);
    }
    if let Some(owner) = lowercase.strip_suffix("'s")
        && is_known(owner)
    {
        return owner.to_owned();
    }

    let letters: String = lowercase
        .chars()
        .filter(|character| *character != '\'')
        .collect();
    if is_known(&letters) {
        return letters;
    }
    let is_acronym =
        part.chars().count() > 1 && part.chars().all(|character| !character.is_lowercase());
    let has_vowel = letters.chars().any(|letter| "aeiouy".contains(letter));
    if is_acronym || !has_vowel || letters.chars().count() <= MAX_SPELLED_LENGTH {
        return spell(&letters);
    }
    match known_pieces(&letters, is_known) {
        Some(pieces) => pieces.join(" "),
        None => spell(&letters),
    }
}

/// Spells `text` as separate letters and digits.
fn spell(text: &str) -> String {
    text.chars()
        .filter(|character| character.is_alphanumeric())
        .map(|character| match character.to_digit(10) {
            Some(digit) => numbers::cardinal(&digit.to_string()),
            None => character.to_lowercase().collect(),
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// Splits `word` into the fewest known words, none of them shorter than [`MIN_PIECE_LENGTH`].
fn known_pieces(word: &str, is_known: &impl Fn(&str) -> bool) -> Option<Vec<String>> {
    let characters: Vec<char> = word.chars().collect();
    let length = characters.len();
    // For each length, the fewest pieces that make up that many characters and where the last starts.
    let mut best: Vec<Option<(usize, usize)>> = vec![None; length + 1];
    best[0] = Some((0, 0));
    for end in MIN_PIECE_LENGTH..=length {
        for start in 0..=end - MIN_PIECE_LENGTH {
            let Some((pieces, _)) = best[start] else {
                continue;
            };
            let piece: String = characters[start..end].iter().collect();
            if is_known(&piece) && best[end].is_none_or(|(fewest, _)| pieces + 1 < fewest) {
                best[end] = Some((pieces + 1, start));
            }
        }
    }

    let mut pieces = Vec::new();
    let mut end = length;
    while end > 0 {
        let (_, start) = best[end]?;
        pieces.push(characters[start..end].iter().collect());
        end = start;
    }
    pieces.reverse();
    Some(pieces)
}

fn is_digit_at(characters: &[char], position: usize) -> bool {
    characters.get(position).is_some_and(char::is_ascii_digit)
}

/// Whether exactly three digits start at `position`, as after the comma in "1,000".
fn is_group_of_three(characters: &[char], position: usize) -> bool {
    (position..position + 3).all(|index| is_digit_at(characters, index))
        && !is_digit_at(characters, position + 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_known(word: &str) -> bool {
        [
            "a", "the", "hello", "world", "cat", "cats", "dog", "bot", "discord", "don't", "have",
            "i", "it", "costs", "up", "today",
        ]
        .contains(&word)
    }

    #[test]
    fn unknown_words_are_spelled_or_taken_apart() {
        let cases = [
            ("Hello world, the dog!", "Hello world, the dog!"),
            ("Don\u{2019}t", "Don't"),
            ("a TTS", "a t t s"),
            ("ABC", "a b c"),
            ("hello tts", "hello t t s"),
            ("pfft", "p f f t"),
            ("discordbot", "discord bot"),
            ("cat's", "cat"),
            ("cat-bot", "cat bot"),
            ("Zorp", "z o r p"),
        ];
        for (text, expected) in cases {
            assert_eq!(speakable(text, is_known), expected, "{text}");
        }
    }

    #[test]
    fn numbers_are_read_as_words() {
        let cases = [
            ("I have 3 cats", "I have three cats"),
            ("The 3rd dog", "The third dog"),
            (
                "It costs $1,250.",
                "It costs one thousand two hundred fifty dollars.",
            ),
            ("Up 50% today", "Up fifty percent today"),
            ("The 4k dog", "The four k dog"),
            ("mp3", "m p three"),
        ];
        for (text, expected) in cases {
            assert_eq!(speakable(text, is_known), expected, "{text}");
        }
    }
}
