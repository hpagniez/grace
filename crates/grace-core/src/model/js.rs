//! What the TypeScript of `src/shared` took from JavaScript without saying so, and the model's ports keep (plan P1,
//! porting rules 3 and 4): a string's `length` in UTF-16 units, the default `sort()` order, `\s` and `trim()`, the
//! order of an object's keys, `Number.isInteger`, and `localeCompare`. Each function says what it keeps.
use serde_json::Value;
use std::cmp::Ordering;

/// JavaScript's `s.length`: UTF-16 code units, not bytes or characters.
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// The order of JavaScript's default `sort()` (and of `<` on strings): by UTF-16 code unit. It differs from Rust's
/// `str` order (by code point) only between a character above U+FFFF and one in U+E000..=U+FFFF.
pub fn cmp_utf16(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// Sorts as JavaScript's `list.sort()` does.
pub fn sort_utf16(list: &mut [String]) {
    list.sort_by(|a, b| cmp_utf16(a, b));
}

/// JavaScript's `\s` (and what `trim()` strips): the WhiteSpace and LineTerminator of ECMAScript. Rust's
/// `char::is_whitespace` differs by two: it has U+0085, which JavaScript does not, and lacks U+FEFF, which it does.
pub fn is_space(c: char) -> bool {
    c == '\u{feff}' || (c.is_whitespace() && c != '\u{85}')
}

/// `s.trim()` of JavaScript.
pub fn trim(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// Whether JavaScript treats the property name as an array index: the canonical text of an integer from 0 to 2^32-2.
fn is_array_index(key: &str) -> bool {
    let canonical = key == "0" || (!key.starts_with('0') && key.bytes().all(|b| b.is_ascii_digit()));
    canonical && !key.is_empty() && key.parse::<u32>().is_ok_and(|n| n != u32::MAX)
}

/// The order in which JavaScript lists an object's own keys (`Object.keys`, `Object.entries`) when they were added in
/// `keys` order: the array indexes first, in ascending order, then the rest as added. An environment named `10` is
/// listed after one named `2` and before `dev`, however they were written.
pub fn key_order<'a>(keys: impl IntoIterator<Item = &'a String>) -> Vec<&'a String> {
    let (mut indexes, others): (Vec<&String>, Vec<&String>) = keys.into_iter().partition(|k| is_array_index(k));
    indexes.sort_by_key(|k| k.parse::<u32>().unwrap_or(u32::MAX));
    indexes.extend(others);
    indexes
}

/// `typeof x === "number" && Number.isFinite(x)`: a JSON number. (JSON has no `NaN` or `Infinity`; `serde_json` refuses
/// a number out of the range of `f64`, where `JSON.parse` would read `Infinity`, and so rejects the whole file.)
pub fn finite(x: &Value) -> Option<f64> {
    x.as_f64().filter(|n| n.is_finite())
}

/// `typeof x === "number" && Number.isInteger(x)`: `4` and `4.0` are both whole numbers in JavaScript.
pub fn integer(x: &Value) -> Option<f64> {
    finite(x).filter(|n| n.fract() == 0.0)
}

/// `integer` for a value that must also fit an unsigned type: a whole number in `0..=max`.
pub fn unsigned(x: &Value, max: u64) -> Option<u64> {
    // 2^64 as f64 is the first value past u64; every whole f64 below it converts exactly.
    integer(x)
        .filter(|n| *n >= 0.0 && *n < 18_446_744_073_709_551_616.0)
        .map(|n| n as u64)
        .filter(|n| *n <= max)
}

/// The primary weight of a character in the CLDR root collation (what `localeCompare` uses with no locale given, and
/// in English): whitespace, then punctuation in this order, then digits, then letters; `None` for what the collation
/// ignores (control characters).
fn primary(c: char) -> Option<u32> {
    const PUNCTUATION: &str = "\t\n\u{b}\u{c}\r _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";
    if let Some(i) = PUNCTUATION.chars().position(|p| p == c) {
        return Some(i as u32);
    }
    match c {
        '0'..='9' => Some(100 + (c as u32 - '0' as u32)),
        'a'..='z' | 'A'..='Z' => Some(200 + (c.to_ascii_lowercase() as u32 - 'a' as u32)),
        c if c.is_control() => None,
        // Not collated: after the letters, by code point, case folded.
        c => Some(1000 + c.to_lowercase().next().unwrap_or(c) as u32),
    }
}

/// An approximation of `a.localeCompare(b)` with no locale given. It is exact for printable ASCII, which is what the
/// names Grace compares are (environments, `[A-Za-z0-9._-]`): the primary weights of the characters decide first
/// (letters without regard to case, `_` before `-` before `.` before digits before letters), and only when they are
/// all equal does case decide, a lower-case letter before an upper-case one (`dev` before `Dev`). A character outside
/// ASCII sorts after the letters by code point: accents are not collated.
pub fn locale_compare(a: &str, b: &str) -> Ordering {
    let weights = |s: &str| s.chars().filter_map(primary).collect::<Vec<_>>();
    let cases = |s: &str| {
        s.chars()
            .filter(|c| primary(*c).is_some())
            .map(char::is_uppercase)
            .collect::<Vec<_>>()
    };
    weights(a).cmp(&weights(b)).then_with(|| cases(a).cmp(&cases(b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn counts_utf16_units() {
        assert_eq!(utf16_len("abc"), 3);
        assert_eq!(utf16_len("é"), 1);
        assert_eq!(utf16_len("日本"), 2);
        assert_eq!(utf16_len("😀"), 2); // JavaScript: "😀".length === 2
    }

    #[test]
    fn sorts_by_utf16_unit_not_by_code_point() {
        // U+1F600 is the pair D83D DE00, which sorts before U+FF5E; by code point it sorts after.
        let mut list = vec!["\u{ff5e}".to_string(), "\u{1f600}".to_string(), "a".to_string()];
        sort_utf16(&mut list);
        assert_eq!(list, ["a", "\u{1f600}", "\u{ff5e}"]);
    }

    #[test]
    fn javascript_spaces_are_not_rusts() {
        for c in [
            '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2003}', '\u{2028}', '\u{3000}',
            '\u{feff}',
        ] {
            assert!(is_space(c), "{c:?}");
        }
        for c in ['\u{85}', '\u{200b}', '\u{180e}', 'a', '\0'] {
            assert!(!is_space(c), "{c:?}");
        }
        assert_eq!(trim("\u{feff} a b\u{85}\n"), "a b\u{85}");
    }

    #[test]
    fn lists_array_indexes_first_in_ascending_order() {
        let keys: Vec<String> = ["b", "10", "2", "a", "01", "4294967295", "4294967294", "0"]
            .map(String::from)
            .to_vec();
        let order: Vec<&str> = key_order(&keys).into_iter().map(String::as_str).collect();
        assert_eq!(order, ["0", "2", "10", "4294967294", "b", "a", "01", "4294967295"]);
    }

    #[test]
    fn reads_whole_numbers_as_javascript_does() {
        assert_eq!(integer(&json!(4)), Some(4.0));
        assert_eq!(integer(&json!(4.0)), Some(4.0));
        assert_eq!(integer(&json!(-3)), Some(-3.0));
        assert_eq!(integer(&json!(2.5)), None);
        assert_eq!(integer(&json!("4")), None);
        assert_eq!(integer(&Value::Null), None);
        assert_eq!(finite(&json!(2.5)), Some(2.5));
        assert_eq!(finite(&json!(true)), None);
        assert_eq!(unsigned(&json!(7), 32), Some(7));
        assert_eq!(unsigned(&json!(7.0), 32), Some(7));
        assert_eq!(unsigned(&json!(33), 32), None);
        assert_eq!(unsigned(&json!(-1), 32), None);
        assert_eq!(unsigned(&json!(1e30), u64::MAX), None);
    }

    #[test]
    fn collates_like_the_root_locale_for_ascii() {
        let sorted = |mut v: Vec<&'static str>| {
            v.sort_by(|a, b| locale_compare(a, b));
            v
        };
        // from `["dev","Dev","DEV","a-b","ab","a_b","a.b","a1","aB","Ab","a","A","b","B"].sort(localeCompare)` in Node 22
        assert_eq!(
            sorted(vec![
                "dev", "Dev", "DEV", "a-b", "ab", "a_b", "a.b", "a1", "aB", "Ab", "a", "A", "b", "B"
            ]),
            [
                "a", "A", "a_b", "a-b", "a.b", "a1", "ab", "aB", "Ab", "b", "B", "dev", "Dev", "DEV"
            ]
        );
        assert_eq!(locale_compare("a", "A"), Ordering::Less);
        assert_eq!(locale_compare("ab", "Aa"), Ordering::Greater);
        assert_eq!(locale_compare("a\u{1}b", "ab"), Ordering::Equal);
        assert_eq!(locale_compare("", "a"), Ordering::Less);
    }
}
