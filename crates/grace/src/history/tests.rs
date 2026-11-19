//! `History.test.tsx`'s view tests, headless on the fake runner. What a row says and which of its buttons take a press
//! are `grace_core::history_list::tests`'; what assistive technology reads, a real run and the dialog's Escape are
//! `grace-e2e/tests/history.rs`'s.
use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Bounds, Entity, Modifiers, Pixels, TestAppContext, VisualTestContext, div, point, px};
use grace_core::model::events::RunOutcome;
use grace_core::model::history::{HistoryEntry, StoredSelection};
use grace_core::model::node::{Layout, NodeKind};
use grace_core::model::tags::FilterState;
use grace_core::state::{AppState, Panel};

use super::*;
use crate::testing::{self, Harness};

/// A clock of the test's own, so no label depends on the time zone or the date: the rows say what `row_text` works out.
fn now() -> f64 {
    1_792_000_000_000.
}

/// The time of the Vitest file's "2026-10-<day> <h>:<m>", as many minutes before `now()` as it was before Sunday 15:00.
fn at(day: u32, h: u32, m: u32) -> f64 {
    let minutes = i64::from(25 - day) * 24 * 60 + (15 - i64::from(h)) * 60 - i64::from(m);
    now() - minutes as f64 * 60_000.
}

fn entry(id: &str) -> HistoryEntry {
    HistoryEntry {
        started_at: at(25, 14, 32),
        duration_millis: 72_000.,
        ..testing::run(id, 41, 2)
    }
}

fn state(runs: Vec<HistoryEntry>) -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 2);
    s.panel = Panel::History;
    s.history = runs;
    s
}

fn setup(cx: &mut TestAppContext, runs: Vec<HistoryEntry>) -> (Harness, VisualTestContext, Entity<History>) {
    setup_at(cx, 1200., runs)
}

fn setup_at(
    cx: &mut TestAppContext,
    width: f32,
    runs: Vec<HistoryEntry>,
) -> (Harness, VisualTestContext, Entity<History>) {
    let slot: Rc<RefCell<Option<Entity<History>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let (h, mut vcx) = testing::open_rooted(cx, width, state(runs), move |grace, _, cx| {
        let list = s
            .borrow_mut()
            .get_or_insert_with(|| {
                let list = cx.new(|_| History::new(grace.clone()));
                list.update(cx, |l, _| l.clock = Rc::new(now));
                list
            })
            .clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_none().child(clear_button(grace, cx)))
            .child(list)
            .into_any_element()
    });
    draw(&mut vcx);
    let list = slot.borrow().clone().unwrap();
    (h, vcx, list)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn bounds(cx: &mut VisualTestContext, selector: String) -> Bounds<Pixels> {
    cx.debug_bounds(selector.clone().leak())
        .unwrap_or_else(|| panic!("{selector} is drawn"))
}

fn drawn(cx: &mut VisualTestContext, selector: &str) -> bool {
    cx.debug_bounds(selector.to_string().leak()).is_some()
}

fn click(cx: &mut VisualTestContext, selector: String) {
    let b = bounds(cx, selector);
    cx.simulate_click(b.center(), Modifiers::none());
    cx.run_until_parked();
}

fn texts(list: &Entity<History>, cx: &mut VisualTestContext) -> Vec<RowText> {
    cx.update(|_, cx| list.read(cx).rows().to_vec())
}

fn selection(kind: NodeKind, file: &str, name: &str) -> StoredSelection {
    StoredSelection {
        kind,
        file: file.into(),
        name: name.into(),
        line: None,
        key: "k".into(),
    }
}

/// 4, 5: the rows say what `history_list::row_text` works out, newest first, each with its columns.
#[gpui::test]
fn lists_a_row_per_run_with_its_labels(cx: &mut TestAppContext) {
    let runs = vec![
        HistoryEntry {
            filter: FilterState {
                include: vec!["smoke".into()],
                exclude: vec!["slow".into()],
                hide_non_matching: false,
            },
            ..entry("20261025-143200-000")
        },
        HistoryEntry {
            started_at: at(24, 17, 20),
            env: Some("qa".into()),
            threads: Some(8),
            outcome: RunOutcome::Cancelled,
            exit_code: None,
            passed: 1,
            failed: 0,
            total: 5,
            selection: Some(selection(NodeKind::Scenario, "tests/users.feature", "create user")),
            ..entry("20261024-172000-000")
        },
    ];
    let expected: Vec<_> = runs.iter().map(|e| row_text(e, now())).collect();
    let (_h, mut cx, list) = setup(cx, runs);
    let rows = texts(&list, &mut cx);
    assert_eq!(rows, expected, "newest first, as the runner sends them");
    assert_eq!(rows[0].result, "41 passed, 2 failed of 43");
    assert_eq!((rows[1].env.as_str(), rows[1].threads.as_str()), ("qa", "8"));
    assert_eq!(rows[1].result, "Cancelled, 1 passed, 0 failed of 5");
    for i in 1..=2 {
        for column in ["when", "env", "threads", "what", "result", "duration", "tags"] {
            assert!(drawn(&mut cx, &format!("history-{column}:{i}")), "{column} of row {i}");
        }
    }
    assert!(!drawn(&mut cx, "history-row:3"));
}

/// 10: a tag is drawn once per tag of the filter, a run with no filter says "—".
#[gpui::test]
fn draws_a_tag_by_its_shape_and_a_run_with_no_filter_says_a_dash(cx: &mut TestAppContext) {
    let runs = vec![
        HistoryEntry {
            filter: FilterState {
                include: vec!["smoke".into()],
                exclude: vec!["slow".into()],
                hide_non_matching: false,
            },
            ..entry("a")
        },
        entry("b"),
    ];
    let (_h, mut cx, list) = setup(cx, runs);
    let rows = texts(&list, &mut cx);
    assert_eq!(
        rows[0].tags,
        [
            Tag {
                text: "@smoke".into(),
                excluded: false
            },
            Tag {
                text: "not @slow".into(),
                excluded: true
            },
        ]
    );
    assert!(rows[1].tags.is_empty());
    for i in 1..=2 {
        assert!(drawn(&mut cx, &format!("history-tags:{i}")));
    }
}

/// 6, 7: where the node sits is a tooltip over its row, not a title; All and Re-run failed say nothing.
#[gpui::test]
fn puts_where_the_node_sits_in_a_tooltip_over_its_row(cx: &mut TestAppContext) {
    let runs = vec![
        HistoryEntry {
            selection: Some(selection(NodeKind::Scenario, "tests/users.feature", "create user")),
            ..entry("a")
        },
        entry("b"), // All
        HistoryEntry {
            selection: None,
            failed_of: Some(at(25, 14, 32)),
            ..entry("c")
        },
    ];
    let (h, mut cx, _list) = setup(cx, runs);
    let rest = |cx: &mut VisualTestContext, row: usize| {
        cx.simulate_mouse_move(point(px(5.), px(690.)), None, Modifiers::none());
        let b = bounds(cx, format!("history-what:{row}"));
        cx.simulate_mouse_move(b.center(), None, Modifiers::none());
        cx.executor().advance_clock(crate::tooltip::DELAY);
        cx.run_until_parked();
        (b, h.tip(cx).map(|t| t.to_string()), h.tip_bounds(cx))
    };
    let (b, tip, drawn_at) = rest(&mut cx, 1);
    assert_eq!(tip.as_deref(), Some("tests/users.feature › create user"));
    // over its row: below, it would take the click meant for the next row
    assert!(
        drawn_at.unwrap().origin.y <= b.origin.y + px(2.),
        "{drawn_at:?} over {b:?}"
    );
    assert_eq!(rest(&mut cx, 2).1, None, "All has no path");
    assert_eq!(rest(&mut cx, 3).1, None, "Failed tests has no path");
}

/// 12: 28 px rows under a header; the strip, the header and the rows scroll as one, and the header sticks (the trends'
/// `the_titles_stay_at_the_top_once_the_strip_has_scrolled_away`).
#[gpui::test]
fn makes_a_row_28_px_under_a_header_that_stays_above_the_rows(cx: &mut TestAppContext) {
    let (_h, mut cx, _list) = setup(cx, vec![entry("a"), entry("b")]);
    let (one, two) = (
        bounds(&mut cx, "history-row:1".into()),
        bounds(&mut cx, "history-row:2".into()),
    );
    assert_eq!(
        (one.size.height, two.origin.y - one.origin.y),
        (px(ROW_HEIGHT), px(ROW_HEIGHT))
    );
    let list = bounds(&mut cx, "history-list".into());
    assert_eq!(
        list.origin.y, one.origin.y,
        "the list starts with its first row; the header is above it"
    );
}

/// 15, 27: no runs, the empty state with the history icon, and Clear history is off; a run replaces it.
#[gpui::test]
fn says_no_runs_yet_in_an_empty_state_with_the_history_icon(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, vec![]);
    assert!(drawn(&mut cx, "empty-state"));
    assert!(!drawn(&mut cx, "history-row:1"));
    assert!(!drawn(&mut cx, "history-list"), "no header over nothing");
    assert_eq!(EMPTY, "No runs yet. Runs appear here when they finish.");
    // Clear history cannot be pressed with no runs
    click(&mut cx, "clear-history".into());
    assert!(h.backend.calls().is_empty());
    h.update(&mut cx, |s| s.history = vec![entry("a")]);
    assert!(!drawn(&mut cx, "empty-state"));
    assert_eq!(texts(&list, &mut cx).len(), 1);
}

/// 16, 17, 18: a row's buttons call the runner with its id; Report and Re-run failed are off where they have nothing to
/// do.
#[gpui::test]
fn the_buttons_of_a_row_call_the_runner_with_its_id(cx: &mut TestAppContext) {
    let runs = vec![entry("20261025-143200-000"), entry("20261024-172000-000")];
    let (h, mut cx, _list) = setup(cx, runs);
    click(&mut cx, "history-report:2".into());
    click(&mut cx, "history-rerun:1".into());
    click(&mut cx, "history-rerun-failed:2".into());
    assert_eq!(
        h.backend.calls(),
        [
            "run_report_path 20261024-172000-000",
            "rerun 20261025-143200-000",
            "rerun_failed 20261024-172000-000"
        ]
    );
    // the report opens in the default browser
    assert_eq!(
        h.seams.opened(),
        [std::path::PathBuf::from(
            "/runs/20261024-172000-000/karate-summary.html"
        )]
    );
}

#[gpui::test]
fn report_and_re_run_failed_do_nothing_where_they_are_off(cx: &mut TestAppContext) {
    let runs = vec![HistoryEntry {
        report_available: false,
        failed: 0,
        passed: 3,
        total: 3,
        ..entry("a")
    }];
    let (h, mut cx, _list) = setup(cx, runs);
    click(&mut cx, "history-report:1".into());
    click(&mut cx, "history-rerun-failed:1".into());
    assert!(h.backend.calls().is_empty());
    click(&mut cx, "history-rerun:1".into());
    assert_eq!(h.backend.calls(), ["rerun a"]);
}

/// 19: every action waits for the runner, a run in progress included; switching tabs stays possible.
#[gpui::test]
fn every_action_is_off_while_the_runner_is_busy_or_a_run_is_active(cx: &mut TestAppContext) {
    let (h, mut cx, _list) = setup(cx, vec![entry("a"), entry("b")]);
    for lock in [|s: &mut AppState| s.busy = true, |s: &mut AppState| s.run.active = true] {
        h.update(&mut cx, lock);
        for row in 1..=2 {
            for button in ["report", "export", "rerun", "rerun-failed"] {
                click(&mut cx, format!("history-{button}:{row}"));
            }
        }
        click(&mut cx, "clear-history".into());
        assert!(h.backend.calls().is_empty(), "{:?}", h.backend.calls());
        // and the toolbar's own
        let a = cx.update(|_, cx| crate::toolbar::actions(&h.grace.read(cx).state));
        assert!(!a.report && !a.rerun_failed);
        h.update(&mut cx, |s| (s.busy, s.run.active) = (false, false));
    }
    click(&mut cx, "history-rerun:1".into());
    assert_eq!(
        h.backend.calls(),
        ["rerun a"],
        "the buttons wake up once the runner is idle"
    );
}

/// How many times the rows were worked out: not on a redraw, so an idle tab costs nothing.
#[gpui::test]
fn works_the_rows_out_when_the_list_changes_not_on_a_redraw(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, vec![entry("a")]);
    let builds = |cx: &mut VisualTestContext| cx.update(|_, cx| list.read(cx).builds);
    assert_eq!(builds(&mut cx), 1);
    draw(&mut cx);
    h.update(&mut cx, |s| s.notice = None);
    assert_eq!(builds(&mut cx), 1, "a redraw with the same runs");
    h.update(&mut cx, |s| s.history = vec![entry("a"), entry("b")]);
    assert_eq!(builds(&mut cx), 2);
}

mod clear {
    use super::*;

    fn title(cx: &mut VisualTestContext) -> bool {
        drawn(cx, "clear-history-title")
    }

    /// Whether the title is `prompt`, measured: the title's width is that of the words in 16 px semibold.
    fn says(cx: &mut VisualTestContext, prompt: &str) -> bool {
        let width = f32::from(bounds(cx, "clear-history-title".into()).size.width);
        let said = cx.update(|window, _| testing::text_width(window, prompt, 16., gpui::FontWeight::SEMIBOLD));
        (width - said).abs() < 1.5
    }

    fn open(cx: &mut TestAppContext, n: usize) -> (Harness, VisualTestContext) {
        let runs: Vec<_> = (0..n).map(|i| entry(&format!("run-{i}"))).collect();
        let (h, mut cx, _) = setup(cx, runs);
        click(&mut cx, "clear-history".into());
        cx.run_until_parked();
        (h, cx)
    }

    /// 20: the dialog says how many runs, Delete asks the runner, Cancel does nothing.
    #[gpui::test]
    fn confirms_with_the_count_cancel_does_nothing_delete_clears(cx: &mut TestAppContext) {
        let (h, mut cx) = open(cx, 3);
        assert!(says(&mut cx, "Delete the 3 runs of this project and their reports?"));
        click(&mut cx, "clear-history-cancel".into());
        cx.executor().advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        assert!(h.backend.calls().is_empty());
        assert!(!title(&mut cx), "Cancel closes it");
        click(&mut cx, "clear-history".into());
        click(&mut cx, "clear-history-confirm".into());
        assert_eq!(h.backend.calls(), ["clear_history"]);
    }

    /// 21: Delete (danger) first, then Cancel, which has the focus (PU-32).
    #[gpui::test]
    fn puts_delete_first_then_cancel_which_has_the_focus(cx: &mut TestAppContext) {
        let (h, mut cx) = open(cx, 2);
        let (delete, cancel) = (
            bounds(&mut cx, "clear-history-confirm".into()),
            bounds(&mut cx, "clear-history-cancel".into()),
        );
        assert!(
            delete.right() <= cancel.left(),
            "Delete {delete:?} before Cancel {cancel:?}"
        );
        // Enter, which presses the focused button, cancels
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.executor().advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        assert!(!title(&mut cx));
        assert!(h.backend.calls().is_empty(), "Enter pressed Cancel, not Delete");
    }

    /// 23: Escape closes it and runs nothing.
    #[gpui::test]
    fn escape_closes_it_and_deletes_nothing(cx: &mut TestAppContext) {
        let (h, mut cx) = open(cx, 2);
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.executor().advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        assert!(!title(&mut cx));
        assert!(h.backend.calls().is_empty());
    }

    /// 22: one run is worded in the singular.
    #[gpui::test]
    fn words_one_run_in_the_singular(cx: &mut TestAppContext) {
        let (_h, mut cx) = open(cx, 1);
        assert!(says(&mut cx, "Delete the 1 run of this project and its report?"));
    }

    /// 24, 25: the dialog keeps the words it was opened with while the runner's empty list arrives, and counts again
    /// each time it opens.
    #[gpui::test]
    fn keeps_the_count_it_was_opened_with_and_counts_again_each_time(cx: &mut TestAppContext) {
        let (h, mut cx) = open(cx, 13);
        let thirteen = "Delete the 13 runs of this project and their reports?";
        assert!(says(&mut cx, thirteen));
        click(&mut cx, "clear-history-confirm".into());
        h.update(&mut cx, |s| s.history = vec![]); // the runner's answer, while the dialog fades out
        assert!(
            !title(&mut cx) || says(&mut cx, thirteen),
            "whole, or gone: never reworded"
        );
        cx.executor().advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        h.update(&mut cx, |s| {
            s.history = vec![entry("a"), entry("b"), entry("c"), entry("d")]
        });
        click(&mut cx, "clear-history".into());
        assert!(says(&mut cx, "Delete the 4 runs of this project and their reports?"));
    }
}

/// PU-32: Cancel, the second button, is the one with the focus when the dialog opens, so Enter cancels. Kit's own Enter
/// confirms an open dialog, so a press of Enter alone does not prove it.
#[gpui::test]
fn cancel_has_the_focus_when_the_dialog_opens(cx: &mut TestAppContext) {
    let (h, mut cx, _list) = setup(cx, vec![entry("a")]);
    let cancel = cx.update(|window, cx| open_clear(&h.grace, window, cx));
    cx.run_until_parked();
    assert!(
        cx.update(|window, _| cancel.is_focused(window)),
        "Cancel holds the focus"
    );
}

/// 16, 18 (Export's half), ER-05, ER-07, ER-08, ER-10: Export asks the runner where the run's report is, asks where to
/// save it, writes the zip there and says so; closing the dialog does nothing.
mod export {
    use super::*;

    fn notice(h: &Harness, cx: &mut VisualTestContext) -> Option<String> {
        cx.update(|_, cx| h.grace.read(cx).state.notice.as_ref().map(|n| n.text.clone()))
    }

    /// A run folder with its report, and a folder to save into; both are removed with the test.
    fn folders() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let run = t.path().join("run");
        std::fs::create_dir_all(run.join("feature-html")).unwrap();
        std::fs::write(run.join("karate-summary.html"), "<html>").unwrap();
        std::fs::write(run.join("feature-html/a.html"), "<html>").unwrap();
        std::fs::write(run.join("run.json"), "{}").unwrap();
        let out = t.path().join("out");
        std::fs::create_dir(&out).unwrap();
        (t, run, out)
    }

    fn zipped(zip: &std::path::Path) -> Vec<String> {
        let mut names: Vec<_> = grace_core::report_zip::read_zip(&std::fs::read(zip).unwrap())
            .unwrap()
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        names.sort();
        names
    }

    /// 18: the row's id goes to the runner, the dialog proposes the project's name for the zip, the zip is written where the
    /// person chose and the notice says where.
    #[gpui::test]
    fn saves_the_zip_where_the_dialog_says_and_says_so(cx: &mut TestAppContext) {
        let (_t, run, out) = folders();
        let (h, mut cx, _list) = setup(cx, vec![entry("20261025-143200-000"), entry("20261024-172000-000")]);
        let zip = out.join("r.zip");
        *h.backend.export_dir.lock().unwrap() = Some(run);
        *h.seams.save.lock().unwrap() = Some(zip.clone());
        click(&mut cx, "history-export:2".into());
        assert_eq!(h.backend.calls(), ["export_source 20261024-172000-000"]);
        assert_eq!(h.seams.saves(), ["p-report-20261024-172000-000.zip"]);
        assert_eq!(zipped(&zip), ["feature-html/a.html", "karate-summary.html"]);
        assert_eq!(
            notice(&h, &mut cx).as_deref(),
            Some(format!("Report saved to {}", zip.display()).as_str())
        );
    }

    /// ER-07: closing the dialog without saving writes nothing and says nothing.
    #[gpui::test]
    fn closing_the_dialog_does_nothing(cx: &mut TestAppContext) {
        let (_t, run, out) = folders();
        let (h, mut cx, _list) = setup(cx, vec![entry("a")]);
        *h.backend.export_dir.lock().unwrap() = Some(run);
        click(&mut cx, "history-export:1".into());
        assert_eq!(h.seams.saves().len(), 1, "the dialog was shown");
        assert_eq!(std::fs::read_dir(out).unwrap().count(), 0);
        assert_eq!(notice(&h, &mut cx), None);
    }

    /// ER-10: a zip that cannot be written says why and leaves nothing.
    #[gpui::test]
    fn says_why_when_the_zip_cannot_be_written_and_leaves_nothing(cx: &mut TestAppContext) {
        let (t, _run, out) = folders();
        let (h, mut cx, _list) = setup(cx, vec![entry("a")]);
        *h.backend.export_dir.lock().unwrap() = Some(t.path().join("gone"));
        *h.seams.save.lock().unwrap() = Some(out.join("r.zip"));
        click(&mut cx, "history-export:1".into());
        let said = notice(&h, &mut cx).unwrap();
        assert!(said.starts_with("The report could not be saved: "), "{said}");
        assert_eq!(std::fs::read_dir(out).unwrap().count(), 0);
    }

    /// 16 (Export's half), ER-05: where Report is off, Export is off and a press shows no dialog; and while the runner is
    /// busy or a run is active, every Export is.
    #[gpui::test]
    fn follows_report_off_without_a_report_and_while_busy(cx: &mut TestAppContext) {
        let runs = vec![
            HistoryEntry {
                report_available: false,
                ..entry("a")
            },
            entry("b"),
        ];
        let (h, mut cx, _list) = setup(cx, runs);
        click(&mut cx, "history-export:1".into());
        assert!(h.backend.calls().is_empty() && h.seams.saves().is_empty());
        for lock in [|s: &mut AppState| s.busy = true, |s: &mut AppState| s.run.active = true] {
            h.update(&mut cx, lock);
            click(&mut cx, "history-export:2".into());
            assert!(h.backend.calls().is_empty() && h.seams.saves().is_empty());
            h.update(&mut cx, |s| (s.busy, s.run.active) = (false, false));
        }
        click(&mut cx, "history-export:2".into());
        assert_eq!(h.seams.saves(), ["p-report-b.zip"], "on again once the runner is quiet");
    }

    /// Between Report and Re-run, as the spec has it (R2.1).
    #[gpui::test]
    fn sits_between_report_and_re_run(cx: &mut TestAppContext) {
        let (_h, mut cx, _list) = setup(cx, vec![entry("a")]);
        let x = |cx: &mut VisualTestContext, what: &str| bounds(cx, format!("history-{what}:1")).origin.x;
        let (report, export, rerun) = (x(&mut cx, "report"), x(&mut cx, "export"), x(&mut cx, "rerun"));
        assert!(report < export && export < rerun);
    }
}

/// A run's tags are drawn whole in a window of 1200 px, whose History table is 1160 px wide (the frame's margins and the
/// panel's borders, as measured on the screenshots): no chip is cut by the cell that holds them. Slice 5 fixed the same cut
/// once; the fourth button, Export, took room from the cell until the other columns were cut to what they need.
#[gpui::test]
fn draws_the_tags_whole_at_1200_px(cx: &mut TestAppContext) {
    let tagged = HistoryEntry {
        started_at: at(24, 17, 20),
        env: Some("staging".into()),
        outcome: RunOutcome::Cancelled,
        exit_code: None,
        passed: 1,
        failed: 1,
        total: 2,
        duration_millis: 83_000.,
        filter: FilterState {
            include: vec!["smoke".into()],
            exclude: vec!["slow".into()],
            hide_non_matching: false,
        },
        ..entry("a")
    };
    // a 1160 px table in IBM Plex: the test fonts' measured buttons are 56 px wider (382 against 326)
    let (_h, mut cx, _list) = setup_at(cx, 1216., vec![tagged]);
    let cell = bounds(&mut cx, "history-tags-cell:1".into());
    for k in 0..2 {
        let chip = bounds(&mut cx, format!("history-tag:1:{k}"));
        assert!(
            chip.right() <= cell.right(),
            "chip {k} {chip:?} runs past its cell {cell:?}"
        );
    }
}

/// Every button of a row lies inside the window and its Actions column, from the window's least width up: below the
/// width where the labelled buttons fit, they show their icon only (Redesigns, `history` (narrow rows)). A press
/// clicks a button's centre, so a button past the window's edge took no press (#384).
#[gpui::test]
fn every_button_of_a_row_fits_the_window_at_any_width(cx: &mut TestAppContext) {
    for width in [crate::window::MIN_SIZE.0, 1200.] {
        let (h, mut cx, _list) = setup_at(cx, width, vec![entry("a")]);
        let viewport = cx.update(|window, _| window.viewport_size().width);
        let column = bounds(&mut cx, "history-actions:1".into());
        assert!(
            column.right() <= viewport,
            "at {width} px the column {column:?} is in the window"
        );
        for button in ["report", "export", "rerun", "rerun-failed"] {
            let b = bounds(&mut cx, format!("history-{button}:1"));
            assert!(
                b.left() >= column.left() && b.right() <= column.right() + px(0.5),
                "at {width} px {button} {b:?} lies inside its column {column:?}"
            );
        }
        let report = bounds(&mut cx, "history-report:1".into()).size.width;
        assert_eq!(
            report == px(24.),
            width < 1000.,
            "at {width} px Report shows its icon only below the row's width"
        );
        click(&mut cx, "history-rerun-failed:1".into());
        assert_eq!(h.backend.calls(), ["rerun_failed a"], "at {width} px");
    }
}
