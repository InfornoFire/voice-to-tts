//! Numbers written with digits, read as English words.

const SMALL: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];

const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

const SCALES: [(u64, &str); 4] = [
    (1_000_000_000_000, "trillion"),
    (1_000_000_000, "billion"),
    (1_000_000, "million"),
    (1_000, "thousand"),
];

/// Longer numbers are read digit by digit.
const MAX_DIGITS: usize = 15;

/// Reads `number`, which has digits and at most one decimal point.
pub fn cardinal(number: &str) -> String {
    match number.split_once('.') {
        Some((whole, fraction)) => format!("{} point {}", whole_number(whole), digits(fraction)),
        None => whole_number(number),
    }
}

/// Reads `number`, which has only digits, as a position such as "third".
pub fn ordinal(number: &str) -> String {
    let words = whole_number(number);
    match words.rsplit_once(' ') {
        Some((head, last)) => format!("{head} {}", ordinal_word(last)),
        None => ordinal_word(&words),
    }
}

fn whole_number(number: &str) -> String {
    let has_leading_zero = number.len() > 1 && number.starts_with('0');
    match number.parse::<u64>() {
        Ok(value) if !has_leading_zero && number.len() <= MAX_DIGITS => words(value),
        _ => digits(number),
    }
}

fn digits(number: &str) -> String {
    number
        .chars()
        .filter_map(|digit| digit.to_digit(10))
        .map(|digit| SMALL[digit as usize])
        .collect::<Vec<_>>()
        .join(" ")
}

fn words(value: u64) -> String {
    let Some(&(scale, name)) = SCALES.iter().find(|(scale, _)| value >= *scale) else {
        return below_thousand(value);
    };
    let head = format!("{} {name}", words(value / scale));
    match value % scale {
        0 => head,
        rest => format!("{head} {}", words(rest)),
    }
}

fn below_thousand(value: u64) -> String {
    match value {
        0..20 => SMALL[value as usize].to_owned(),
        20..100 => {
            let tens = TENS[(value / 10) as usize];
            match value % 10 {
                0 => tens.to_owned(),
                ones => format!("{tens} {}", SMALL[ones as usize]),
            }
        }
        _ => {
            let head = format!("{} hundred", SMALL[(value / 100) as usize]);
            match value % 100 {
                0 => head,
                rest => format!("{head} {}", below_thousand(rest)),
            }
        }
    }
}

fn ordinal_word(word: &str) -> String {
    match word {
        "one" => "first".to_owned(),
        "two" => "second".to_owned(),
        "three" => "third".to_owned(),
        "five" => "fifth".to_owned(),
        "eight" => "eighth".to_owned(),
        "nine" => "ninth".to_owned(),
        "twelve" => "twelfth".to_owned(),
        _ => match word.strip_suffix('y') {
            Some(stem) => format!("{stem}ieth"),
            None => format!("{word}th"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_read_as_words() {
        let cases = [
            ("0", "zero"),
            ("7", "seven"),
            ("20", "twenty"),
            ("25", "twenty five"),
            ("101", "one hundred one"),
            ("1000", "one thousand"),
            (
                "1234567",
                "one million two hundred thirty four thousand five hundred sixty seven",
            ),
            ("007", "zero zero seven"),
            ("3.14", "three point one four"),
        ];
        for (number, expected) in cases {
            assert_eq!(cardinal(number), expected, "{number}");
        }
    }

    #[test]
    fn positions_are_read_as_ordinal_words() {
        let cases = [
            ("1", "first"),
            ("3", "third"),
            ("12", "twelfth"),
            ("20", "twentieth"),
            ("21", "twenty first"),
            ("100", "one hundredth"),
        ];
        for (number, expected) in cases {
            assert_eq!(ordinal(number), expected, "{number}");
        }
    }
}
