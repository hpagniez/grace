//! `src/main/propertiesFile.ts`: the `.properties` import of Run properties (docs/specs/2026-11-04-env-var-properties.md).
//! Reads a file by the rules of `java.util.Properties.load` and gives its keys to the window; it never writes the
//! file and no message it makes holds a value.
use std::fmt;
use std::fs;
use std::path::Path;

use indexmap::IndexMap;

use crate::model::env::{ImportEntry, ImportResult};

const MAX_BYTES: u64 = 1024 * 1024;
const BOM: [u8; 3] = [0xef, 0xbb, 0xbf];

/// The white space of a `.properties` line: space, tab, form feed.
const fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\x0c')
}

fn trim_start(s: &str) -> &str {
    s.trim_start_matches(is_space)
}

/// Whether `s` ends in an odd number of backslashes: its line goes on in the next one.
fn continues(s: &str) -> bool {
    s.chars().rev().take_while(|&c| c == '\\').count() % 2 == 1
}

/// A malformed `\uXXXX` escape. Names the line its property starts on, never a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadEscape {
    pub line: usize,
}

impl fmt::Display for BadEscape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad \\uXXXX escape on line {}", self.line)
    }
}

impl std::error::Error for BadEscape {}

/// `\t`, `\n`, `\r`, `\f`, `\uXXXX` and `\<char>` as `java.util.Properties` reads them.
///
/// Built from UTF-16 units, as a JavaScript string is: Java files write a character above U+FFFF as two `\uXXXX`
/// escapes, which meet in the result. A lone surrogate cannot be a Rust `char`, so it comes out as U+FFFD.
fn unescape(s: &str, line: usize) -> Result<String, BadEscape> {
    let mut out: Vec<u16> = Vec::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buf = [0u16; 2];
            out.extend_from_slice(c.encode_utf16(&mut buf));
            continue;
        }
        // a backslash at the end of the text is dropped
        let Some(c) = chars.next() else { break };
        match c {
            'u' => {
                let hex: String = chars.by_ref().take(4).collect();
                if hex.chars().count() != 4 || !hex.chars().all(|d| d.is_ascii_hexdigit()) {
                    return Err(BadEscape { line });
                }
                out.push(u16::from_str_radix(&hex, 16).map_err(|_| BadEscape { line })?);
            }
            't' => out.push(u16::from(b'\t')),
            'n' => out.push(u16::from(b'\n')),
            'r' => out.push(u16::from(b'\r')),
            'f' => out.push(0x0c),
            other => {
                let mut buf = [0u16; 2];
                out.extend_from_slice(other.encode_utf16(&mut buf));
            }
        }
    }
    Ok(String::from_utf16_lossy(&out))
}

/// `text.split(/\r\n|\r|\n/)`: any line ending, and a last piece after the final one.
fn split_lines(text: &str) -> Vec<&str> {
    let mut lines = vec![];
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '\r' => {
                lines.push(&text[start..i]);
                start = i + 1;
                if let Some(&(next, '\n')) = chars.peek() {
                    chars.next();
                    start = next + 1;
                }
            }
            '\n' => {
                lines.push(&text[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    lines.push(&text[start..]);
    lines
}

/// The keys of a `.properties` text, by the rules of `java.util.Properties.load`: comment and blank lines, `=`, `:`
/// or white space between key and value, continued lines and escapes. A repeated key keeps its last value at its
/// first place. Fails on a malformed `\uXXXX`, naming the line its property starts on, never a value.
pub fn parse_properties(text: &str) -> Result<Vec<ImportEntry>, BadEscape> {
    let lines = split_lines(text);
    let mut found: IndexMap<String, String> = IndexMap::new();
    let mut i = 0;
    while i < lines.len() {
        let mut logical = trim_start(lines[i]).to_string();
        if logical.is_empty() || logical.starts_with('#') || logical.starts_with('!') {
            i += 1;
            continue;
        }
        let start = i + 1;
        while continues(&logical) {
            logical.pop();
            i += 1;
            if i >= lines.len() {
                break;
            }
            logical.push_str(trim_start(lines[i]));
        }
        // the key's end
        let mut end = logical.len();
        let mut escaped = false;
        for (at, c) in logical.char_indices() {
            if !escaped && (c == '=' || c == ':' || is_space(c)) {
                end = at;
                break;
            }
            escaped = c == '\\' && !escaped;
        }
        // the value's start
        let mut value = trim_start(&logical[end..]);
        if let Some(rest) = value.strip_prefix(['=', ':']) {
            value = trim_start(rest);
        }
        let key = unescape(&logical[..end], start)?;
        found.insert(key, unescape(value, start)?);
        i += 1;
    }
    Ok(found
        .into_iter()
        .map(|(key, value)| ImportEntry { key, value })
        .collect())
}

/// The name and keys of the `.properties` file at `file`, read once and never written, or a message for the window.
/// Never fails, and no message holds a value.
pub fn read_properties_file(file: &Path) -> ImportResult {
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let read = || -> std::io::Result<Option<Vec<u8>>> {
        if fs::metadata(file)?.len() > MAX_BYTES {
            return Ok(None);
        }
        fs::read(file).map(Some)
    };
    // The text after the colon is the operating system's: Node's `ENOENT: no such file or directory, stat '<path>'`
    // has no Rust equivalent, and the path is left out.
    let mut bytes = match read() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return ImportResult::Error(format!("{name} is too large to import")),
        Err(err) => return ImportResult::Error(format!("{name} could not be read: {err}")),
    };
    // Windows editors write a byte order mark; Java would keep it in the first key's name
    if bytes.starts_with(&BOM) {
        bytes.drain(..BOM.len());
    }
    // ISO-8859-1, as Properties.load(InputStream) reads it: every byte is the character of its number
    let text: String = bytes.iter().map(|&b| char::from(b)).collect();
    match parse_properties(&text) {
        Err(err) => ImportResult::Error(format!("{name} could not be imported: {err}")),
        Ok(entries) if entries.is_empty() => ImportResult::Error(format!("{name} has no properties")),
        Ok(entries) => ImportResult::File { file: name, entries },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn entries(lines: &[&str]) -> IndexMap<String, String> {
        parse_properties(&lines.join("\n"))
            .unwrap()
            .into_iter()
            .map(|e| (e.key, e.value))
            .collect()
    }

    fn entry(key: &str, value: &str) -> ImportEntry {
        ImportEntry {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    fn map(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    mod parse_properties {
        use super::*;

        #[test]
        fn reads_keys_the_way_java_util_properties_load_does() {
            let text = [
                "# a comment",
                "! another",
                "",
                "   ",
                "a=1",
                "b : 2",
                "c 3",
                "d=x\\",
                "  y",
                "e=\\u0041",
                "t=a\\tb",
                "k\\=x=1",
                "  spaced   =   value  ",
                "empty=",
                "bare",
            ];
            let expected = map(&[
                ("a", "1"),
                ("b", "2"),
                ("c", "3"),
                ("d", "xy"),
                ("e", "A"),
                ("t", "a\tb"),
                ("k=x", "1"),
                ("spaced", "value  "),
                ("empty", ""),
                ("bare", ""),
            ]);
            // like `toEqual` on objects, `IndexMap`'s equality ignores the order; the next test pins the order
            assert_eq!(entries(&text), expected);
        }

        #[test]
        fn keeps_a_repeated_keys_last_value_at_its_first_place() {
            assert_eq!(
                parse_properties("a=1\nb=2\na=3").unwrap(),
                [entry("a", "3"), entry("b", "2")]
            );
        }

        #[test]
        fn drops_a_continuation_at_the_end_of_the_file() {
            assert_eq!(entries(&["f=1\\"]), map(&[("f", "1")]));
        }

        #[test]
        fn does_not_continue_a_comment_line_and_splits_on_any_line_ending() {
            assert_eq!(
                parse_properties("# note \\\r\na=1\rb=2\r\nc=3").unwrap(),
                [entry("a", "1"), entry("b", "2"), entry("c", "3")]
            );
        }

        #[test]
        fn keeps_an_escaped_backslash_at_a_lines_end_without_continuing() {
            assert_eq!(entries(&["p=C:\\\\", "q=1"]), map(&[("p", "C:\\"), ("q", "1")]));
        }

        #[test]
        fn refuses_a_malformed_u_escape_with_the_line_its_property_starts_on() {
            let err = parse_properties("a=1\nk=\\u00G1").unwrap_err();
            assert_eq!(err.to_string(), "bad \\uXXXX escape on line 2");
            let err = parse_properties("k=\\u12").unwrap_err();
            assert_eq!(err.to_string(), "bad \\uXXXX escape on line 1");
        }

        // Not in the TypeScript tests: what the UTF-16 handling of `unescape` is for.
        #[test]
        fn joins_the_two_escapes_of_a_character_above_ffff() {
            assert_eq!(entries(&["smile=\\uD83D\\uDE00"]), map(&[("smile", "\u{1F600}")]));
            assert_eq!(entries(&["lone=\\uD83D"]), map(&[("lone", "\u{FFFD}")]));
        }
    }

    mod read_properties_file {
        use super::*;
        use std::path::PathBuf;

        fn file(dir: &TempDir, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
            let f = dir.path().join(name);
            fs::write(&f, content).unwrap();
            f
        }

        fn dir() -> TempDir {
            tempfile::Builder::new().prefix("kk-props-").tempdir().unwrap()
        }

        #[test]
        fn gives_the_files_name_and_keys_and_leaves_the_file_as_it_was() {
            let d = dir();
            let f = file(&d, "secrets-dev.properties", "db.user=u\ndb.password=p\n");
            let before = fs::metadata(&f).unwrap().modified().unwrap();
            assert_eq!(
                read_properties_file(&f),
                ImportResult::File {
                    file: "secrets-dev.properties".to_string(),
                    entries: vec![entry("db.user", "u"), entry("db.password", "p")],
                }
            );
            assert_eq!(fs::read_to_string(&f).unwrap(), "db.user=u\ndb.password=p\n");
            assert_eq!(fs::metadata(&f).unwrap().modified().unwrap(), before);
        }

        #[test]
        fn drops_a_utf_8_byte_order_mark_and_reads_the_rest_as_iso_8859_1() {
            let d = dir();
            let mut content = vec![0xef, 0xbb, 0xbf];
            content.extend_from_slice(b"db.user=u\nw=\xe9");
            let f = file(&d, "s.properties", content);
            assert_eq!(
                read_properties_file(&f),
                ImportResult::File {
                    file: "s.properties".to_string(),
                    entries: vec![entry("db.user", "u"), entry("w", "é")],
                }
            );
        }

        #[test]
        fn refuses_a_file_over_1_mb() {
            let d = dir();
            let f = file(&d, "big.properties", vec![b'a'; 1024 * 1024 + 1]);
            assert_eq!(
                read_properties_file(&f),
                ImportResult::Error("big.properties is too large to import".to_string())
            );
        }

        #[test]
        fn says_why_a_file_cannot_be_read() {
            let d = dir();
            let f = d.path().join("kk-props-missing").join("gone.properties");
            let ImportResult::Error(message) = read_properties_file(&f) else {
                panic!("expected an error");
            };
            assert!(message.starts_with("gone.properties could not be read: "), "{message}");
        }

        #[test]
        fn says_a_file_has_no_properties() {
            let d = dir();
            let f = file(&d, "notes.properties", "# only a comment\n\n");
            assert_eq!(
                read_properties_file(&f),
                ImportResult::Error("notes.properties has no properties".to_string())
            );
        }

        #[test]
        fn refuses_a_malformed_escape_without_a_value_in_the_message() {
            let d = dir();
            let f = file(&d, "s.properties", "secret=hunter2\\u00G1");
            assert_eq!(
                read_properties_file(&f),
                ImportResult::Error("s.properties could not be imported: bad \\uXXXX escape on line 1".to_string())
            );
        }
    }
}
