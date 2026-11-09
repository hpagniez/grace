//! `EnvPicker.test.tsx`'s and `Toolbar.test.tsx`'s tests of the environment box.
use super::*;
use crate::testing::{Harness, open_with, project};
use gpui::{Modifiers, TestAppContext, VisualTestContext};
use grace_core::model::env::PropertyListsView;
use grace_core::model::node::Layout;

fn view(env: Option<&str>, suggestions: &[&str], overlays: &[&str]) -> EnvView {
    EnvView {
        env: env.map(String::from),
        suggestions: suggestions.iter().map(|s| s.to_string()).collect(),
        overlays: overlays.iter().map(|s| s.to_string()).collect(),
        properties: PropertyListsView::default(),
        secrets_available: true,
    }
}

mod words {
    use super::*;

    #[test]
    fn hints_only_at_an_environment_without_its_overlay_file_in_any_letter_case() {
        assert_eq!(hint(&view(None, &[], &[])), None);
        assert_eq!(hint(&view(Some("QA"), &[], &["qa"])), None);
        assert_eq!(
            hint(&view(Some("staging"), &[], &["qa"])).as_deref(),
            Some("No karate-config-staging.js found. Fine if karate-config.js handles it.")
        );
    }

    #[test]
    fn suggests_none_then_the_environments_that_hold_the_text() {
        let v = view(None, &["prod", "qa"], &[]);
        assert_eq!(suggestions(&v, ""), ["(none)", "prod", "qa"]);
        assert_eq!(suggestions(&v, "Q"), ["qa"]);
        assert_eq!(suggestions(&v, "no"), ["(none)"]);
        assert!(suggestions(&v, "x").is_empty());
    }

    #[test]
    fn a_commit_sends_a_new_name_once_and_refuses_a_bad_one() {
        assert_eq!(commit(" qa ", None, None), Commit::Send(Some("qa".into())));
        assert_eq!(commit("", Some("qa"), None), Commit::Send(None));
        assert_eq!(commit(NONE, Some("qa"), None), Commit::Send(None));
        assert_eq!(commit("qa", Some("qa"), None), Commit::Restore);
        assert_eq!(commit("", None, None), Commit::Restore);
        assert_eq!(commit("qa", None, Some(&Some("qa".into()))), Commit::Pending);
        assert_eq!(commit("-qa", None, None), Commit::Invalid);
        assert_eq!(commit(&"a".repeat(65), None, None), Commit::Invalid);
    }
}

struct Box_ {
    h: Harness,
    picker: Entity<EnvPicker>,
}

fn open(cx: &mut TestAppContext, state: AppState) -> (Box_, VisualTestContext) {
    let slot: Rc<std::cell::RefCell<Option<Entity<EnvPicker>>>> = Rc::default();
    let keep = slot.clone();
    let (h, cx) = open_with(cx, 1400., state, move |grace, window, cx| {
        let picker = cx.new(|cx| EnvPicker::new(grace.clone(), window, cx));
        *keep.borrow_mut() = Some(picker.clone());
        Box::new(move |_, _, _| div().p_4().child(picker.clone()).into_any_element())
    });
    let picker = slot.borrow().clone().unwrap();
    let mut cx = cx;
    // a box hears its focus only in the active window
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (Box_ { h, picker }, cx)
}

fn with_env(env: Option<&str>) -> AppState {
    let mut s = project("D:/p", Layout::Standalone, 2);
    s.env = Some(view(env, &["prod", "qa"], &["prod", "qa"]));
    s.env_generation = 1;
    s
}

impl Box_ {
    fn text(&self, cx: &mut VisualTestContext) -> String {
        cx.update(|_, cx| self.picker.read(cx).input.read(cx).value().to_string())
    }

    fn placeholder(&self, cx: &mut VisualTestContext) -> String {
        cx.update(|_, cx| placeholder(&self.h.grace.read(cx).state))
    }

    fn focus(&self, cx: &mut VisualTestContext) {
        let input = cx.update(|_, cx| self.picker.read(cx).input.clone());
        cx.update(|window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        // focus listeners run when the next frame draws
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }

    fn blur(&self, cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.blur(cx));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }

    fn type_text(&self, cx: &mut VisualTestContext, text: &str) {
        cx.simulate_input(text);
        cx.run_until_parked();
    }

    fn key(&self, cx: &mut VisualTestContext, key: &str) {
        cx.simulate_keystrokes(key);
        cx.run_until_parked();
    }

    fn sent(&self) -> Vec<String> {
        self.h
            .backend
            .calls()
            .into_iter()
            .filter(|c| c.starts_with("set_env"))
            .collect()
    }

    /// The runner's answer: the environment in force, as a new view.
    fn echo(&self, cx: &mut VisualTestContext, env: Option<&str>) {
        self.h.update(cx, |s| {
            s.env = Some(view(env, &["prod", "qa"], &["prod", "qa"]));
            s.env_generation += 1;
        });
    }

    fn disabled(&self, cx: &mut VisualTestContext) -> bool {
        cx.update(|_, cx| EnvPicker::disabled(&self.h.grace.read(cx).state))
    }
}

#[gpui::test]
fn shows_the_environment_and_none_as_an_empty_box_with_a_placeholder(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(Some("qa")));
    assert_eq!(b.text(&mut cx), "qa");
    b.echo(&mut cx, None);
    assert_eq!(b.text(&mut cx), "");
    assert_eq!(b.placeholder(&mut cx), NONE);
}

#[gpui::test]
fn is_disabled_with_no_view_and_while_busy_or_running(cx: &mut TestAppContext) {
    let mut s = with_env(None);
    s.env = None;
    let (b, mut cx) = open(cx, s);
    assert!(b.disabled(&mut cx));
    b.echo(&mut cx, None);
    assert!(!b.disabled(&mut cx));
    b.h.update(&mut cx, |s| s.busy = true);
    assert!(b.disabled(&mut cx));
    b.h.update(&mut cx, |s| {
        s.busy = false;
        s.run.active = true;
    });
    assert!(b.disabled(&mut cx));
}

#[gpui::test]
fn focus_empties_the_box_with_the_environment_as_placeholder_and_leaving_it_untouched_puts_it_back(
    cx: &mut TestAppContext,
) {
    let (b, mut cx) = open(cx, with_env(Some("qa")));
    b.focus(&mut cx);
    assert_eq!(b.text(&mut cx), "");
    assert_eq!(b.placeholder(&mut cx), "qa");
    b.blur(&mut cx);
    assert_eq!(b.text(&mut cx), "qa");
    assert!(b.sent().is_empty());
}

#[gpui::test]
fn enter_sends_a_typed_name_once_until_the_runner_answers(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(None));
    b.focus(&mut cx);
    b.type_text(&mut cx, "staging");
    assert!(b.sent().is_empty(), "typing alone sends nothing");
    b.key(&mut cx, "enter");
    assert_eq!(b.sent(), ["set_env staging"]);
    b.key(&mut cx, "enter");
    b.blur(&mut cx);
    assert_eq!(b.sent(), ["set_env staging"], "on its way already");
    b.echo(&mut cx, Some("staging"));
    assert_eq!(b.text(&mut cx), "staging");
}

#[gpui::test]
fn leaving_a_typed_box_commits_it_and_none_sends_no_environment(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(Some("qa")));
    b.focus(&mut cx);
    b.type_text(&mut cx, NONE);
    b.blur(&mut cx);
    assert_eq!(b.sent(), ["set_env null"]);
}

#[gpui::test]
fn a_bad_name_says_why_and_sends_nothing(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(None));
    b.focus(&mut cx);
    b.type_text(&mut cx, "bad name");
    b.key(&mut cx, "enter");
    assert!(b.sent().is_empty());
    assert!(cx.debug_bounds("env-error").is_some());
    b.type_text(&mut cx, "x");
    assert!(cx.debug_bounds("env-error").is_none(), "typing clears the error");
}

#[gpui::test]
fn a_refused_change_puts_the_runners_environment_back(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(Some("qa")));
    b.focus(&mut cx);
    b.type_text(&mut cx, "prod");
    b.key(&mut cx, "enter");
    // refused: the runner re-sends the environment in force
    b.echo(&mut cx, Some("qa"));
    assert_eq!(b.text(&mut cx), "qa");
}

#[gpui::test]
fn escape_puts_the_environment_back(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(Some("qa")));
    b.focus(&mut cx);
    b.type_text(&mut cx, "pro");
    b.key(&mut cx, "escape");
    assert_eq!(b.text(&mut cx), "qa");
    b.blur(&mut cx);
    assert!(b.sent().is_empty());
}

#[gpui::test]
fn the_suggestions_show_while_the_box_has_the_focus_and_follow_the_text(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(None));
    assert!(cx.debug_bounds("env-suggestions").is_none());
    b.focus(&mut cx);
    assert!(cx.debug_bounds("env-option-2").is_some(), "(none), prod and qa");
    b.type_text(&mut cx, "q");
    assert!(cx.debug_bounds("env-option-0").is_some());
    assert!(cx.debug_bounds("env-option-1").is_none(), "only qa holds q");
    b.blur(&mut cx);
    assert!(cx.debug_bounds("env-suggestions").is_none());
}

#[gpui::test]
fn the_arrow_keys_and_enter_pick_a_suggestion(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(None));
    b.focus(&mut cx);
    b.key(&mut cx, "down");
    b.key(&mut cx, "down");
    b.key(&mut cx, "enter");
    assert_eq!(b.sent(), ["set_env prod"]);
    assert_eq!(b.text(&mut cx), "prod");
    assert!(cx.debug_bounds("env-suggestions").is_none(), "a pick closes them");
}

#[gpui::test]
fn a_click_on_a_suggestion_picks_it(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(None));
    b.focus(&mut cx);
    b.type_text(&mut cx, "q");
    // only qa holds a q; a press on it must not send the q typed
    let qa = cx.debug_bounds("env-option-0").unwrap();
    cx.simulate_click(qa.center(), Modifiers::default());
    cx.run_until_parked();
    b.blur(&mut cx);
    assert_eq!(b.sent(), ["set_env qa"], "sent once, the blur after it included");
}

#[gpui::test]
fn says_when_the_environment_has_no_overlay_file(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(Some("qa")));
    assert!(cx.debug_bounds("env-hint").is_none());
    b.echo(&mut cx, Some("staging"));
    assert!(cx.debug_bounds("env-hint").is_some());
}

#[gpui::test]
fn below_1280_px_the_word_env_goes_from_sight_and_says_itself_in_a_tooltip(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(None));
    let wide = cx.debug_bounds("env-input").unwrap();
    cx.simulate_resize(gpui::size(px(1000.), px(700.)));
    cx.run_until_parked();
    let narrow = cx.debug_bounds("env-input").unwrap();
    assert!(narrow.origin.x < wide.origin.x, "the word no longer takes room");
    cx.simulate_mouse_move(narrow.center(), None, Modifiers::none());
    cx.executor()
        .advance_clock(crate::tooltip::DELAY + std::time::Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(b.h.tip(&mut cx).as_deref(), Some(LABEL));
}

#[gpui::test]
fn a_press_elsewhere_commits_what_was_typed_before_its_click(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, with_env(None));
    b.focus(&mut cx);
    b.type_text(&mut cx, "staging");
    cx.simulate_mouse_down(
        gpui::point(px(1000.), px(600.)),
        gpui::MouseButton::Left,
        Modifiers::none(),
    );
    assert_eq!(b.sent(), ["set_env staging"]);
}
