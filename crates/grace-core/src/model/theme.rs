//! `src/shared/theme.ts`: the Appearance setting (docs/specs/2026-10-29-premium-ui.md R1.8, R8).
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The `ground` colour of `src/index.css` in each theme: the window's background (and the startup screen's, in
/// Electron Grace's `index.html`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ground {
    pub light: &'static str,
    pub dark: &'static str,
}

pub const GROUND: Ground = Ground {
    light: "#f3efe6",
    dark: "#151821",
};

/// Follow the system, or force one theme. Written to the settings file as `"system"`, `"light"` or `"dark"`.
///
/// Differs from the TypeScript's string union in being an enum, written as the same text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

pub const THEMES: [Theme; 3] = [Theme::System, Theme::Light, Theme::Dark];

impl Theme {
    /// The name the settings file and the window use.
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
}

/// `x` when it is a theme, else `Theme::System`.
pub fn parse_theme(x: &Value) -> Theme {
    THEMES
        .into_iter()
        .find(|t| x.as_str() == Some(t.as_str()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    mod parse_theme {
        use super::*;

        #[test]
        fn keeps_the_three_themes() {
            for t in THEMES {
                assert_eq!(parse_theme(&json!(t.as_str())), t);
            }
            assert_eq!(parse_theme(&json!("dark")), Theme::Dark);
        }

        #[test]
        fn reads_anything_else_as_system() {
            // `undefined` has no JSON value: it is `null` here
            for x in [
                json!("purple"),
                json!(""),
                Value::Null,
                Value::Null,
                json!(1),
                json!({}),
                json!(["dark"]),
            ] {
                assert_eq!(parse_theme(&x), Theme::System, "{x}");
            }
        }

        /// Not in the TypeScript: the settings file's text is the enum's, both ways.
        #[test]
        fn serialises_as_the_settings_file_does() {
            for t in THEMES {
                assert_eq!(serde_json::to_value(t).unwrap(), json!(t.as_str()));
                assert_eq!(serde_json::from_value::<Theme>(json!(t.as_str())).unwrap(), t);
            }
        }
    }

    mod ground {
        use super::*;

        fn repo() -> PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
        }

        /// The text between the braces after the first match of `header` (nested braces balanced).
        fn block(css: &str, header: &str) -> String {
            let m = Regex::new(header)
                .unwrap()
                .find(css)
                .unwrap_or_else(|| panic!("no block matches {header}"));
            let rest = &css[m.end()..];
            let mut depth = 1;
            let mut end = rest.len();
            for (i, c) in rest.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            rest[..end].to_string()
        }

        /// The `--color-<name>: #rrggbb;` declarations of a block, by name, in lower case.
        fn colors(css: &str) -> std::collections::HashMap<String, String> {
            let re = Regex::new(r"(?i)--color-([a-z-]+):\s*(#[0-9a-f]{6})\s*;").unwrap();
            re.captures_iter(css)
                .map(|m| (m[1].to_string(), m[2].to_lowercase()))
                .collect()
        }

        /// The light tokens of `src/index.css` (`@theme`) and the dark ones (the light with the dark media block's
        /// overrides on top), as `test/cssTokens.ts` reads them.
        fn tokens() -> (
            std::collections::HashMap<String, String>,
            std::collections::HashMap<String, String>,
        ) {
            let css = std::fs::read_to_string(repo().join("src/index.css")).unwrap();
            let css = Regex::new(r"/\*[\s\S]*?\*/")
                .unwrap()
                .replace_all(&css, "")
                .into_owned();
            let light = colors(&block(&css, r"@theme\s*\{"));
            let overrides = colors(&block(&css, r"@media\s*\(prefers-color-scheme:\s*dark\)\s*\{"));
            let mut dark = light.clone();
            dark.extend(overrides);
            (light, dark)
        }

        #[test]
        fn equals_the_ground_tokens_of_src_index_css() {
            let (light, dark) = tokens();
            assert_eq!(
                (GROUND.light, GROUND.dark),
                (light["ground"].as_str(), dark["ground"].as_str())
            );
        }

        #[test]
        fn is_what_index_html_s_startup_screen_draws_in_the_ink_of_src_index_css() {
            let (light, dark) = tokens();
            let html = std::fs::read_to_string(repo().join("index.html"))
                .unwrap()
                .to_lowercase();
            for hex in [&light["ground"], &dark["ground"], &light["ink"], &dark["ink"]] {
                assert!(html.contains(hex.as_str()), "{hex}");
            }
        }
    }
}
