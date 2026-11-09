//! `src/shared/events.ts`: what a run reports. Electron Grace sent these over IPC as `AppEvent`s; GPUI Grace has no
//! IPC (migration guide, Architecture), and the runner reports the same events to the `Grace` entity instead.
use super::env::EnvView;
use super::flaky::FlakyTest;
use super::history::HistoryEntry;
use super::http::HttpCall;
use super::node::{Layout, Status, TestNode};
use super::tags::FilterState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// Gherkin prefix as written: Given, When, Then, And, or `*`.
    pub keyword: String,
    /// Karate keyword and its argument, e.g. `match response.message == 'x'`.
    pub text: String,
    pub line: u32,
    /// Karate's `result.status` as-is.
    pub status: String,
    #[serde(with = "crate::model::js_num")]
    pub millis: f64,
    pub background: bool,
    pub log: Option<String>,
    /// The `#` lines Karate attached to the step, trimmed; absent when it has none (step comments R1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments: Option<Vec<String>>,
}

/// A step of a test as its feature file has it, before any run (step debugger R4.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStep {
    pub keyword: String,
    pub text: String,
    pub line: u32,
    pub background: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments: Option<Vec<String>>,
}

/// A step debugger breakpoint: the tree's `file` (Karate's feature path) and a step line.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Breakpoint {
    pub file: String,
    pub line: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DebugCommand {
    Continue,
    Over,
    Into,
    Pause,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VarKind {
    String,
    Number,
    Boolean,
    Null,
    Json,
    Xml,
    Function,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugVar {
    pub name: String,
    pub kind: VarKind,
    /// The Java class's simple name, for `other` only.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    pub value: String,
    /// Characters left out after the first 65,536.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut: Option<u32>,
}

/// Where a debug run waits: the step it has not run yet, and the scenario's variables.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paused {
    /// The paused test's key in the tree; None if the tree has no such test.
    pub key: Option<String>,
    pub file: String,
    pub line: u32,
    /// 0 for the test's own steps, 1 or more inside a called feature.
    pub depth: u32,
    pub prefix: String,
    pub keyword: String,
    pub text: String,
    pub vars: Vec<DebugVar>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioUpdate {
    pub key: String,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "crate::model::js_num_opt")]
    pub duration_millis: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Where Karate's reason starts in `error`, in UTF-16 units as Electron Grace counted (#229 R2.1); None: the whole
    /// error is the reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_at: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub scenario_count: u32,
    pub scenarios_passed: u32,
    pub scenarios_failed: u32,
    #[serde(with = "crate::model::js_num")]
    pub duration_millis: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunOutcome {
    Finished,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub root: String,
    pub layout: Layout,
}

/// A saved recent folder and whether it is still a folder on disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentEntry {
    pub path: String,
    pub exists: bool,
}

/// The build tool Grace runs to list a project's libraries or to build its Java code (`listing`). Written as Electron
/// Grace's `"Maven" | "Gradle"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildTool {
    Maven,
    Gradle,
}

/// What the runner reports to the window, as Electron Grace's `AppEvent` union. Written the same as its JSON
/// (`{"type":"runFinished",...}`, camelCase fields), so a recorded event reads back unchanged.
// `build?: true` of the union is a `bool` that is written only when true; `success?: boolean` stays an `Option`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AppEvent {
    Project {
        project: Option<Project>,
        tree: Option<TestNode>,
        error: Option<String>,
        /// Folders below the project that look like a Karate project, relative with '/'; only on a tree with no test
        /// (docs/specs/2026-11-04-subfolder-offer.md).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subfolders: Option<Vec<String>>,
    },
    RunStarted {
        selection: String,
        /// The run's folder name under `runs/`.
        run_id: String,
        keys: Vec<String>,
        total: u32,
        #[serde(with = "crate::model::js_num")]
        started_at: f64,
        env: Option<String>,
        /// A step debugger run: one thread, with the listener.
        debug: bool,
    },
    Scenarios {
        updates: Vec<ScenarioUpdate>,
    },
    /// The open project's breakpoints, as the runner holds them.
    Breakpoints {
        breakpoints: Vec<Breakpoint>,
    },
    DebugPaused {
        paused: Paused,
    },
    DebugResumed,
    Steps {
        key: String,
        steps: Vec<Step>,
    },
    Calls {
        key: String,
        calls: Vec<HttpCall>,
    },
    Console {
        lines: Vec<String>,
    },
    RunFinished {
        outcome: RunOutcome,
        exit_code: Option<i32>,
        summary: Option<Summary>,
        report_available: bool,
    },
    /// `build`: the notice offers the Build button (docs/specs/2026-11-04-build-from-notice.md R1).
    Notice {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        success: Option<bool>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        build: bool,
    },
    /// The open project's tag filter, as the runner holds and applies it.
    Filter {
        filter: FilterState,
    },
    /// The open project's thread count, as the runner holds it and passes to every run.
    Threads {
        count: u32,
    },
    /// Whether watch mode is on for the open project (docs/specs/2026-11-03-watch-mode.md).
    Watch {
        on: bool,
    },
    /// The open project's environment, suggestions and run properties, as the runner holds them.
    Environment {
        view: EnvView,
    },
    /// The open project's listed runs, newest first.
    History {
        runs: Vec<HistoryEntry>,
    },
    /// The flaky tests of the tree last sent, by their key in it (F15). Follows every `project` and every `history`.
    Flaky {
        tests: Vec<FlakyTest>,
    },
    /// The runner is discovering or running; only Cancel makes sense meanwhile.
    Busy {
        busy: bool,
    },
    /// The build tool Grace is running to list the project's libraries, or to build its Java code when `build`;
    /// `None` when it is done.
    Listing {
        tool: Option<BuildTool>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        build: bool,
    },
    /// The folder being opened, or `None`. Sent around each open and in every replay.
    Opening {
        root: Option<String>,
    },
    /// The saved recent folders, newest first, each checked for existence.
    Recent {
        entries: Vec<RecentEntry>,
    },
    // `updates`, `updatePrompt` and `updateProgress` carry `UpdateView`, `Release` and `UpdateProgress` of
    // `src/shared/updates.ts`, which `model::updates` ports in a parallel change: `serde_json::Value` until it lands,
    // then these three fields take its types.
    /// What the runner holds about updates: settings, the last check and the saved copies of the data.
    Updates {
        view: serde_json::Value,
    },
    /// The startup modal's release (R4.1).
    UpdatePrompt {
        release: serde_json::Value,
    },
    /// The install in progress or its failure; `None` when there is none to show (R8).
    UpdateProgress {
        progress: Option<serde_json::Value>,
    },
}

impl AppEvent {
    /// A plain notice: neither a success nor an offer to build.
    pub fn notice(message: impl Into<String>) -> Self {
        AppEvent::Notice {
            message: message.into(),
            success: None,
            build: false,
        }
    }
}

#[cfg(test)]
mod tests {
    // New in the port (`src/shared/events.ts` has no test): the union keeps its JSON form.
    use super::*;

    fn json(e: &AppEvent) -> String {
        serde_json::to_string(e).unwrap()
    }

    #[test]
    fn a_run_finished_event_is_written_as_the_union_has_it() {
        let e = AppEvent::RunFinished {
            outcome: RunOutcome::Finished,
            exit_code: Some(1),
            summary: None,
            report_available: true,
        };
        assert_eq!(
            json(&e),
            r#"{"type":"runFinished","outcome":"finished","exitCode":1,"summary":null,"reportAvailable":true}"#
        );
        assert_eq!(serde_json::from_str::<AppEvent>(&json(&e)).unwrap(), e);
    }

    #[test]
    fn a_notice_leaves_out_what_it_does_not_set() {
        assert_eq!(json(&AppEvent::notice("hi")), r#"{"type":"notice","message":"hi"}"#);
        let build = AppEvent::Notice {
            message: "x".into(),
            success: Some(false),
            build: true,
        };
        assert_eq!(
            json(&build),
            r#"{"type":"notice","message":"x","success":false,"build":true}"#
        );
        assert_eq!(serde_json::from_str::<AppEvent>(&json(&build)).unwrap(), build);
    }

    #[test]
    fn camel_case_variants_and_fields() {
        let e = AppEvent::RunStarted {
            selection: "d:".into(),
            run_id: "r1".into(),
            keys: vec!["s:a#1".into()],
            total: 1,
            started_at: 1_700_000_000_000.0,
            env: None,
            debug: false,
        };
        assert_eq!(
            json(&e),
            r#"{"type":"runStarted","selection":"d:","runId":"r1","keys":["s:a#1"],"total":1,"startedAt":1700000000000,"env":null,"debug":false}"#
        );
        assert_eq!(json(&AppEvent::DebugResumed), r#"{"type":"debugResumed"}"#);
        assert_eq!(
            json(&AppEvent::Listing {
                tool: Some(BuildTool::Maven),
                build: false
            }),
            r#"{"type":"listing","tool":"Maven"}"#
        );
    }
}
