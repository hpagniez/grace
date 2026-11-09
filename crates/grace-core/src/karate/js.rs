//! The JavaScript behaviours the TypeScript modules lean on, so the Rust ports give the same results on the same input
//! (porting rules 3 and 4 of `docs/plans/2026-11-09-gpui-p1-core.md`): what `trim`, `\s` and `split(/\r?\n/)` count as
//! space and line end, what a string's `length` is, how a number is written, the order `Object.entries` gives, and what
//! reading a property of a Karate event's data does when the data is not what Grace expects.
use super::events::DataError;
use serde_json::{Map, Value};
use std::cmp::Ordering;

/// JavaScript's white space and line terminators, as the inside of a regex class: what `\s` matches and `trim` removes.
/// Rust's `\s` differs (it has U+0085 and not U+FEFF).
pub(crate) const WS: &str =
    r"\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}";

/// JavaScript's `.` (without the `s` flag): anything but a line terminator, as a regex class.
pub(crate) const DOT: &str = r"[^\n\r\x{2028}\x{2029}]";

pub(crate) fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{B}' | '\u{C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}'
    )
}

/// `String.prototype.trim`.
pub(crate) fn trim(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// `s.length`: UTF-16 units.
pub(crate) fn len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// JavaScript's `<` on strings (and the default `sort()`): UTF-16 code units, not bytes.
pub(crate) fn cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// `text.split(/\r?\n/)`. A `\r` that no `\n` follows stays in its line.
pub(crate) fn lines(text: &str) -> Vec<&str> {
    let mut out: Vec<&str> = text.split('\n').collect();
    let last = out.len() - 1;
    for line in &mut out[..last] {
        *line = line.strip_suffix('\r').unwrap_or(line);
    }
    out
}

/// `String(n)` for a number: the shortest digits that read back, written as ECMAScript's Number::toString does
/// (`1`, not `1.0`; `1e+21`; `1e-7`).
pub(crate) fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n == 0.0 {
        return "0".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if n < 0.0 {
        return format!("-{}", number_to_string(-n));
    }
    // Rust's `{:e}` is the shortest round-trip digits as `d.ddde<exp>`: k digits, the point after the first.
    let sci = format!("{n:e}");
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let point = exp.parse::<i32>().unwrap_or(0) + 1; // the value is 0.<digits> x 10^point
    let zeros = |count: i32| "0".repeat(count.max(0) as usize);
    if k <= point && point <= 21 {
        format!("{digits}{}", zeros(point - k))
    } else if 0 < point && point <= 21 {
        format!("{}.{}", &digits[..point as usize], &digits[point as usize..])
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", zeros(-point))
    } else {
        let e = point - 1;
        let sign = if e < 0 { '-' } else { '+' };
        if k == 1 {
            format!("{digits}e{sign}{}", e.abs())
        } else {
            format!("{}.{}e{sign}{}", &digits[..1], &digits[1..], e.abs())
        }
    }
}

/// A number of an event's data as JavaScript would have it, for a place where Karate writes a whole number.
pub(crate) fn as_u32(v: &Value) -> Option<u32> {
    v.as_f64()
        .filter(|n| n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(u32::MAX))
        .map(|n| n as u32)
}

/// JavaScript truthiness of a JSON value (`undefined` and `null` are both `Null` here).
pub(crate) fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `String(v)` as a template literal does it, for the values that Karate's events hold. `undefined` and `null` are
/// both `Null` here and read `undefined`.
pub(crate) fn display(v: &Value) -> String {
    match v {
        Value::Null => "undefined".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => match (n.as_i64(), n.as_u64()) {
            (Some(i), _) => i.to_string(),
            (_, Some(u)) => u.to_string(),
            _ => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        },
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(display).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

static NULL: Value = Value::Null;

/// `v.key`. A missing property is `undefined`, and so is any property of a number, string or list; `Null` stands for
/// both `undefined` and `null`. Reading a property of a missing value throws in JavaScript, and is an error here.
pub(crate) fn prop<'a>(v: &'a Value, key: &str) -> Result<&'a Value, DataError> {
    match v {
        Value::Null => Err(DataError::new(format!(
            "Karate event data: cannot read `{key}` of a missing value"
        ))),
        Value::Object(map) => Ok(map.get(key).unwrap_or(&NULL)),
        _ => Ok(&NULL),
    }
}

/// `for (const x of v ?? [])`: a missing list is empty; a value that is not a list is an error (JavaScript throws on a
/// number or an object, and `scenarioResults`, `stepResults` and `callResults` are never strings).
pub(crate) fn items<'a>(v: &'a Value, what: &str) -> Result<&'a [Value], DataError> {
    match v {
        Value::Null => Ok(&[]),
        Value::Array(items) => Ok(items),
        _ => Err(DataError::new(format!("Karate event data: `{what}` is not a list"))),
    }
}

/// The text of a string, `None` for anything else.
pub(crate) fn text(v: &Value) -> Option<&str> {
    v.as_str()
}

/// A list of strings (`tags`, `comments`) as a `Vec`: a missing list is empty, and an element that is not a string is
/// left out.
pub(crate) fn strings(v: &Value) -> Vec<String> {
    match v {
        Value::Array(items) => items.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

fn array_index(key: &str) -> Option<u32> {
    key.parse::<u32>()
        .ok()
        .filter(|n| *n != u32::MAX && n.to_string() == key)
}

/// `Object.entries(map)`: the keys that are array indices first, ascending, then the others in the order they were
/// written.
pub(crate) fn entries(map: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut indices: Vec<(u32, &String, &Value)> = Vec::new();
    let mut named: Vec<(&String, &Value)> = Vec::new();
    for (key, value) in map {
        match array_index(key) {
            Some(n) => indices.push((n, key, value)),
            None => named.push((key, value)),
        }
    }
    indices.sort_by_key(|(n, _, _)| *n);
    indices.into_iter().map(|(_, k, v)| (k, v)).chain(named).collect()
}

/// `JSON.stringify(v)` for a value parsed from JSON: no spaces, numbers as JavaScript writes them, an object's keys in
/// `Object.entries` order.
pub(crate) fn json_stringify(v: &Value) -> String {
    let mut out = String::new();
    write_json(v, &mut out);
    out
}

fn write_json(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(_) => out.push_str(&display(v)),
        Value::String(s) => out.push_str(&serde_json::to_string(s).unwrap_or_default()),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (i, (key, value)) in entries(map).into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_default());
                out.push(':');
                write_json(value, out);
            }
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    // New in the port: Rust has no built-in of these, and each rule is one the TypeScript got from the language.
    use super::*;
    use serde_json::json;

    #[test]
    fn trim_removes_what_javascript_removes() {
        assert_eq!(trim("\u{FEFF} a b \u{A0}\r\n"), "a b");
        // U+0085 is white space for Rust's `trim` and not for JavaScript's
        assert_eq!(trim("\u{85}a"), "\u{85}a");
    }

    #[test]
    fn lines_split_on_an_optional_carriage_return_and_a_newline() {
        assert_eq!(lines("a\r\nb\nc\rd"), ["a", "b", "c\rd"]);
        assert_eq!(lines("a\n"), ["a", ""]);
        assert_eq!(lines(""), [""]);
        assert_eq!(lines("a\r"), ["a\r"]);
    }

    #[test]
    fn len_counts_utf16_units() {
        assert_eq!(len("aé"), 2);
        assert_eq!(len("a😀"), 3);
    }

    #[test]
    fn cmp_compares_utf16_units() {
        // U+FF5E is one unit 0xFF5E; U+1F600 is the units 0xD83D 0xDE00: UTF-16 puts the emoji first, UTF-8 last
        assert_eq!(cmp("\u{1F600}", "\u{FF5E}"), Ordering::Less);
        assert!("\u{1F600}" > "\u{FF5E}");
    }

    #[test]
    fn numbers_are_written_as_javascript_writes_them() {
        for (n, s) in [
            (1.0, "1"),
            (100.0, "100"),
            (-0.0, "0"),
            (1.5, "1.5"),
            (0.1, "0.1"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e21, "1e+21"),
            (1.5e300, "1.5e+300"),
            (0.000001, "0.000001"),
            (1e-7, "1e-7"),
            (1.25e-7, "1.25e-7"),
            (-2.5, "-2.5"),
        ] {
            assert_eq!(number_to_string(n), s, "{n}");
        }
    }

    #[test]
    fn json_stringify_matches_javascript() {
        let v: Value =
            serde_json::from_str(r#"{"b":1.0,"a":[1,"x\ny",null,true],"2":{"x":1e21},"1":0.5,"s":"é\u0001"}"#).unwrap();
        assert_eq!(
            json_stringify(&v),
            r#"{"1":0.5,"2":{"x":1e+21},"b":1,"a":[1,"x\ny",null,true],"s":"é\u0001"}"#
        );
    }

    #[test]
    fn entries_put_array_indices_first() {
        let v: Value = serde_json::from_str(r#"{"z":1,"10":2,"a":3,"2":4,"01":5}"#).unwrap();
        let keys: Vec<&str> = entries(v.as_object().unwrap())
            .into_iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(keys, ["2", "10", "z", "a", "01"]);
    }

    #[test]
    fn prop_reads_like_javascript() {
        let v = json!({"a": {"b": 1}, "n": null});
        assert_eq!(prop(prop(&v, "a").unwrap(), "b").unwrap(), &json!(1));
        assert_eq!(prop(&v, "missing").unwrap(), &Value::Null);
        assert_eq!(prop(&json!(42), "a").unwrap(), &Value::Null);
        assert!(prop(prop(&v, "n").unwrap(), "a").is_err());
    }

    #[test]
    fn items_take_a_list_or_nothing() {
        assert_eq!(items(&Value::Null, "x").unwrap().len(), 0);
        assert_eq!(items(&json!([1, 2]), "x").unwrap().len(), 2);
        assert!(items(&json!(42), "x").is_err());
    }

    #[test]
    fn truthiness_follows_javascript() {
        for (v, t) in [
            (json!(null), false),
            (json!(0), false),
            (json!(0.0), false),
            (json!(""), false),
            (json!(false), false),
            (json!("0"), true),
            (json!([]), true),
            (json!({}), true),
            (json!(1), true),
        ] {
            assert_eq!(truthy(&v), t, "{v}");
        }
    }
}
