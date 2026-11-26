//! What the Variables tab says about a paused scenario's variables (`Variables.tsx`, step debugger R7.4): their order,
//! each one's kind, its one line, how much Java cut and the whole value it opens into. The values come masked from the
//! runner; nothing here does more with them than shape them to be shown.
use crate::model::events::{DebugVar, VarKind};
use crate::model::js::locale_compare;

/// The characters of a value its row shows: more than a window is wide, and about a screenful in its tooltip.
pub const LINE_LIMIT: usize = 2000;

/// The variables by name, as `localeCompare` orders them.
pub fn sorted(vars: &[DebugVar]) -> Vec<&DebugVar> {
    let mut sorted: Vec<&DebugVar> = vars.iter().collect();
    sorted.sort_by(|a, b| locale_compare(&a.name, &b.name));
    sorted
}

/// The Java class's simple name of a value of another kind, else its kind.
pub fn type_label(v: &DebugVar) -> &str {
    v.type_name.as_deref().unwrap_or(match v.kind {
        VarKind::String => "string",
        VarKind::Number => "number",
        VarKind::Boolean => "boolean",
        VarKind::Null => "null",
        VarKind::Json => "json",
        VarKind::Xml => "xml",
        VarKind::Function => "function",
        VarKind::Other => "other",
    })
}

/// Whether a value opens into a box with the whole of it.
pub fn opens(kind: VarKind) -> bool {
    matches!(kind, VarKind::Json | VarKind::Xml)
}

/// The whole value: JSON indented by two spaces, in its own key order; a value Java cut short is no longer JSON, so it
/// shows as sent, as does one `serde_json` refuses (nested more than 127 deep).
pub fn full_value(v: &DebugVar) -> String {
    if v.kind == VarKind::Json
        && v.cut.is_none_or(|c| c == 0)
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&v.value)
        && let Ok(indented) = serde_json::to_string_pretty(&json)
    {
        return indented;
    }
    v.value.clone()
}

/// A value as one line, each control character and line separator a space, cut after `LINE_LIMIT` characters: GPUI
/// shapes one line, which a newline would stop, and shapes the whole of it on each paint.
pub fn value_line(value: &str) -> String {
    let mut chars = value.chars();
    let mut line: String = chars
        .by_ref()
        .take(LINE_LIMIT)
        .map(|c| {
            if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') {
                ' '
            } else {
                c
            }
        })
        .collect();
    if chars.next().is_some() {
        line.push('…');
    }
    line
}

/// "36,864 more characters": what Java cut, its thousands apart as in `en-US`.
pub fn more_characters(n: u32) -> String {
    let digits = n.to_string();
    let mut grouped = String::new();
    for (i, d) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(d);
    }
    format!("{grouped} more characters")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, kind: VarKind, value: &str) -> DebugVar {
        DebugVar {
            name: name.into(),
            kind,
            type_name: None,
            value: value.into(),
            cut: None,
        }
    }

    #[test]
    fn sorts_by_name_as_locale_compare_does() {
        let vars: Vec<DebugVar> = ["user", "Zed", "big", "_x", "doc"]
            .map(|n| var(n, VarKind::Null, "null"))
            .into();
        let names: Vec<&str> = sorted(&vars).into_iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["_x", "big", "doc", "user", "Zed"]);
    }

    #[test]
    fn labels_a_variable_with_its_java_type_else_its_kind_and_opens_only_json_and_xml() {
        use VarKind::*;
        let kinds = [String, Number, Boolean, Null, Json, Xml, Function, Other];
        let vars = kinds.map(|k| var("a", k, ""));
        assert_eq!(
            vars.each_ref().map(type_label),
            [
                "string", "number", "boolean", "null", "json", "xml", "function", "other"
            ]
        );
        assert_eq!(kinds.map(opens), [false, false, false, false, true, true, false, false]);
        let day = DebugVar {
            type_name: Some("LocalDate".into()),
            ..var("day", Other, "2026-11-02")
        };
        assert_eq!(type_label(&day), "LocalDate");
    }

    #[test]
    fn indents_json_in_its_own_key_order_and_shows_xml_a_cut_value_and_what_is_not_json_as_sent() {
        let json = var("u", VarKind::Json, r#"{"z":7,"a":["b"],"none":{},"s":"é\n"}"#);
        assert_eq!(
            full_value(&json),
            "{\n  \"z\": 7,\n  \"a\": [\n    \"b\"\n  ],\n  \"none\": {},\n  \"s\": \"é\\n\"\n}"
        );
        let xml = var("x", VarKind::Xml, "<root><item>x</item></root>");
        assert_eq!(full_value(&xml), "<root><item>x</item></root>");
        let cut = DebugVar {
            cut: Some(4),
            ..var("c", VarKind::Json, r#"{"a":1}"#)
        };
        assert_eq!(full_value(&cut), r#"{"a":1}"#, "a value Java cut is shown as sent");
        assert_eq!(full_value(&var("b", VarKind::Json, "[1,")), "[1,");
    }

    #[test]
    fn makes_one_line_of_a_value_and_cuts_a_long_one_between_characters() {
        assert_eq!(value_line("a\nb\r\n\tc\u{2028}d\u{2029}e\u{0}f"), "a b   c d e f");
        let whole = "é".repeat(LINE_LIMIT);
        assert_eq!(value_line(&whole), whole);
        let long = "😀".repeat(LINE_LIMIT + 1);
        assert_eq!(value_line(&long), format!("{}…", "😀".repeat(LINE_LIMIT)));
    }

    #[test]
    fn says_how_much_was_cut_with_thousands_apart() {
        assert_eq!(more_characters(1), "1 more characters");
        assert_eq!(more_characters(999), "999 more characters");
        assert_eq!(more_characters(36_864), "36,864 more characters");
        assert_eq!(more_characters(1_000_000), "1,000,000 more characters");
    }
}
