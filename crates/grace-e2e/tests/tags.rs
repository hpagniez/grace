//! `e2e/tags.spec.ts` (TF-*), and `virtual.spec.ts`'s filter bar with many tags (CF-07) and its filter scroll
//! (VL-20), through the accessibility tree. A chip is a button named with its state ("@smoke, included"); a row the
//! filter dims has no run button.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::{Path, PathBuf};
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(60);

fn label<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

/// The tree item whose name starts with `label` and a comma.
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

fn rows(tree: &Node) -> Vec<String> {
    tree.all()
        .into_iter()
        .filter(|n| n.role == Role::TreeItem)
        .map(|n| n.name.split(", ").next().unwrap_or_default().to_owned())
        .collect()
}

/// Whether the row of `label` says `status` (in lower case).
fn says(tree: &Node, label: &str, status: &str) -> bool {
    row(tree, label)
        .is_some_and(|r| r.name == format!("{label}, {status}") || r.name.starts_with(&format!("{label}, {status}, ")))
}

/// The chip of `tag`, in the state its name says.
fn chip<'a>(tree: &'a Node, tag: &str) -> Option<&'a Node> {
    let start = format!("@{tag}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.starts_with(&start))
}

fn chips(tree: &Node) -> Vec<String> {
    tree.all()
        .into_iter()
        .filter(|n| n.role == Role::Button && n.name.starts_with('@'))
        .map(|n| n.name.clone())
        .collect()
}

/// Whether `label`'s row has its run button: the filter dims a row by taking it away.
fn runs(tree: &Node, label: &str) -> bool {
    tree.find(&Role::Button, &format!("Run {label}")).is_some()
}

/// Presses the chip once per state given and waits for the runner's answer each time: the window never assumes.
fn click_chip(app: &App, tag: &str, states: &[&str]) {
    for state in states {
        let now = chip(&app.tree().unwrap(), tag)
            .unwrap_or_else(|| panic!("a chip @{tag}"))
            .clone();
        app.press(&now).unwrap();
        let want = format!("@{tag}, {state}");
        app.wait_for(&want, WAIT, |t| t.find(&Role::Button, &want).is_some())
            .unwrap();
    }
}

fn hide_non_matching(app: &App) {
    app.press_named(Role::CheckBox, "Hide non-matching").unwrap();
}

/// Opens each feature in turn, waiting for it to open before the next: an Expand button takes a click on its centre,
/// and the rows a feature opens move the buttons below it.
fn open_features(app: &App, features: &[&str]) {
    for f in features {
        app.press_named(Role::Button, &format!("Expand {f}")).unwrap();
        app.wait_for_node(Role::Button, &format!("Collapse {f}")).unwrap();
    }
}

/// Grace on a fixture project, its tests found.
fn open(dir: &Path, found: &str) -> App {
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| label(t, found).is_some())
        .unwrap();
    app
}

fn open_clean() -> (PathBuf, App) {
    let dir = project("clean").unwrap();
    let app = open(&dir, "9 tests · Not run yet");
    open_features(&app, &["basic.feature", "outline.feature", "slow.feature"]);
    app.wait_for("the features open", WAIT, |t| {
        row(t, "passing hello").is_some() && row(t, "smoke and slow").is_some()
    })
    .unwrap();
    (dir, app)
}

/// The clean project with "@smoke" included and "@wip" excluded.
fn open_clean_smoke_not_wip() -> (PathBuf, App) {
    let (dir, app) = open_clean();
    click_chip(&app, "smoke", &["included"]);
    click_chip(&app, "wip", &["included", "excluded"]);
    (dir, app)
}

/// Like another editor: strips every "@wip" tag from basic.feature.
fn remove_wip_tags(dir: &Path) {
    let file = dir.join("tests").join("basic.feature");
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(file, text.replace("@wip", "")).unwrap();
}

/// TF-01
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn chips_list_the_projects_tags() {
    let (_, app) = open_clean();
    // the chips with the tooltip's next click as their description (`Tooltip`'s `describe`), read together: UI
    // Automation can answer a read before it has a node's description
    let hint = "Not filtered. Click to include.";
    let tree = app
        .wait_for("the chips and their hints", WAIT, |t| {
            chip(t, "slow").is_some_and(|c| c.description == hint)
        })
        .unwrap();
    assert_eq!(
        chips(&tree),
        ["@slow, not filtered", "@smoke, not filtered", "@wip, not filtered"],
        "{}",
        tree.dump()
    );
    assert!(tree.find(&Role::CheckBox, "Hide non-matching").is_some());
    assert!(tree.find(&Role::Button, "Clear").is_none());
}

/// TF-02
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn include_and_exclude_dim_the_tree() {
    let (_, app) = open_clean_smoke_not_wip();
    let tree = app
        .wait_for("2 of 9 match", WAIT, |t| label(t, "2 of 9 tests match").is_some())
        .unwrap();
    for name in ["passing hello", "smoke and slow"] {
        assert!(runs(&tree, name), "{name}: {}", tree.dump());
    }
    for name in ["failing match", "smoke but wip", "outline.feature"] {
        assert!(
            row(&tree, name).is_some() && !runs(&tree, name),
            "{name}: {}",
            tree.dump()
        );
    }
}

/// TF-03
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_third_click_turns_a_chip_off() {
    let (_, app) = open_clean();
    click_chip(&app, "smoke", &["included"]);
    app.wait_for("outline dimmed", WAIT, |t| !runs(t, "outline.feature"))
        .unwrap();
    click_chip(&app, "smoke", &["excluded", "not filtered"]);
    app.wait_for("nothing dimmed, no counter", WAIT, |t| {
        runs(t, "outline.feature")
            && runs(t, "failing match")
            && !t.all().iter().any(|n| n.name.ends_with("tests match"))
    })
    .unwrap();
}

/// TF-04
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn hide_non_matching_hides_the_other_rows() {
    let (_, app) = open_clean_smoke_not_wip();
    hide_non_matching(&app);
    let tree = app
        .wait_for("only the matches", WAIT, |t| row(t, "outline.feature").is_none())
        .unwrap();
    assert_eq!(
        rows(&tree),
        [
            "tests",
            "basic.feature",
            "passing hello",
            "slow.feature",
            "smoke and slow"
        ],
        "{}",
        tree.dump()
    );
}

/// TF-05
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn run_all_runs_only_matching_tests() {
    let (_, app) = open_clean();
    // "failing match" gets a status first, to see that a filtered run leaves it alone
    app.press_named(Role::Button, "Run failing match").unwrap();
    app.wait_for("failing match failed", RUN, |t| {
        label(t, "Finished").is_some() && says(t, "failing match", "failed")
    })
    .unwrap();
    click_chip(&app, "smoke", &["included"]);
    click_chip(&app, "wip", &["included", "excluded"]);
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("2 of 2 passed", RUN, |t| {
        label(t, "Finished").is_some()
            && label(t, "2 / 2").is_some()
            && label(t, "2 passed").is_some()
            && says(t, "passing hello", "passed")
            && says(t, "smoke and slow", "passed")
            && says(t, "failing match", "failed")
            && says(t, "smoke but wip", "not run")
    })
    .unwrap();
}

/// TF-06
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_feature_run_is_filtered() {
    let (_, app) = open_clean();
    click_chip(&app, "wip", &["included", "excluded"]);
    app.press_named(Role::Button, "Run basic.feature").unwrap();
    app.wait_for("2 of 2 passed", RUN, |t| {
        label(t, "Finished").is_some()
            && label(t, "2 / 2").is_some()
            && says(t, "passing hello", "passed")
            && says(t, "calls helper feature", "passed")
            && says(t, "failing match", "not run")
            && says(t, "smoke but wip", "not run")
    })
    .unwrap();
}

/// TF-07
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_tag_removed_before_a_run_does_not_widen_it() {
    let (dir, app) = open_clean();
    click_chip(&app, "wip", &["included"]);
    remove_wip_tags(&dir);
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("Nothing to run", WAIT, |t| label(t, "Nothing to run").is_some())
        .unwrap();
    let tree = app.tree().unwrap();
    assert!(label(&tree, "Finished").is_none(), "{}", tree.dump());
    // the pre-run dry run sees the new tree but prunes nothing
    assert!(tree.find(&Role::Button, "@wip, included").is_some(), "{}", tree.dump());
}

/// TF-08
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn part_of_an_outline_matches() {
    let dir = project("tags").unwrap();
    let app = open(&dir, "4 tests · Not run yet");
    open_features(&app, &["tags.feature"]);
    app.wait_for("the outline", WAIT, |t| row(t, "speed <n>").is_some())
        .unwrap();
    click_chip(&app, "fast", &["included"]);
    app.press_named(Role::Button, "Run speed <n>").unwrap();
    app.wait_for("rows 1 and 2 passed alone", RUN, |t| {
        label(t, "Finished").is_some()
            && label(t, "2 / 2").is_some()
            && says(t, "speed 1", "passed")
            && says(t, "speed 2", "passed")
            && says(t, "speed 3", "not run")
            && says(t, "other", "not run")
    })
    .unwrap();
}

/// TF-09: the counter says nothing matches. That Run all is disabled then is the toolbar's headless test (WS-10):
/// on Linux AccessKit drops a button's disabled state.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn nothing_matches() {
    let dir = project("tags").unwrap();
    let app = open(&dir, "4 tests · Not run yet");
    click_chip(&app, "feat", &["included", "excluded"]);
    app.wait_for("0 of 4", WAIT, |t| label(t, "0 of 4 tests match").is_some())
        .unwrap();
}

/// TF-10
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_filter_survives_a_restart() {
    let dir = project("clean").unwrap();
    let app = open(&dir, "9 tests · Not run yet");
    click_chip(&app, "smoke", &["included"]);
    hide_non_matching(&app);
    app.wait_for("outline hidden", WAIT, |t| row(t, "outline.feature").is_none())
        .unwrap();
    let app = app.relaunch(&[]).unwrap();
    app.wait_for("the filter back", WAIT, |t| {
        t.find(&Role::Button, "@smoke, included").is_some()
            && t.find(&Role::Button, "@wip, not filtered").is_some()
            && row(t, "basic.feature").is_some()
            && row(t, "outline.feature").is_none()
    })
    .unwrap();
}

/// TF-11
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_tag_removed_from_the_project_leaves_the_filter() {
    let (dir, app) = open_clean();
    click_chip(&app, "wip", &["included", "excluded"]);
    remove_wip_tags(&dir);
    app.press_named(Role::Button, "Refresh").unwrap();
    let tree = app
        .wait_for("@wip gone", WAIT, |t| {
            chip(t, "wip").is_none() && chip(t, "smoke").is_some()
        })
        .unwrap();
    assert_eq!(chips(&tree), ["@slow, not filtered", "@smoke, not filtered"]);
    assert!(runs(&tree, "failing match"), "{}", tree.dump());
    // the one project's saved filter, pruned
    app.wait_until("the pruned filter saved", WAIT, || {
        app.settings()["projects"].as_object().is_some_and(|p| {
            p.len() == 1
                && p.values()
                    .all(|f| *f == json!({ "include": [], "exclude": [], "hideNonMatching": false }))
        })
    })
    .unwrap();
}

/// A project of 54 features in one folder, each with one scenario and its own tag `@module-N`.
fn many_tags() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("grace-e2e-tags-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    for i in 0..54 {
        std::fs::write(
            dir.join("tests").join(format!("feature-{i:02}.feature")),
            format!("@module-{i:02}\nFeature: feature {i}\n\nScenario: scenario {i}\n  * def x = {i}\n"),
        )
        .unwrap();
    }
    dir
}

/// The fold chip: "N more tags" with ", K on".
fn more(tree: &Node) -> Option<&Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.contains(" more tag"))
}

/// CF-07: 54 tags keep the bar one line; the panel holds them all and includes one; Escape closes it and gives the
/// focus back to the fold chip, and Tab goes from there into the panel.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn many_tags_keep_a_one_line_bar() {
    let dir = many_tags();
    let app = open(&dir, "54 tests · Not run yet");
    let tree = app.wait_for("the fold chip", WAIT, |t| more(t).is_some()).unwrap();
    let shown = chips(&tree).len();
    let folded = 54 - shown;
    assert!(folded > 0 && shown > 0, "{}", tree.dump());
    assert_eq!(
        more(&tree).unwrap().name,
        format!("{folded} more tags"),
        "{}",
        tree.dump()
    );
    // its expanded state is the headless tests': on Linux the pinned AccessKit adapter drops it

    app.press(more(&tree).unwrap()).unwrap();
    let tree = app
        .wait_for("the panel", WAIT, |t| t.find(&Role::Group, "All tags").is_some())
        .unwrap();
    let panel = tree.find(&Role::Group, "All tags").unwrap();
    assert_eq!(chips(panel).len(), 54, "{}", tree.dump());
    let last = chip(panel, "module-53").unwrap().clone();
    app.press(&last).unwrap();
    app.wait_for("module-53 included, the panel open, the count on", WAIT, |t| {
        t.find(&Role::Group, "All tags")
            .is_some_and(|p| p.find(&Role::Button, "@module-53, included").is_some())
            && more(t).is_some_and(|m| m.name.ends_with(", 1 on"))
            && label(t, "1 of 54 tests match").is_some()
    })
    .unwrap();
    app.key(Key::Escape).unwrap();
    app.wait_for("the panel closed, the focus on the fold chip", WAIT, |t| {
        t.find(&Role::Group, "All tags").is_none() && more(t).is_some_and(|m| m.states.focused)
    })
    .unwrap();
    // the keyboard opens it again, and Tab goes from the fold chip into the panel (R3.4)
    app.key(Key::Enter).unwrap();
    app.wait_for("the panel again", WAIT, |t| t.find(&Role::Group, "All tags").is_some())
        .unwrap();
    app.key(Key::Tab).unwrap();
    app.wait_for("the focus on the panel's first chip", WAIT, |t| {
        t.find(&Role::Group, "All tags").is_some_and(|p| {
            p.all()
                .iter()
                .any(|n| n.states.focused && n.name.starts_with("@module-00, "))
        })
    })
    .unwrap();
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
}

/// VL-20: a filter change brings the selected row into view. The last feature, selected at the end of a short list,
/// is far below once the filter is cleared and every feature is back above it.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_filter_change_scrolls_to_the_selected_row() {
    let dir = many_tags();
    let app = open(&dir, "54 tests · Not run yet");
    app.wait_for("the fold chip", WAIT, |t| more(t).is_some()).unwrap();
    // only module-00 and module-53 shown
    for tag in ["module-00", "module-53"] {
        let tree = app
            .wait_for("the panel closed", WAIT, |t| {
                more(t).is_some() && t.find(&Role::Group, "All tags").is_none()
            })
            .unwrap();
        app.press_named(Role::Button, &more(&tree).unwrap().name).unwrap();
        app.wait_for("the panel", WAIT, |t| t.find(&Role::Group, "All tags").is_some())
            .unwrap();
        // the chip in the panel: the one on the line, if any, comes first in the tree
        app.press_named(Role::Button, &format!("@{tag}, not filtered")).unwrap();
        let want = format!("@{tag}, included");
        app.wait_for(&want, WAIT, |t| t.find(&Role::Button, &want).is_some())
            .unwrap();
        app.key(Key::Escape).unwrap();
    }
    hide_non_matching(&app);
    let tree = app
        .wait_for("two features left", WAIT, |t| {
            row(t, "feature-53.feature").is_some() && row(t, "feature-30.feature").is_none()
        })
        .unwrap();
    app.press(row(&tree, "feature-53.feature").unwrap()).unwrap();
    app.wait_for("feature-53 selected", WAIT, |t| {
        row(t, "feature-53.feature").is_some_and(|r| r.states.selected)
    })
    .unwrap();
    app.press_named(Role::Button, "Clear").unwrap();
    let tree = app
        .wait_for("every feature back, feature-53 in view", WAIT, |t| {
            label(t, "1 of 54 tests match").is_none()
                && t.all().iter().all(|n| !n.name.starts_with("@module-53, included"))
                && row(t, "feature-53.feature").is_some_and(|r| r.states.selected)
                && row(t, "feature-00.feature").is_none()
        })
        .unwrap();
    // one wait for the filter and the scroll: on Windows CI the list had not moved yet when the filter was read
    assert!(row(&tree, "feature-53.feature").is_some(), "{}", tree.dump());
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
}
