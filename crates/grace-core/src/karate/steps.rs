//! `src/main/karate/steps.ts`: the steps of a test, from Karate's `FEATURE_EXIT` (what ran) and from the feature file
//! (what is written).
use super::events::DataError;
use super::js;
use crate::model::events::{FileStep, Step};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

/// The steps of one scenario of a feature, by Karate refId.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioSteps {
    pub ref_id: String,
    pub steps: Vec<Step>,
}

/// Steps of every scenario in a depth-0 `FEATURE_EXIT`'s data, by Karate refId. Steps of called features are not
/// expanded. Data that is not shaped as Karate writes it is an error, as it was a thrown `TypeError`.
// A `refId` that is not a string reads as "": no test has one, so its steps match nothing, as `undefined` did.
pub fn extract_steps(feature_exit_data: &Value) -> Result<Vec<ScenarioSteps>, DataError> {
    let mut scenarios = Vec::new();
    for sc in js::items(js::prop(feature_exit_data, "scenarioResults")?, "scenarioResults")? {
        let ref_id = js::text(js::prop(sc, "refId")?).unwrap_or_default().to_owned();
        let mut steps = Vec::new();
        for r in js::items(js::prop(sc, "stepResults")?, "stepResults")? {
            let step = js::prop(r, "step")?;
            let result = js::prop(r, "result")?;
            let keyword = js::prop(step, "keyword")?;
            let text = js::prop(step, "text")?;
            steps.push(Step {
                keyword: js::text(js::prop(step, "prefix")?).unwrap_or_default().to_owned(),
                // Karate's synthetic step (a failed config) has no keyword (#229 R1.3)
                text: if js::truthy(keyword) {
                    format!("{} {}", js::display(keyword), js::display(text))
                } else {
                    js::text(text).unwrap_or_default().to_owned()
                },
                line: js::as_u32(js::prop(step, "line")?).unwrap_or(0),
                status: js::text(js::prop(result, "status")?).unwrap_or_default().to_owned(),
                millis: js::prop(result, "millis")?.as_f64().unwrap_or(0.0),
                background: js::truthy(js::prop(step, "background")?),
                log: js::text(js::prop(r, "stepLog")?).map(str::to_owned),
                comments: Some(js::prop(step, "comments")?)
                    .filter(|c| js::truthy(c))
                    .map(js::strings),
            });
        }
        scenarios.push(ScenarioSteps { ref_id, steps });
    }
    Ok(scenarios)
}

fn regex(pattern: String) -> Regex {
    Regex::new(&pattern).expect("a fixed pattern")
}

static STEP: LazyLock<Regex> = LazyLock::new(|| regex(format!(r"^(\*|Given|When|Then|And|But)[{}]", js::WS)));
static HEADER: LazyLock<Regex> = LazyLock::new(|| {
    regex(format!(
        r"^(@|(Feature|Rule|Scenario|Scenario Outline|Examples|Scenarios)[{}]*:)",
        js::WS
    ))
});
static BACKGROUND: LazyLock<Regex> = LazyLock::new(|| regex(format!(r"^Background[{}]*:", js::WS)));

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Background,
    Test,
}

/// The background's steps, then those of the section whose header is on `section_line` (a scenario or an outline),
/// as written. A step is a line whose first word is a Gherkin keyword, outside a doc string (step debugger R4.2).
/// Like Karate's lexer, a step takes the `#` lines since the last line that is neither blank nor a comment (step
/// comments R1.1).
pub fn file_steps(feature_text: &str, section_line: u32) -> Vec<FileStep> {
    let mut steps = Vec::new();
    let mut section: Option<Section> = None;
    let mut fence: Option<&str> = None;
    let mut comments: Vec<String> = Vec::new();
    for (i, raw) in js::lines(feature_text).into_iter().enumerate() {
        let t = js::trim(raw);
        if t.is_empty() {
            continue;
        }
        if fence.is_none() && t.starts_with('#') {
            comments.push(t.to_owned());
            continue;
        }
        let above = std::mem::take(&mut comments);
        if let Some(open) = fence {
            if t.starts_with(open) {
                fence = None;
            }
        } else if t.starts_with("\"\"\"") || t.starts_with("```") {
            fence = Some(&t[..3]);
        } else if BACKGROUND.is_match(t) {
            section = Some(Section::Background);
        } else if i + 1 == section_line as usize {
            section = Some(Section::Test);
        } else if HEADER.is_match(t) {
            section = None;
        } else if let Some(section) = section.filter(|_| STEP.is_match(t)) {
            let end = t.find(js::is_space).unwrap_or(t.len());
            steps.push(FileStep {
                keyword: t[..end].to_owned(),
                text: js::trim(&t[end..]).to_owned(),
                line: i as u32 + 1,
                background: section == Section::Background,
                comments: (!above.is_empty()).then_some(above),
            });
        }
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::karate::events::{EventKind, read_all_events};
    use serde_json::json;
    use std::path::Path;

    fn root(dir: &str, name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/fixtures")
            .join(dir)
            .join(name)
    }

    /// The data of the first depth-0 `FEATURE_EXIT` of `file` (of the one for `feature`, if given).
    fn exit_data(file: &str, feature: Option<&str>) -> Value {
        read_all_events(root("events", file))
            .unwrap()
            .into_iter()
            .find(|e| {
                e.kind == EventKind::FeatureExit
                    && e.at_depth_zero()
                    && feature.is_none_or(|f| e.data["relativePath"] == f)
            })
            .unwrap()
            .data
    }

    fn basic() -> Vec<ScenarioSteps> {
        extract_steps(&exit_data("run-clean.jsonl", Some("tests/basic.feature"))).unwrap()
    }

    mod extract_steps {
        use super::*;

        #[test]
        fn shows_a_step_with_no_keyword_by_its_text_alone_229_r1_3() {
            let data = exit_data("run-http-config-fail.jsonl", None);
            let scenarios = extract_steps(&data).unwrap();
            let [step, rest @ ..] = scenarios[0].steps.as_slice() else {
                panic!("no step")
            };
            assert!(rest.is_empty());
            assert_eq!(step.keyword, "*");
            assert_eq!(step.line, 0);
            assert_eq!(step.status, "failed");
            assert!(
                step.text
                    .starts_with("Scenario execution failed: Config evaluation failed: karate-config.js - js failed:"),
                "{}",
                step.text
            );
        }

        #[test]
        fn returns_one_entry_per_scenario_by_ref_id() {
            let ids: Vec<String> = basic().into_iter().map(|s| s.ref_id).collect();
            assert_eq!(ids, ["[1:7]", "[2:15]", "[3:22]", "[4:28]"]);
        }

        #[test]
        fn maps_a_failed_step() {
            let scenarios = basic();
            let step = scenarios[1].steps.iter().find(|s| s.status == "failed").unwrap();
            assert_eq!(step.keyword, "And");
            assert!(step.text.starts_with("match response.message"));
            assert_eq!(step.line, 20);
        }

        #[test]
        fn keeps_the_log_of_a_step_and_flags_background_steps() {
            let scenarios = basic();
            let steps = &scenarios[0].steps;
            let get = steps.iter().find(|s| s.text == "method get").unwrap();
            let log = get.log.as_deref().unwrap();
            assert!(log.contains("GET"));
            assert!(log.contains("200"));
            assert!(steps[0].background);
            assert!(!steps[1].background);
            assert_eq!(steps[1].log, None);
        }

        #[test]
        fn keeps_the_comments_karate_attached_to_a_step_backgrounds_too_step_comments_r1() {
            let scenarios = basic();
            let steps = &scenarios[1].steps;
            assert_eq!(
                steps.iter().find(|s| s.line == 20).unwrap().comments,
                Some(vec!["# expected greeting is wrong on purpose".to_owned()])
            );
            assert_eq!(steps.iter().find(|s| s.line == 18).unwrap().comments, None);
            let data = json!({
                "scenarioResults": [{
                    "refId": "[1:3]",
                    "stepResults": [{
                        "step": {
                            "prefix": "*", "keyword": "url", "text": "u", "line": 4,
                            "background": true, "comments": ["# base"]
                        },
                        "result": { "status": "passed", "millis": 1 }
                    }]
                }]
            });
            let scenarios = extract_steps(&data).unwrap();
            assert_eq!(scenarios[0].steps[0].comments, Some(vec!["# base".to_owned()]));
        }

        // New in the port: the shapes that were a `TypeError` are an error, not a panic or a guess.
        #[test]
        fn data_that_is_not_shaped_as_karate_writes_it_is_an_error() {
            assert!(extract_steps(&json!({"scenarioResults": 42})).is_err());
            assert!(extract_steps(&json!({"scenarioResults": [{"stepResults": [{}]}]})).is_err());
            assert_eq!(extract_steps(&json!({})).unwrap(), []);
        }
    }

    mod file_steps {
        use super::*;

        fn debug() -> String {
            std::fs::read_to_string(root("projects", "debug/debug.feature")).unwrap()
        }

        #[test]
        fn lists_the_backgrounds_steps_then_the_tests_own_as_written() {
            let text = debug();
            let steps = file_steps(&text, 6);
            assert_eq!(steps.iter().map(|s| s.line).collect::<Vec<_>>(), [4, 7, 8, 9, 10]);
            assert_eq!(
                steps[0],
                FileStep {
                    keyword: "*".into(),
                    text: "def base = 'from background'".into(),
                    line: 4,
                    background: true,
                    comments: None,
                }
            );
            assert_eq!(
                steps[1],
                FileStep {
                    keyword: "*".into(),
                    text: "def name = 'grace'".into(),
                    line: 7,
                    background: false,
                    comments: None,
                }
            );
            assert_eq!(
                file_steps(&text, 12).iter().map(|s| s.line).collect::<Vec<_>>(),
                [4, 13]
            );
        }

        #[test]
        fn skips_doc_strings_tables_and_comments_and_stops_an_outline_at_its_examples() {
            let text = [
                "Feature: f",                // 1
                "  Scenario Outline: o <n>", // 2
                "    Given def a = <n>",     // 3
                "    * def doc =",           // 4
                "      \"\"\"",              // 5
                "      * not a step",        // 6
                "      \"\"\"",              // 7
                "    # * a comment",         // 8
                "    And table t",           // 9
                "      | x |",               // 10
                "  Examples:",               // 11
                "    | n |",                 // 12
                "    | 1 |",                 // 13
                "  Scenario: next",          // 14
                "    * def b = 1",           // 15
            ]
            .join("\r\n");
            let steps: Vec<(String, u32)> = file_steps(&text, 2).into_iter().map(|s| (s.keyword, s.line)).collect();
            assert_eq!(
                steps,
                [("Given".to_owned(), 3), ("*".to_owned(), 4), ("And".to_owned(), 9)]
            );
        }

        #[test]
        fn attaches_the_comments_above_a_step_as_karate_does_step_comments_r1() {
            let text = [
                "Feature: f",                  // 1
                "  # above Background",        // 2
                "  Background:",               // 3
                "    # the base",              // 4
                "    * url 'u'",               // 5
                "  # above the tag",           // 6
                "  @t",                        // 7
                "  # above Scenario",          // 8
                "  Scenario: s",               // 9
                "    # why",                   // 10
                "",                            // 11
                "  #where from  ",             // 12
                "    * def a = 1",             // 13
                "    * def doc =",             // 14
                "    # before the doc string", // 15
                "      \"\"\"",                // 16
                "      # in the doc string",   // 17
                "      \"\"\"",                // 18
                "    * table t",               // 19
                "      | x |",                 // 20
                "      # between rows",        // 21
                "      | 1 |",                 // 22
                "    * def b = 2",             // 23
                "    # after the last step",   // 24
                "  Scenario: next",            // 25
                "    * def c = 3",             // 26
                "  # at the end",              // 27
            ]
            .join("\n");
            let pairs = |line: u32| -> Vec<(u32, Option<Vec<String>>)> {
                file_steps(&text, line)
                    .into_iter()
                    .map(|s| (s.line, s.comments))
                    .collect()
            };
            let some = |c: &[&str]| Some(c.iter().map(|s| s.to_string()).collect::<Vec<_>>());
            assert_eq!(
                pairs(9),
                [
                    (5, some(&["# the base"])),
                    (13, some(&["# why", "#where from"])),
                    (14, None),
                    (19, None),
                    (23, None),
                ]
            );
            assert_eq!(file_steps(&text, 9)[2].comments, None);
            assert_eq!(pairs(25), [(5, some(&["# the base"])), (26, None)]);
        }
    }
}
