//! Runs a whole project the way Grace does, without a window: opens the folder, runs every test (`d:`) with the
//! environment, filter, thread count and run properties saved for it in the data folder, prints its progress and
//! summary, and exits with Karate's exit code. The run's folder, `run.json` and report, stays in the data folder where
//! Electron Grace puts it.
//!
//! ```text
//! cargo run -p grace-core --example grace-run -- <project folder> [--data <dir>] [--vendor <dir>] [--console]
//! ```
//!
//! `--data` is Grace's data folder (`GRACE_USER_DATA`, else the platform's); `--vendor` the folder of `karate.jar`
//! (the repository's `vendor/`); `--console` prints Karate's own lines too. Exits 2 when the folder cannot be opened or
//! nothing runs, 3 when its tests cannot be discovered, else Karate's code (1 when Java gave none).
use grace_core::history::remove_leftover_runs;
use grace_core::karate::argfile::remove_leftover_arg_files;
use grace_core::karate::process::kill_orphan;
use grace_core::model::Status;
use grace_core::model::events::AppEvent;
use grace_core::paths::data_dir;
use grace_core::runner::{Runner, project_dir};
use grace_core::secrets::os_secret_box;
use grace_core::vendor::vendor_paths;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Args {
    folder: String,
    data: Option<PathBuf>,
    vendor: Option<PathBuf>,
    console: bool,
}

fn args() -> Result<Args, String> {
    let mut folder = None;
    let mut data = None;
    let mut vendor = None;
    let mut console = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--data" => data = Some(PathBuf::from(it.next().ok_or("--data needs a folder")?)),
            "--vendor" => vendor = Some(PathBuf::from(it.next().ok_or("--vendor needs a folder")?)),
            "--console" => console = true,
            _ if folder.is_none() && !a.starts_with("--") => folder = Some(a),
            _ => return Err(format!("unexpected argument {a}")),
        }
    }
    Ok(Args {
        folder: folder.ok_or("give the project folder")?,
        data,
        vendor,
        console,
    })
}

/// What the run said, as the events come.
#[derive(Default)]
struct Progress {
    total: usize,
    run_id: Option<String>,
    started: Option<Instant>,
    last_line: Option<Instant>,
    done: HashMap<String, Status>,
    exit: Option<Option<i32>>,
    project_error: Option<String>,
}

impl Progress {
    fn counts(&self) -> (usize, usize) {
        let passed = self.done.values().filter(|s| **s == Status::Passed).count();
        let failed = self.done.values().filter(|s| **s == Status::Failed).count();
        (passed, failed)
    }

    fn line(&mut self, force: bool) {
        let now = Instant::now();
        if !force && self.last_line.is_some_and(|at| now - at < Duration::from_secs(1)) {
            return;
        }
        self.last_line = Some(now);
        let (passed, failed) = self.counts();
        let secs = self.started.map_or(0.0, |s| (now - s).as_secs_f64());
        println!(
            "[{secs:6.1}s] {:>5}/{} done: {passed} passed, {failed} failed",
            self.done.len(),
            self.total
        );
    }

    fn on(&mut self, e: &AppEvent, console: bool) {
        match e {
            AppEvent::Opening { root: Some(root) } => println!("Opening {root}"),
            AppEvent::Project { tree, error, .. } => match (tree, error) {
                (_, Some(error)) => {
                    println!("The tests cannot be discovered:\n{error}");
                    self.project_error = Some(error.clone());
                }
                (Some(tree), None) => println!("Discovered {} tests", grace_core::karate::tree::leaf_keys(tree).len()),
                (None, None) => {}
            },
            AppEvent::RunStarted { run_id, total, env, .. } => {
                self.total = *total as usize;
                self.run_id = Some(run_id.clone());
                self.started = Some(Instant::now());
                let env = env.as_deref().map_or_else(String::new, |e| format!(" on {e}"));
                println!("Running {total} tests{env} (run {run_id})");
            }
            AppEvent::Scenarios { updates } => {
                for u in updates {
                    if matches!(
                        u.status,
                        Status::Passed | Status::Failed | Status::NotRun | Status::Interrupted
                    ) {
                        self.done.insert(u.key.clone(), u.status);
                    }
                }
                self.line(false);
            }
            AppEvent::Console { lines } if console => lines.iter().for_each(|l| println!("  | {l}")),
            AppEvent::RunFinished {
                outcome,
                exit_code,
                summary,
                report_available,
            } => {
                self.line(true);
                self.exit = Some(*exit_code);
                let code = exit_code.map_or_else(|| "none".to_string(), |c| c.to_string());
                let outcome = format!("{outcome:?}").to_lowercase();
                println!("Run {outcome}, exit code {code}, report {report_available}");
                if let Some(s) = summary {
                    println!(
                        "Karate: {} tests, {} passed, {} failed, {:.1} s",
                        s.scenario_count,
                        s.scenarios_passed,
                        s.scenarios_failed,
                        s.duration_millis / 1000.0
                    );
                }
            }
            AppEvent::Notice { message, .. } => println!("Notice: {message}"),
            _ => {}
        }
    }
}

fn main() -> ExitCode {
    let args = match args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("grace-run: {e}\nusage: grace-run <project folder> [--data <dir>] [--vendor <dir>] [--console]");
            return ExitCode::from(2);
        }
    };
    let Some(data) = args.data.or_else(data_dir) else {
        eprintln!("grace-run: no data folder: give --data");
        return ExitCode::from(2);
    };
    let vendor_dir = args.vendor.unwrap_or_else(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("vendor")
    });
    let vendor = vendor_paths(&vendor_dir);
    // what Grace does as it starts: a Java a crash left behind, argument files and run folders without run.json
    kill_orphan(&data.join("running.pid"), &vendor.jre_dir);
    remove_leftover_arg_files(&data);
    remove_leftover_runs(&data);

    let progress = Arc::new(Mutex::new(Progress::default()));
    let console = args.console;
    let runner = {
        let progress = Arc::clone(&progress);
        Runner::new(data.clone(), vendor, Arc::new(os_secret_box()), move |e| {
            progress.lock().unwrap().on(&e, console)
        })
    };
    let began = Instant::now();
    let opened = match runner.open(&args.folder) {
        Ok(opened) => opened,
        Err(e) => {
            eprintln!("grace-run: {e}");
            runner.dispose();
            return ExitCode::from(2);
        }
    };
    let root = match runner.current() {
        Some(AppEvent::Project {
            project: Some(project), ..
        }) => project.root,
        _ => {
            runner.dispose();
            return ExitCode::from(2);
        }
    };
    if !opened || progress.lock().unwrap().project_error.is_some() {
        runner.dispose();
        return ExitCode::from(if opened { 3 } else { 2 });
    }
    let ran = runner.run("d:");
    runner.dispose();
    if let Err(e) = ran {
        eprintln!("grace-run: {e}");
        return ExitCode::from(2);
    }
    let p = progress.lock().unwrap();
    println!(
        "Took {:.1} s from open to the stored run",
        began.elapsed().as_secs_f64()
    );
    let Some(exit) = p.exit else {
        return ExitCode::from(if p.project_error.is_some() { 3 } else { 2 });
    };
    if let Some(id) = &p.run_id {
        let dir = project_dir(&data, &root).join("runs").join(id);
        println!("run.json: {}", dir.join("run.json").display());
        let report = dir.join("karate-summary.html");
        if report.exists() {
            println!("Report:   {}", report.display());
        }
    }
    ExitCode::from(exit.map_or(1, |c| u8::try_from(c).unwrap_or(1)))
}
