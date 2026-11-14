//! `seedRuns` and `projectDataDir` of `e2e/helpers.ts`: run folders written into a data folder before Grace opens its
//! project, so a test starts with a history it did not have to run.
use std::path::{Path, PathBuf};

use chrono::{Duration, Local};
use grace_core::history::create_run_dir;
use serde_json::{Value, json};

/// `<data folder>/projects/<hash>`: where Grace keeps the runs of the project at `root`.
pub fn project_data_dir(data: &Path, root: &Path) -> PathBuf {
    grace_core::runner::project_dir(data, &root.to_string_lossy())
}

/// The run folders of the project at `root`.
pub fn runs_dir(data: &Path, root: &Path) -> PathBuf {
    project_data_dir(data, root).join("runs")
}

/// What a seeded run holds beside the clean sample project's "Run all".
#[derive(Default)]
pub struct Seed {
    /// A `karate-summary.html` in each run folder: its Report can open.
    pub report: bool,
    /// Fields that replace those of every record (`record` of `seedRuns`), as an object.
    pub record: Value,
}

/// Writes `n` valid run folders, one minute apart and all in the past (the last is the newest), into the history of
/// the project at `root`: each is "Run all" on the clean sample project that finished with "failing match" failed.
/// Returns their ids, oldest first.
pub fn seed_runs(data: &Path, root: &Path, n: usize, seed: &Seed) -> Vec<String> {
    let project = project_data_dir(data, root);
    let now = Local::now();
    (0..n)
        .map(|i| {
            let started = now - Duration::minutes((n - i) as i64);
            let run = create_run_dir(&project, &started).expect("a run folder");
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut record = json!({
                "version": 1,
                "startedAt": started.timestamp_millis(),
                "durationMillis": 5000,
                "env": null,
                "threads": 1,
                "fingerprint": null,
                "filter": { "include": [], "exclude": [], "hideNonMatching": false },
                "selection": { "kind": "folder", "file": "", "name": name, "line": null, "key": "d:" },
                "failedOf": null,
                "outcome": "finished",
                "exitCode": 1,
                "tests": [
                    { "kind": "scenario", "file": "tests/basic.feature", "refId": "[1:7]", "name": "passing hello",
                      "line": 7, "key": "s:tests/basic.feature#1", "status": "passed", "durationMillis": 400 },
                    { "kind": "scenario", "file": "tests/basic.feature", "refId": "[1:15]", "name": "failing match",
                      "line": 15, "key": "s:tests/basic.feature#2", "status": "failed", "durationMillis": 300 },
                ],
            });
            if let (Some(own), Some(over)) = (record.as_object_mut(), seed.record.as_object()) {
                own.extend(over.clone());
            }
            std::fs::write(run.dir.join("run.json"), record.to_string()).expect("run.json");
            if seed.report {
                std::fs::write(run.dir.join("karate-summary.html"), "<html></html>").expect("a report");
            }
            run.id
        })
        .collect()
}
