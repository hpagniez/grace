//! `cargo xtask audit`: `npm audit --audit-level=high`, less the advisories listed here. npm has no way to let one
//! advisory pass, and one without a patched release would otherwise stop every PR (`checks` is required).
//!
//! It fails closed: a run of npm that did not end well passes only when its report was read and every high or critical
//! advisory in it is listed. A report that cannot be read, or that names no such advisory, fails.

/// The advisories let through, each with why and since when. Remove an entry once its fix is in `package-lock.json`:
/// the check says so when npm no longer reports it.
const ALLOWED: &[(&str, &str)] = &[(
    "GHSA-fc5q-39xc-vjpf",
    "picomatch <= 4.0.7 (ReDoS), no patched release on 2026-11-18; in build tools only: `npm audit --omit=dev` is clean",
)];

/// A JSON value, as far as the report needs it: `Other` is a number, true, false or null.
#[derive(Debug, PartialEq)]
enum Json {
    Other,
    Text(String),
    List(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn text(&self) -> Option<&str> {
        match self {
            Json::Text(t) => Some(t),
            _ => None,
        }
    }
}

/// Reads one JSON document; None when `text` is not one.
fn parse(text: &str) -> Option<Json> {
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;
    let value = value(&chars, &mut at)?;
    space(&chars, &mut at);
    (at == chars.len()).then_some(value)
}

fn space(c: &[char], at: &mut usize) {
    while c.get(*at).is_some_and(|ch| ch.is_whitespace()) {
        *at += 1;
    }
}

fn value(c: &[char], at: &mut usize) -> Option<Json> {
    space(c, at);
    match c.get(*at)? {
        '{' => {
            *at += 1;
            let mut fields = Vec::new();
            loop {
                space(c, at);
                if c.get(*at) == Some(&'}') {
                    *at += 1;
                    return Some(Json::Object(fields));
                }
                if !fields.is_empty() {
                    (c.get(*at) == Some(&',')).then_some(())?;
                    *at += 1;
                    space(c, at);
                }
                let key = string(c, at)?;
                space(c, at);
                (c.get(*at) == Some(&':')).then_some(())?;
                *at += 1;
                fields.push((key, value(c, at)?));
            }
        }
        '[' => {
            *at += 1;
            let mut items = Vec::new();
            loop {
                space(c, at);
                if c.get(*at) == Some(&']') {
                    *at += 1;
                    return Some(Json::List(items));
                }
                if !items.is_empty() {
                    (c.get(*at) == Some(&',')).then_some(())?;
                    *at += 1;
                }
                items.push(value(c, at)?);
            }
        }
        '"' => string(c, at).map(Json::Text),
        _ => {
            let from = *at;
            while c
                .get(*at)
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || "+-.".contains(*ch))
            {
                *at += 1;
            }
            let word: String = c[from..*at].iter().collect();
            let number = word.starts_with(|ch: char| ch == '-' || ch.is_ascii_digit()) && word.parse::<f64>().is_ok();
            (number || matches!(word.as_str(), "true" | "false" | "null")).then_some(Json::Other)
        }
    }
}

fn string(c: &[char], at: &mut usize) -> Option<String> {
    (c.get(*at) == Some(&'"')).then_some(())?;
    *at += 1;
    let mut out = String::new();
    loop {
        let ch = *c.get(*at)?;
        *at += 1;
        match ch {
            '"' => return Some(out),
            '\\' => {
                let escaped = *c.get(*at)?;
                *at += 1;
                out.push(match escaped {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    'u' => {
                        let hex: String = c.get(*at..*at + 4)?.iter().collect();
                        *at += 4;
                        // half of a pair has no character of its own: nothing the check reads holds one
                        char::from_u32(u32::from_str_radix(&hex, 16).ok()?).unwrap_or('\u{fffd}')
                    }
                    other => other,
                });
            }
            other => out.push(other),
        }
    }
}

/// An advisory of the report: its id (the end of its address), severity, package and title.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Advisory {
    id: String,
    severity: String,
    package: String,
    title: String,
}

/// The advisories of `npm audit --json`'s report, each once. A package's `via` holds the advisories against it and
/// the names of the packages it inherits one from; only the first count. None when the report is not one.
fn advisories(report: &Json) -> Option<Vec<Advisory>> {
    let Json::Object(packages) = report.get("vulnerabilities")? else {
        return None;
    };
    let mut found = Vec::new();
    for (_, package) in packages {
        let Some(Json::List(via)) = package.get("via") else {
            return None;
        };
        for advisory in via.iter().filter(|v| matches!(v, Json::Object(_))) {
            let field = |name| advisory.get(name).and_then(Json::text).map(str::to_string);
            let url = field("url")?;
            found.push(Advisory {
                id: url.rsplit('/').next().unwrap_or_default().to_string(),
                severity: field("severity")?,
                package: field("name")?,
                title: field("title").unwrap_or_default(),
            });
        }
    }
    found.sort();
    found.dedup();
    Some(found)
}

/// What the check says: the lines to print, and whether it passes. `ok` is whether npm ended well, which it does when
/// it has nothing at level high; `report` is what it printed.
fn judge(ok: bool, report: &str, allowed: &[(&str, &str)]) -> (Vec<String>, bool) {
    let Some(found) = parse(report).as_ref().and_then(advisories) else {
        return (
            vec![format!(
                "npm audit's report could not be read:\n{}",
                report.chars().take(2000).collect::<String>()
            )],
            false,
        );
    };
    let serious = |a: &&Advisory| a.severity == "high" || a.severity == "critical";
    let reason = |a: &Advisory| allowed.iter().find(|(id, _)| *id == a.id).map(|(_, why)| *why);
    let mut lines = Vec::new();
    for a in found.iter().filter(serious) {
        match reason(a) {
            Some(why) => lines.push(format!("let through: {} ({}): {why}", a.id, a.package)),
            None => lines.push(format!(
                "{} ({}, {}): {}\n  https://github.com/advisories/{}",
                a.id, a.package, a.severity, a.title, a.id
            )),
        }
    }
    for (id, _) in allowed {
        if !found.iter().any(|a| a.id == *id) {
            // GitHub shows it on the run; it must not fail the PR that brings the fix
            lines.push(format!(
                "::warning::npm audit no longer reports {id}: remove it from crates/xtask/src/audit.rs"
            ));
        }
    }
    let refused = found.iter().filter(serious).any(|a| reason(a).is_none());
    if !ok && !refused && !found.iter().any(|a| serious(&a)) {
        lines.push("npm audit failed, and its report names no high or critical advisory".to_string());
        return (lines, false);
    }
    (lines, !refused)
}

pub fn run() -> u8 {
    // npm is a script on Windows, which `Command` starts only by its full name
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let out = match std::process::Command::new(npm)
        .args(["audit", "--audit-level=high", "--json"])
        .output()
    {
        Ok(out) => out,
        Err(e) => {
            eprintln!("npm audit could not be started: {e}");
            return 1;
        }
    };
    let (lines, passes) = judge(out.status.success(), &String::from_utf8_lossy(&out.stdout), ALLOWED);
    for line in lines {
        println!("{line}");
    }
    if !passes {
        eprint!("{}", String::from_utf8_lossy(&out.stderr));
    }
    u8::from(!passes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALLOW: &[(&str, &str)] = &[("GHSA-aaaa-bbbb-cccc", "no fix yet")];

    fn advisory(id: &str, package: &str, severity: &str) -> String {
        format!(
            r#"{{ "source": 1, "name": "{package}", "title": "a \"quoted\" title", "url": "https://github.com/advisories/{id}", "severity": "{severity}", "cvss": {{ "score": 7.5 }}, "range": "<=4.0.7" }}"#
        )
    }

    /// A report as npm prints it: each package with its severity and its `via`, an advisory or a package's name.
    fn report(packages: &[(&str, &[String])]) -> String {
        let packages: Vec<String> = packages
            .iter()
            .map(|(name, via)| {
                format!(
                    "\"{name}\": {{\n \"name\": \"{name}\", \"severity\": \"high\", \"isDirect\": false,\n \"via\": [{}],\n \"effects\": [], \"fixAvailable\": true }}",
                    via.join(", ")
                )
            })
            .collect();
        format!(
            "{{\n \"auditReportVersion\": 2,\n \"vulnerabilities\": {{ {} }},\n \"metadata\": {{ \"vulnerabilities\": {{ \"high\": 2, \"total\": 2 }} }}\n}}\n",
            packages.join(", ")
        )
    }

    /// A `via` that is a package's name: the advisory is inherited from it.
    const INHERITED: &str = "\"picomatch\"";

    #[test]
    fn reads_json() {
        // '@' stands for the backslash of a unicode escape
        let text = r#" { "a": [-2.5e3, true, null], "b": "x\n\"@u00e9\\", "c": {} } "#;
        assert_eq!(
            parse(&text.replace('@', &char::from(92u8).to_string())),
            Some(Json::Object(vec![
                ("a".into(), Json::List(vec![Json::Other, Json::Other, Json::Other])),
                ("b".into(), Json::Text("x\n\"é\\".into())),
                ("c".into(), Json::Object(vec![])),
            ]))
        );
        for bad in ["", "{", "{\"a\" 1}", "[1 2]", "{} x", "\"open", "nul", "NaN", "1x"] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn nothing_at_level_high_passes() {
        let (lines, passes) = judge(true, &report(&[]), &[]);
        assert!(passes && lines.is_empty(), "{lines:?}");
        // an advisory below high does not fail npm, nor the check
        let low = report(&[("glob", &[advisory("GHSA-low0-low0-low0", "glob", "moderate")])]);
        assert_eq!(judge(true, &low, &[]), (vec![], true));
    }

    #[test]
    fn the_listed_advisory_alone_passes_with_its_reason() {
        let text = report(&[
            ("picomatch", &[advisory("GHSA-aaaa-bbbb-cccc", "picomatch", "high")]),
            ("vite", &[INHERITED.to_string()]),
        ]);
        let (lines, passes) = judge(false, &text, ALLOW);
        assert!(passes, "{lines:?}");
        assert_eq!(lines, ["let through: GHSA-aaaa-bbbb-cccc (picomatch): no fix yet"]);
    }

    #[test]
    fn another_high_advisory_on_the_same_package_fails() {
        let text = report(&[(
            "picomatch",
            &[
                advisory("GHSA-aaaa-bbbb-cccc", "picomatch", "high"),
                advisory("GHSA-zzzz-zzzz-zzzz", "picomatch", "high"),
            ],
        )]);
        let (lines, passes) = judge(false, &text, ALLOW);
        assert!(!passes);
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("GHSA-zzzz-zzzz-zzzz (picomatch, high)")),
            "{lines:?}"
        );
    }

    #[test]
    fn a_high_advisory_on_a_package_that_also_inherits_the_listed_one_fails() {
        let vite = advisory("GHSA-vite-vite-vite", "vite", "critical");
        let text = report(&[
            ("picomatch", &[advisory("GHSA-aaaa-bbbb-cccc", "picomatch", "high")]),
            ("vite", &[INHERITED.to_string(), vite]),
        ]);
        let (lines, passes) = judge(false, &text, ALLOW);
        assert!(!passes);
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("GHSA-vite-vite-vite (vite, critical)")),
            "{lines:?}"
        );
    }

    #[test]
    fn a_report_that_cannot_be_read_fails() {
        for text in [
            "",
            "npm error code ENOLOCK",
            r#"{ "error": { "code": "ENOLOCK", "summary": "no lockfile" } }"#,
        ] {
            for ok in [true, false] {
                let (lines, passes) = judge(ok, text, ALLOW);
                assert!(!passes, "{text}");
                assert!(
                    lines[0].starts_with("npm audit's report could not be read"),
                    "{lines:?}"
                );
            }
        }
        // an advisory without its address or severity is not one the check can judge
        let odd = report(&[("x", &[r#"{ "name": "x", "title": "t" }"#.to_string()])]);
        assert!(!judge(false, &odd, ALLOW).1);
    }

    #[test]
    fn a_failed_npm_whose_report_names_no_high_advisory_fails() {
        let text = report(&[("glob", &[advisory("GHSA-low0-low0-low0", "glob", "moderate")])]);
        let (lines, passes) = judge(false, &text, &[]);
        assert!(!passes);
        assert_eq!(
            lines,
            ["npm audit failed, and its report names no high or critical advisory"]
        );
        assert!(!judge(false, &report(&[]), &[]).1);
    }

    #[test]
    fn a_listed_advisory_npm_no_longer_reports_is_said_and_does_not_fail() {
        let (lines, passes) = judge(true, &report(&[]), ALLOW);
        assert!(passes);
        assert_eq!(
            lines,
            ["::warning::npm audit no longer reports GHSA-aaaa-bbbb-cccc: remove it from crates/xtask/src/audit.rs"]
        );
    }
}
