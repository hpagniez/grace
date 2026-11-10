//! `consoleFind.ts`: what the console's find and error stepping look for (console search spec R1, R3).
//!
//! A match's `start` and `end` are byte offsets in the line, where Electron Grace's were UTF-16 offsets: GPUI
//! highlights a text run by byte range, so the console draws them as they are.

/// A match of the find text: `line` indexes the lines searched, `start` and `end` the line's bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

/// A line Karate logged at ERROR level: the test that colours it red.
pub fn is_error_line(line: &str) -> bool {
    line.contains(" ERROR ")
}

fn same(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// The end of `query` matched at byte `at` of `text`, case aside, char by char: the offsets stay on the line as
/// written even where lower-casing would change its length.
fn match_at(text: &str, at: usize, query: &str) -> Option<usize> {
    let mut chars = text[at..].char_indices();
    for q in query.chars() {
        let (_, c) = chars.next()?;
        if !same(c, q) {
            return None;
        }
    }
    Some(chars.next().map_or(text.len(), |(i, _)| at + i))
}

/// Every match of `query` as plain text, case aside, never overlapping.
pub fn find_matches<S: AsRef<str>>(lines: impl IntoIterator<Item = S>, query: &str) -> Vec<Match> {
    let mut found = Vec::new();
    if query.is_empty() {
        return found;
    }
    for (line, text) in lines.into_iter().enumerate() {
        let text = text.as_ref();
        let mut at = 0;
        while at < text.len() {
            match match_at(text, at, query) {
                Some(end) => {
                    found.push(Match { line, start: at, end });
                    at = end;
                }
                None => at += text[at..].chars().next().map_or(1, char::len_utf8),
            }
        }
    }
    found
}

/// The ERROR lines, and the failed tests of Karate's end-of-run summary: `    - <name>` between `failed features:`
/// and the next line that starts with `=`. Karate's text, not a contract: e2e runs real Karate against it.
pub fn error_lines<S: AsRef<str>>(lines: impl IntoIterator<Item = S>) -> Vec<usize> {
    let mut found = Vec::new();
    let mut summary = false;
    for (i, text) in lines.into_iter().enumerate() {
        let text = text.as_ref();
        if text == "failed features:" {
            summary = true;
        } else if text.starts_with('=') {
            summary = false;
        } else if if summary {
            text.starts_with("    - ")
        } else {
            is_error_line(text)
        } {
            found.push(i);
        }
    }
    found
}

/// Where stepping starts with nothing current: the first item at or below line `top` going down, the last above it
/// going up, each wrapping round; None for no item.
pub fn start_index(line_of: &[usize], top: usize, down: bool) -> Option<usize> {
    if line_of.is_empty() {
        return None;
    }
    let found = if down {
        line_of.iter().position(|&l| l >= top)
    } else {
        line_of.iter().rposition(|&l| l < top)
    };
    Some(found.unwrap_or(if down { 0 } else { line_of.len() - 1 }))
}

/// The next index going down or up, wrapping round. `count` is at least 1.
pub fn step_index(count: usize, current: usize, down: bool) -> usize {
    if down {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    }
}

#[cfg(test)]
mod tests;
