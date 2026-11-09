//! `runner.home.test.ts`.
use super::discovery::{busy, cancel_at, opening};
use super::*;
use crate::model::events::RecentEntry;
use crate::settings::{load_last_project, load_project_threads, load_recent_projects, save_opened_project};

const BUSY: &str = "A run is already in progress";

/// The busy, opening and project events.
fn around(t: &TestRunner) -> Vec<AppEvent> {
    t.events()
        .into_iter()
        .filter(|e| {
            matches!(
                e,
                AppEvent::Busy { .. } | AppEvent::Opening { .. } | AppEvent::Project { .. }
            )
        })
        .collect()
}

fn recent(t: &TestRunner) -> Vec<Vec<RecentEntry>> {
    t.events()
        .into_iter()
        .filter_map(|e| match e {
            AppEvent::Recent { entries } => Some(entries),
            _ => None,
        })
        .collect()
}

fn entry(path: &str) -> RecentEntry {
    RecentEntry {
        path: path.to_string(),
        exists: true,
    }
}

#[test]
fn open_emits_opening_around_the_discovery() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    assert_eq!(
        around(&t),
        vec![
            AppEvent::Busy { busy: true },
            AppEvent::Opening {
                root: Some(fx.root.clone())
            },
            t.last_event("project"),
            AppEvent::Opening { root: None },
            AppEvent::Busy { busy: false },
        ]
    );
}

#[test]
fn a_successful_open_saves_the_folder_first_and_as_last_project() {
    let t = setup();
    let (clean, tags) = (fixture("clean"), fixture("tags"));
    t.runner.open(&clean.root).unwrap();
    t.runner.open(&tags.root).unwrap();
    assert_eq!(
        load_recent_projects(&t.data_dir),
        [tags.root.clone(), clean.root.clone()]
    );
    assert_eq!(load_last_project(&t.data_dir), Some(tags.root.clone()));
    until(|| recent(&t).iter().any(|e| e.len() == 2));
    assert_eq!(recent(&t).pop().unwrap(), [entry(&tags.root), entry(&clean.root)]);
}

#[test]
fn a_failed_open_from_the_home_screen_changes_nothing() {
    let t = setup();
    let fx = fixture("clean");
    cancel_at(&t, opening);
    assert!(!t.runner.open(&fx.root).unwrap());
    assert_eq!(t.last_event("project"), no_project());
    assert_eq!(t.notices(), ["Cancelled"]);
    assert_eq!(t.last_event("opening"), AppEvent::Opening { root: None });
    assert_eq!(load_recent_projects(&t.data_dir), Vec::<String>::new());
    assert_eq!(load_last_project(&t.data_dir), None);
    let seen = t.count("opening");
    let missing = std::env::temp_dir().join("kk-does-not-exist");
    t.runner.open(&missing.to_string_lossy()).unwrap();
    assert_eq!(t.count("opening"), seen);
}

#[test]
fn a_failed_list_save_keeps_the_project_open() {
    let t = setup();
    fs::create_dir(t.data_dir.join("settings.json")).unwrap();
    let fx = fixture("clean");
    assert!(t.runner.open(&fx.root).unwrap());
    assert_eq!(t.last("project")["project"]["root"], json!(fx.root));
    assert!(
        t.notices()
            .iter()
            .any(|n| n.starts_with("The recent list could not be saved: "))
    );
}

#[test]
fn open_recent_opens_only_a_saved_folder() {
    let t = setup();
    let (clean, tags) = (fixture("clean"), fixture("tags"));
    save_opened_project(&t.data_dir, &clean.root).unwrap();
    t.runner.open_recent(&tags.root).unwrap();
    assert_eq!(t.notices(), ["Invalid request"]);
    assert_eq!(t.count("busy"), 0);
    t.runner.open_recent(&clean.root).unwrap();
    assert_eq!(t.last("project")["project"]["root"], json!(clean.root));
}

/// Calls `open_recent(listed)` again as the first `open_recent` starts to discover.
fn open_recent_twice(t: &TestRunner, listed: &str) {
    let runner = t.runner.clone();
    let again = listed.to_string();
    let done = std::sync::Once::new();
    t.on_event(move |e| {
        if opening(e) {
            done.call_once(|| runner.open_recent(&again).unwrap());
        }
    });
    t.runner.open_recent(listed).unwrap();
}

#[test]
fn a_second_open_recent_of_the_folder_being_opened_is_ignored() {
    let t = setup();
    let fx = fixture("clean");
    save_opened_project(&t.data_dir, &fx.root).unwrap();
    open_recent_twice(&t, &fx.root);
    assert_eq!(t.notices(), Vec::<String>::new());
}

#[test]
fn a_second_open_recent_is_ignored_whichever_way_the_entry_is_spelled() {
    let t = setup();
    let fx = fixture("clean");
    // open resolves it, a listed entry may not be
    let listed = format!("{}{}", fx.root, std::path::MAIN_SEPARATOR);
    save_opened_project(&t.data_dir, &listed).unwrap();
    open_recent_twice(&t, &listed);
    assert_eq!(t.notices(), Vec::<String>::new());
}

#[test]
fn remove_recent_says_so_when_the_list_cannot_be_saved() {
    let t = setup();
    let fx = fixture("clean");
    save_opened_project(&t.data_dir, &fx.root).unwrap();
    // the list stays readable, its new copy cannot be written
    fs::create_dir(t.data_dir.join("settings.json.tmp")).unwrap();
    t.runner.remove_recent(&fx.root);
    let notices = t.notices();
    assert_eq!(notices.len(), 1);
    assert!(notices[0].starts_with("The recent list could not be saved: "));
    assert_eq!(load_recent_projects(&t.data_dir), [fx.root.as_str()]);
}

#[test]
fn remove_recent_drops_a_saved_folder_only() {
    let t = setup();
    let (clean, tags) = (fixture("clean"), fixture("tags"));
    save_opened_project(&t.data_dir, &tags.root).unwrap();
    save_opened_project(&t.data_dir, &clean.root).unwrap();
    t.runner.remove_recent(&tags.root);
    assert_eq!(load_recent_projects(&t.data_dir), [clean.root.as_str()]);
    until(|| !recent(&t).is_empty());
    assert_eq!(recent(&t).pop().unwrap(), [entry(&clean.root)]);
    t.runner.remove_recent("D:\\not-saved");
    assert_eq!(t.notices(), ["Invalid request"]);
}

#[test]
fn close_forgets_the_project_and_last_project_keeps_the_rest() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_threads(&json!(2));
    t.runner.close();
    assert_eq!(t.last_event("project"), no_project());
    assert_eq!(t.runner.current(), None);
    assert_eq!(load_last_project(&t.data_dir), None);
    assert_eq!(load_recent_projects(&t.data_dir), [fx.root.as_str()]);
    assert_eq!(load_project_threads(&t.data_dir, &fx.root), 2);
    assert_eq!(
        t.runner.replay(),
        vec![AppEvent::Opening { root: None }, AppEvent::Busy { busy: false }]
    );
    t.runner.open(&fx.root).unwrap();
    assert_eq!(t.last_event("threads"), AppEvent::Threads { count: 2 });
}

#[test]
fn close_shows_the_notice_after_the_project_event_which_the_window_clears() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let settings = t.data_dir.join("settings.json");
    fs::remove_file(&settings).unwrap(); // open saved it: replace it with a folder
    fs::create_dir(&settings).unwrap(); // reading it as a file fails
    t.runner.close();
    assert!(t.last_notice().starts_with("The last project could not be forgotten: "));
    let events = t.json();
    let notice = events.iter().rposition(|e| e["type"] == "notice").unwrap();
    let project = events.iter().rposition(|e| e["type"] == "project").unwrap();
    assert!(notice > project);
}

#[test]
fn close_is_refused_while_busy() {
    let t = setup();
    let fx = fixture("clean");
    {
        let runner = t.runner.clone();
        let done = std::sync::Once::new();
        t.on_event(move |e| {
            if busy(e) {
                done.call_once(|| runner.close());
            }
        });
    }
    t.runner.open(&fx.root).unwrap();
    assert_eq!(t.notices(), [BUSY]);
    assert_eq!(t.count("project"), 1);
    assert_eq!(t.last("project")["project"]["root"], json!(fx.root));
    assert_eq!(load_last_project(&t.data_dir), Some(fx.root.clone()));
}

#[test]
fn replay_during_an_open_carries_the_folder_being_opened() {
    let t = setup();
    let fx = fixture("clean");
    let during: Arc<std::sync::OnceLock<Vec<AppEvent>>> = Arc::default();
    {
        let (runner, during) = (t.runner.clone(), Arc::clone(&during));
        t.on_event(move |e| {
            if opening(e) {
                let _ = during.set(runner.replay());
            }
        });
    }
    t.runner.open(&fx.root).unwrap();
    assert_eq!(
        during.get().unwrap(),
        &vec![
            AppEvent::Opening {
                root: Some(fx.root.clone())
            },
            AppEvent::Busy { busy: true },
        ]
    );
}
