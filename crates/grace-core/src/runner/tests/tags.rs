//! `runner.tags.test.ts`.
use super::discovery::{cancel_at, opening};
use super::*;
use crate::karate::command::{BuildArgs, build_args, detect_layout, tag_args};
use crate::karate::process::{SpawnOptions, spawn_karate};
use crate::karate::tree::build_tree;
use crate::model::tags::{FilterState, matching_leaves, project_tags};
use crate::settings::load_project_filter;

fn remove_wip(root: &Path) {
    let file = root.join("tests").join("basic.feature");
    let text = fs::read_to_string(&file).unwrap();
    let edited = regex::Regex::new(" ?@wip").unwrap().replace_all(&text, "").into_owned();
    assert_ne!(edited, text);
    fs::write(&file, edited).unwrap();
}

/// A dry run spawned like the Runner's, with `-t` from `f`.
fn dry_run_tree(root: &str, f: &FilterState) -> TestNode {
    let out = temp("kk-out-");
    let layout = detect_layout(Path::new(root));
    let vendor = vendor();
    let tags = tag_args(f);
    let targets: Vec<String> = layout
        .feature_roots
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let args = build_args(&BuildArgs {
        jar: &vendor.jar,
        classpath: &layout.classpath,
        root: Path::new(root),
        out_dir: out.path(),
        dry_run: true,
        tags: &tags,
        targets: &targets,
        env: None,
        arg_file: None,
        threads: 1,
        config_dir: None,
        debug: None,
        junit_xml: false,
    });
    let proc = spawn_karate(&vendor.java, &args, Path::new(root), |_| {}, SpawnOptions::default());
    assert_eq!(proc.wait(), Some(0));
    let events = read_all_events(out.path().join("karate-json").join("karate-events.jsonl")).unwrap();
    build_tree(&events, "x").unwrap()
}

#[test]
fn lists_inherited_feature_and_examples_tags() {
    let t = setup();
    let fx = fixture("tags");
    t.runner.open(&fx.root).unwrap();
    assert_eq!(project_tags(&t.tree()), ["fast", "feat", "ol", "other", "slow"]);
}

#[test]
fn runs_all_with_a_filter() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_filter(filter(&["smoke"], &["wip"], false));
    assert_eq!(
        t.last_event("filter"),
        AppEvent::Filter {
            filter: filter(&["smoke"], &["wip"], false)
        }
    );
    t.runner.run("d:").unwrap();
    assert_eq!(t.last("runStarted")["total"], 2);
    assert_match(
        &t.last("runFinished")["summary"],
        json!({ "scenarioCount": 2, "scenariosPassed": 2 }),
    );
}

#[test]
fn filters_a_feature_run() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_filter(filter(&[], &["wip"], false));
    t.runner.run("f:tests/basic.feature").unwrap();
    assert_eq!(t.last("runStarted")["total"], 2);
    assert_match(
        &t.last("runFinished")["summary"],
        json!({ "scenarioCount": 2, "scenariosPassed": 2 }),
    );
}

#[test]
fn runs_the_matching_rows_of_an_outline() {
    let t = setup();
    let fx = fixture("tags");
    t.runner.open(&fx.root).unwrap();
    let outline = by_name(&t.tree(), "speed <n>");
    t.runner.set_filter(filter(&["fast"], &[], false));
    t.runner.run(&outline.key).unwrap();
    let fast: Vec<&str> = outline.children[..2].iter().map(|r| r.key.as_str()).collect();
    assert_match(&t.last("runStarted"), json!({ "keys": fast, "total": 2 }));
    assert_match(
        &t.last("runFinished")["summary"],
        json!({ "scenarioCount": 2, "scenariosPassed": 2 }),
    );
}

#[test]
fn does_not_run_an_outlines_ignore_rows_when_every_listed_row_matches() {
    let t = setup();
    let fx = fixture("tags");
    t.runner.open(&fx.root).unwrap();
    let outline = by_name(&t.tree(), "speed <n>");
    assert_eq!(outline.children.len(), 3); // the @ignore Examples row is not listed
    t.runner.set_filter(filter(&["ol"], &[], false));
    t.runner.run(&outline.key).unwrap();
    assert_eq!(t.last("runStarted")["total"], 3);
    assert_match(
        &t.last("runFinished")["summary"],
        json!({ "scenarioCount": 3, "scenariosPassed": 3 }),
    );
    assert_eq!(t.count("notice"), 0);
}

#[test]
fn refuses_a_clicked_test_that_no_longer_matches() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let key = t.key("passing hello");
    t.runner.set_filter(filter(&["smoke"], &[], false));
    let file = fx.join("tests/basic.feature");
    let text = fs::read_to_string(&file).unwrap();
    let edited = regex::Regex::new(r"@smoke(\r?\nScenario: passing hello)")
        .unwrap()
        .replace(&text, "$1")
        .into_owned();
    assert_ne!(edited, text);
    fs::write(&file, edited).unwrap();
    t.runner.run(&key).unwrap();
    assert_eq!(t.last_notice(), "This test no longer matches the filter");
    assert_eq!(t.count("runStarted"), 0);
}

#[test]
fn a_vanished_include_tag_runs_nothing() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_filter(filter(&["wip"], &[], false));
    remove_wip(fx.path());
    t.runner.run("d:").unwrap();
    assert_eq!(t.last_notice(), "Nothing to run");
    assert_eq!(t.count("runStarted"), 0);
    // the pre-run discovery neither prunes nor saves
    assert_eq!(t.count("filter"), 2);
    assert!(t.runner.replay().contains(&AppEvent::Filter {
        filter: filter(&["wip"], &[], false)
    }));
    assert_eq!(load_project_filter(&t.data_dir, &fx.root), filter(&["wip"], &[], false));
}

#[test]
fn refresh_prunes_vanished_tags_and_saves() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_filter(filter(&["wip"], &["slow"], true));
    remove_wip(fx.path());
    t.runner.refresh().unwrap();
    assert_eq!(
        t.last_event("filter"),
        AppEvent::Filter {
            filter: filter(&[], &["slow"], true)
        }
    );
    assert_eq!(load_project_filter(&t.data_dir, &fx.root), filter(&[], &["slow"], true));
}

#[test]
fn restores_the_filter_on_open() {
    let first = setup();
    let fx = fixture("clean");
    first.runner.open(&fx.root).unwrap();
    first.runner.set_filter(filter(&["smoke"], &["wip"], true));
    first.runner.dispose();
    let second = setup_with(Setup {
        data: Some(Arc::clone(&first.data)),
        ..Setup::default()
    });
    second.runner.open(&fx.root).unwrap();
    let saved = filter(&["smoke"], &["wip"], true);
    assert_eq!(second.last_event("filter"), AppEvent::Filter { filter: saved.clone() });
    assert_eq!(second.runner.replay(), idle_replay(&second, saved, 1));
}

#[test]
fn a_cancelled_open_restores_the_filter_before() {
    let t = setup();
    let (clean, maven) = (fixture("clean"), fixture("maven"));
    t.runner.open(&clean.root).unwrap();
    t.runner.set_filter(filter(&["smoke"], &[], false));
    cancel_at(&t, opening);
    assert!(!t.runner.open(&maven.root).unwrap());
    assert_eq!(
        t.last_event("filter"),
        AppEvent::Filter {
            filter: filter(&["smoke"], &[], false)
        }
    );
    t.runner.run("d:").unwrap();
    assert_eq!(t.last("runStarted")["total"], 3);
}

#[test]
fn applies_a_filter_it_cannot_save() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let settings = t.data_dir.join("settings.json");
    fs::remove_file(&settings).unwrap(); // open saved it: replace it with a folder
    fs::create_dir(&settings).unwrap(); // writing a file there fails
    t.runner.set_filter(filter(&["smoke"], &[], false));
    assert!(t.last_notice().contains("The filter could not be saved"));
    assert_eq!(
        t.last_event("filter"),
        AppEvent::Filter {
            filter: filter(&["smoke"], &[], false)
        }
    );
    t.runner.run("d:").unwrap();
    assert_eq!(t.last("runStarted")["total"], 3);
}

#[test]
fn rejects_a_filter_change_while_busy() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let run = t.bg(|r| r.run("f:tests/slow.feature"));
    until(|| t.count("runStarted") > 0);
    t.runner.set_filter(filter(&["smoke"], &[], false));
    assert_eq!(t.last_notice(), "A run is already in progress");
    assert_eq!(t.last_event("filter"), AppEvent::Filter { filter: no_filter() });
    assert!(t.runner.replay().contains(&AppEvent::Filter { filter: no_filter() }));
    t.runner.cancel();
    run.join().unwrap().unwrap();
}

#[test]
fn karate_agrees_with_the_matching_function() {
    let fx = fixture("tags");
    let tree = dry_run_tree(&fx.root, &no_filter());
    // keys number rows by their position among the rows listed, so compare Karate's refIds
    let ids = |n: Vec<&TestNode>| -> Vec<String> {
        n.iter()
            .map(|l| format!("{}|{}", l.file, l.ref_id.as_deref().unwrap_or_default()))
            .collect()
    };
    let cases = [
        (filter(&["fast"], &[], false), 2),
        (filter(&["fast", "other"], &[], false), 3),
        (filter(&[], &["slow"], false), 3),
    ];
    for (f, count) in cases {
        let expected = ids(matching_leaves(&tree, &f));
        assert_eq!(expected.len(), count);
        let karate = dry_run_tree(&fx.root, &f);
        assert_eq!(ids(matching_leaves(&karate, &no_filter())), expected);
    }
}
