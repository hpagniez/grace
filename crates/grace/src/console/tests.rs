//! `Console.test.tsx`'s and `Banner.test.tsx`'s tests, headless on the fake runner: the lines, the banner, the find
//! bar and the error stepping. What a screen reader reads and the real keys are `grace-e2e/tests/console.rs`'s.
use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AppContext as _, Entity, IntoElement as _, ParentElement as _, Styled as _, TestAppContext, VisualTestContext, div,
    px,
};
use grace_core::model::events::AppEvent;
use grace_core::model::node::Layout;
use grace_core::state::{CONSOLE_LIMIT, Panel};

use super::*;
use crate::testing::{self, Harness};

/// A console 300 px tall on a project of 3 tests.
fn setup(cx: &mut TestAppContext) -> (Harness, VisualTestContext, Entity<Console>) {
    let slot: Rc<RefCell<Option<Entity<Console>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let state = testing::project("/p", Layout::Standalone, 3);
    let (h, vcx) = testing::open_with(cx, 1000., state, move |grace, window, cx| {
        let c = cx.new(|cx| Console::new(grace.clone(), window, cx));
        *s.borrow_mut() = Some(c.clone());
        Box::new(move |_, _, _| div().h(px(300.)).flex().flex_col().child(c.clone()).into_any_element())
    });
    let console = slot.borrow().clone().unwrap();
    (h, vcx, console)
}

fn add(h: &Harness, cx: &mut VisualTestContext, lines: &[&str]) {
    let lines = lines.iter().map(|l| l.to_string()).collect();
    h.update(cx, |s| s.reduce(&AppEvent::Console { lines }));
}

fn numbered(n: usize, from: usize) -> Vec<String> {
    (from..from + n).map(|i| format!("l{i}")).collect()
}

fn add_numbered(h: &Harness, cx: &mut VisualTestContext, n: usize, from: usize) {
    let lines = numbered(n, from);
    h.update(cx, |s| s.reduce(&AppEvent::Console { lines }));
}

/// A new run: the console empties.
fn clear(h: &Harness, cx: &mut VisualTestContext) {
    h.update(cx, |s| {
        s.console_dropped += s.console.len() as u64;
        s.console.clear();
    });
}

fn open_bar(c: &Entity<Console>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| c.update(cx, |c, cx| c.open_bar(window, cx)));
    draw(cx);
}

fn type_text(cx: &mut VisualTestContext, text: &str) {
    cx.simulate_input(text);
    cx.run_until_parked();
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

fn count(c: &Entity<Console>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, cx| c.read(cx).count(cx))
}

fn error_count(c: &Entity<Console>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, cx| c.read(cx).error_count(cx))
}

/// The first item in view: 0 is the banner, line `i` is item `i + 1`.
fn top(c: &Entity<Console>, cx: &mut VisualTestContext) -> usize {
    cx.update(|_, cx| c.read(cx).list.logical_scroll_top().item_ix)
}

fn following(c: &Entity<Console>, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| c.read(cx).list.is_following_tail())
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

mod lines {
    use super::*;

    #[gpui::test]
    fn keeps_following_once_the_store_holds_the_line_cap(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add_numbered(&h, &mut cx, CONSOLE_LIMIT, 0);
        add_numbered(&h, &mut cx, 10, CONSOLE_LIMIT);
        draw(&mut cx);
        assert_eq!(cx.update(|_, cx| h.grace.read(cx).state.console.len()), CONSOLE_LIMIT);
        assert!(following(&c, &mut cx));
        let at = top(&c, &mut cx);
        assert!(at > CONSOLE_LIMIT - 40, "{at}");
        add_numbered(&h, &mut cx, 5, CONSOLE_LIMIT + 10);
        draw(&mut cx);
        assert!(following(&c, &mut cx));
        assert!(top(&c, &mut cx) >= at);
    }

    #[gpui::test]
    fn keeps_the_banner_above_the_lines_past_the_line_cap(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add_numbered(&h, &mut cx, CONSOLE_LIMIT + 10, 0);
        draw(&mut cx);
        // the banner is item 0 whatever dropped: the list has one item more than the lines
        assert_eq!(cx.update(|_, cx| c.read(cx).list.item_count()), CONSOLE_LIMIT + 1);
    }

    #[gpui::test]
    fn styles_error_and_warn_lines(cx: &mut TestAppContext) {
        let (_h, mut cx, _c) = setup(cx);
        let t = cx.update(|_, cx| Theme::global(cx).tokens);
        assert_eq!(line_colour("plain", t), None);
        assert_eq!(line_colour("12:00 [main] ERROR boom", t), Some(t.failed));
        assert_eq!(line_colour("12:00 [main] WARN careful", t), Some(t.cancelled));
    }

    #[gpui::test]
    fn follows_new_lines_unless_the_user_scrolled_up(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add_numbered(&h, &mut cx, 100, 0);
        draw(&mut cx);
        let bottom = top(&c, &mut cx);
        assert!(bottom > 60, "{bottom}");
        // the user scrolls up
        cx.update(|_, cx| c.read(cx).list.scroll_by(px(-400.)));
        draw(&mut cx);
        let up = top(&c, &mut cx);
        assert!(up < bottom);
        add(&h, &mut cx, &["more"]);
        draw(&mut cx);
        assert_eq!(top(&c, &mut cx), up);
        // back at the bottom, it follows again
        cx.update(|_, cx| c.read(cx).list.scroll_by(px(10_000.)));
        draw(&mut cx);
        add(&h, &mut cx, &["again"]);
        draw(&mut cx);
        assert!(following(&c, &mut cx));
        assert!(top(&c, &mut cx) > bottom);
    }

    #[gpui::test]
    fn a_new_run_clears_the_lines(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add_numbered(&h, &mut cx, 50, 0);
        clear(&h, &mut cx);
        add(&h, &mut cx, &["first of the new run"]);
        assert_eq!(cx.update(|_, cx| c.read(cx).list.item_count()), 2);
    }
}

mod banner {
    use grace_core::banner::BannerState;

    use super::*;

    #[gpui::test]
    fn says_what_the_app_is_doing(cx: &mut TestAppContext) {
        let (h, mut cx, _c) = setup(cx);
        let state = cx.update(|_, cx| h.grace.read(cx).state.clone());
        assert_eq!(state_text(BannerState::None, &state), "Open a Karate project folder");
        assert_eq!(state_text(BannerState::Starting, &state), "Starting Karate…");
        assert_eq!(
            state_text(BannerState::Ready, &state),
            "Ready. Press Run all, or ▶ on a test."
        );
        assert_eq!(
            state_text(BannerState::Finding, &state),
            crate::progress::finding_text(&state)
        );
    }

    #[test]
    fn the_cursor_blinks_only_while_grace_works_and_never_with_reduced_motion() {
        assert!(blinks(Some(BannerState::Finding), false));
        assert!(blinks(Some(BannerState::Starting), false));
        assert!(!blinks(Some(BannerState::Ready), false));
        assert!(!blinks(Some(BannerState::None), false));
        assert!(!blinks(None, false));
        assert!(!blinks(Some(BannerState::Finding), true));
        assert!(!blinks(Some(BannerState::Starting), true));
    }

    #[test]
    fn names_the_pinned_versions() {
        let vendor: serde_json::Value = serde_json::from_str(include_str!("../../../../scripts/vendor.json")).unwrap();
        let package: serde_json::Value = serde_json::from_str(include_str!("../../../../package.json")).unwrap();
        assert_eq!(VERSIONS.0, package["version"].as_str().unwrap());
        assert_eq!(VERSIONS.1, vendor["karate"]["version"].as_str().unwrap());
        assert_eq!(VERSIONS.2, vendor["jre"]["version"].as_str().unwrap());
    }
}

mod find {
    use super::*;

    fn held(c: &Entity<Console>, cx: &mut VisualTestContext) -> Match {
        cx.update(|_, cx| {
            let c = c.read(cx);
            c.matches()[c.current(cx).unwrap()]
        })
    }

    #[gpui::test]
    fn opening_the_bar_shows_the_console_tab_and_focuses_its_field(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        h.update(&mut cx, |s| s.show_panel(Panel::History));
        open_bar(&c, &mut cx);
        assert_eq!(cx.update(|_, cx| h.grace.read(cx).state.panel), Panel::Console);
        assert!(cx.update(|_, cx| c.read(cx).is_open()));
        let field = cx.update(|_, cx| c.read(cx).field.clone());
        assert!(cx.update(|window, cx| gpui::Focusable::focus_handle(field.read(cx), cx).is_focused(window)));
    }

    #[gpui::test]
    fn counts_matches_and_steps_with_enter_and_shift_enter_wrapping(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &["get user 1", "nothing", "create USER", "user user"]);
        open_bar(&c, &mut cx);
        type_text(&mut cx, "user");
        assert_eq!(count(&c, &mut cx), "1 of 4");
        keys(&mut cx, "enter");
        assert_eq!(count(&c, &mut cx), "2 of 4");
        keys(&mut cx, "enter");
        keys(&mut cx, "enter");
        assert_eq!(count(&c, &mut cx), "4 of 4");
        keys(&mut cx, "enter");
        assert_eq!(count(&c, &mut cx), "1 of 4");
        keys(&mut cx, "shift-enter");
        assert_eq!(count(&c, &mut cx), "4 of 4");
        // the buttons step too
        c.update(&mut cx, |c, cx| c.step(true, cx));
        assert_eq!(count(&c, &mut cx), "1 of 4");
    }

    #[gpui::test]
    fn shows_no_results_and_nothing_for_no_text(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &["a line"]);
        open_bar(&c, &mut cx);
        assert_eq!(count(&c, &mut cx), "");
        type_text(&mut cx, "zzz");
        assert_eq!(count(&c, &mut cx), NO_RESULTS);
    }

    #[gpui::test]
    fn escape_closes_the_bar_keeps_the_text_and_gives_the_focus_back(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &["user"]);
        let before = cx.update(|_, cx| cx.focus_handle());
        cx.update(|window, cx| window.focus(&before, cx));
        open_bar(&c, &mut cx);
        type_text(&mut cx, "user");
        keys(&mut cx, "escape");
        assert!(!cx.update(|_, cx| c.read(cx).is_open()));
        assert_eq!(cx.update(|_, cx| c.read(cx).query(cx)), "user");
        assert!(cx.update(|window, _| before.is_focused(window)));
        // reopened, it finds again with its text
        open_bar(&c, &mut cx);
        assert_eq!(count(&c, &mut cx), "1 of 1");
    }

    #[gpui::test]
    fn keeps_the_text_across_a_tab_switch_and_a_new_run(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &["user"]);
        open_bar(&c, &mut cx);
        type_text(&mut cx, "user");
        h.update(&mut cx, |s| s.show_panel(Panel::History));
        h.update(&mut cx, |s| s.show_panel(Panel::Console));
        assert_eq!(count(&c, &mut cx), "1 of 1");
        clear(&h, &mut cx);
        add(&h, &mut cx, &["user again", "user"]);
        assert_eq!(cx.update(|_, cx| c.read(cx).query(cx)), "user");
        assert_eq!(count(&c, &mut cx), "1 of 2");
    }

    #[gpui::test]
    fn keeps_the_current_match_on_its_line_as_lines_arrive(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &["user a", "user b"]);
        open_bar(&c, &mut cx);
        type_text(&mut cx, "user");
        keys(&mut cx, "enter");
        assert_eq!(count(&c, &mut cx), "2 of 2");
        add(&h, &mut cx, &["user c"]);
        assert_eq!(count(&c, &mut cx), "2 of 3");
    }

    #[gpui::test]
    fn keeps_the_current_matchs_line_when_older_lines_drop_at_the_cap(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        let mut lines = numbered(CONSOLE_LIMIT - 2, 0);
        lines.push("hit one".into());
        lines.push("hit two".into());
        h.update(&mut cx, |s| s.reduce(&AppEvent::Console { lines }));
        open_bar(&c, &mut cx);
        type_text(&mut cx, "hit");
        if count(&c, &mut cx) == "1 of 2" {
            keys(&mut cx, "enter");
        }
        assert_eq!(held(&c, &mut cx).line, CONSOLE_LIMIT - 1);
        add(&h, &mut cx, &["x", "y", "z"]);
        // three older lines dropped: the same line moved up by three
        assert_eq!(held(&c, &mut cx).line, CONSOLE_LIMIT - 4);
        assert_eq!(count(&c, &mut cx), "2 of 2");
    }

    #[gpui::test]
    fn a_step_stops_following_the_bottom(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        let mut lines = vec!["hit".to_string()];
        lines.extend(numbered(200, 0));
        h.update(&mut cx, |s| s.reduce(&AppEvent::Console { lines }));
        draw(&mut cx);
        assert!(following(&c, &mut cx));
        open_bar(&c, &mut cx);
        type_text(&mut cx, "hit");
        draw(&mut cx);
        assert!(!following(&c, &mut cx));
        assert!(top(&c, &mut cx) <= 1, "the match's line shows");
        add(&h, &mut cx, &["more"]);
        draw(&mut cx);
        assert!(top(&c, &mut cx) <= 1, "new lines do not move the view");
    }

    #[gpui::test]
    fn picks_the_current_match_from_the_view_again_when_the_bar_reopens(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        let mut lines = vec!["hit top".to_string()];
        lines.extend(numbered(200, 0));
        lines.push("hit bottom".into());
        h.update(&mut cx, |s| s.reduce(&AppEvent::Console { lines }));
        draw(&mut cx);
        open_bar(&c, &mut cx);
        type_text(&mut cx, "hit");
        // the view was at the bottom: the first match from its top is the last line's
        assert_eq!(count(&c, &mut cx), "2 of 2");
        keys(&mut cx, "enter");
        assert_eq!(count(&c, &mut cx), "1 of 2");
        draw(&mut cx);
        keys(&mut cx, "escape");
        cx.update(|_, cx| c.read(cx).list.scroll_by(px(100_000.)));
        draw(&mut cx);
        open_bar(&c, &mut cx);
        assert_eq!(count(&c, &mut cx), "2 of 2");
    }

    #[gpui::test]
    fn does_not_search_while_another_tab_shows_or_the_bar_is_closed(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &["user"]);
        assert!(cx.update(|_, cx| c.read(cx).matches().is_empty()));
        open_bar(&c, &mut cx);
        type_text(&mut cx, "user");
        assert_eq!(cx.update(|_, cx| c.read(cx).matches().len()), 1);
        h.update(&mut cx, |s| s.show_panel(Panel::Http));
        assert!(cx.update(|_, cx| c.read(cx).matches().is_empty()));
        h.update(&mut cx, |s| s.show_panel(Panel::Console));
        keys(&mut cx, "escape");
        add(&h, &mut cx, &["user"]);
        assert!(cx.update(|_, cx| c.read(cx).matches().is_empty()));
    }

    #[gpui::test]
    fn paints_matches_and_the_current_one_brighter(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &["aa aa"]);
        open_bar(&c, &mut cx);
        type_text(&mut cx, "aa");
        let (t, painted) = cx.update(|_, cx| {
            let c = c.read(cx);
            (Theme::global(cx).tokens, c.highlights(0, 5, c.current(cx), None, cx))
        });
        let colours: Vec<_> = painted.iter().map(|(r, s)| (r.clone(), s.background_color)).collect();
        assert_eq!(
            colours,
            vec![
                (0..2, Some(Theme::color(t.found_current))),
                (3..5, Some(Theme::color(t.found))),
            ]
        );
    }
}

mod errors {
    use super::*;

    const LINES: [&str; 4] = [
        "12:00 [main] ERROR first",
        "12:00 [main] INFO fine",
        "12:00 [main] ERROR second",
        "12:00 [main] WARN careful",
    ];

    #[gpui::test]
    fn shows_the_count_and_steps_through_the_errors_wrapping(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &LINES);
        assert_eq!(cx.update(|_, cx| c.read(cx).errors().to_vec()), vec![0, 2]);
        assert_eq!(error_count(&c, &mut cx), "2 errors");
        c.update(&mut cx, |c, cx| c.step_error(true, cx));
        assert_eq!(error_count(&c, &mut cx), "1 of 2 errors");
        c.update(&mut cx, |c, cx| c.step_error(true, cx));
        assert_eq!(error_count(&c, &mut cx), "2 of 2 errors");
        c.update(&mut cx, |c, cx| c.step_error(true, cx));
        assert_eq!(error_count(&c, &mut cx), "1 of 2 errors");
        c.update(&mut cx, |c, cx| c.step_error(false, cx));
        assert_eq!(error_count(&c, &mut cx), "2 of 2 errors");
    }

    #[gpui::test]
    fn leaves_no_error_current_after_a_new_run(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &LINES);
        c.update(&mut cx, |c, cx| c.step_error(true, cx));
        clear(&h, &mut cx);
        add(&h, &mut cx, &LINES);
        assert_eq!(error_count(&c, &mut cx), "2 errors");
    }

    #[gpui::test]
    fn counts_one_error_in_the_singular(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &[LINES[0]]);
        assert_eq!(error_count(&c, &mut cx), "1 error");
        c.update(&mut cx, |c, cx| c.step_error(true, cx));
        assert_eq!(error_count(&c, &mut cx), "1 of 1 error");
    }

    #[gpui::test]
    fn paints_the_current_errors_line(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &LINES);
        c.update(&mut cx, |c, cx| c.step_error(true, cx));
        let (t, painted) = cx.update(|_, cx| {
            let c = c.read(cx);
            (
                Theme::global(cx).tokens,
                c.highlights(0, LINES[0].len(), None, c.current_error(cx), cx),
            )
        });
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].0, 0..LINES[0].len());
        assert_eq!(painted[0].1.background_color, Some(Theme::color(t.failed_tint)));
    }

    #[gpui::test]
    fn has_no_group_and_steps_nowhere_without_errors(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &[LINES[1], LINES[3]]);
        assert!(cx.update(|_, cx| c.read(cx).errors().is_empty()));
        assert!(cx.update(|_, cx| Console::errors_group(&c, cx).is_none()));
        c.update(&mut cx, |c, cx| c.step_error(true, cx));
        assert_eq!(cx.update(|_, cx| c.read(cx).current_error(cx)), None);
    }

    #[gpui::test]
    fn belongs_to_the_console_tab(cx: &mut TestAppContext) {
        let (h, mut cx, c) = setup(cx);
        add(&h, &mut cx, &LINES);
        h.update(&mut cx, |s| s.show_panel(Panel::History));
        assert!(cx.update(|_, cx| c.read(cx).errors().is_empty()));
        h.update(&mut cx, |s| s.show_panel(Panel::Console));
        assert_eq!(cx.update(|_, cx| c.read(cx).errors().len()), 2);
    }
}
