//! `Threads.test.tsx`'s tests, and `Toolbar.test.tsx`'s of the threads box.
use super::*;
use crate::testing::{Harness, open_with, project};
use gpui::{Modifiers, TestAppContext, VisualTestContext};
use grace_core::model::node::Layout;
use grace_core::state::ThreadsView;
use serde_json::json;

mod words {
    use super::*;

    #[test]
    fn hands_the_runner_a_number_as_written_and_null_for_anything_else() {
        assert_eq!(value("16"), json!(16));
        assert_eq!(value(" 2.5 "), json!(2.5));
        assert_eq!(value("0"), json!(0));
        assert_eq!(value(""), Value::Null);
        assert_eq!(value("1e"), Value::Null);
    }

    #[test]
    fn steps_by_one_between_1_and_32() {
        assert_eq!(step("3", true), Some(4));
        assert_eq!(step("3", false), Some(2));
        assert_eq!(step("32", true), None);
        assert_eq!(step("1", false), None);
        assert_eq!(step("2.5", true), Some(3));
        assert_eq!(step("2.5", false), Some(2));
        assert_eq!(step("40", false), Some(32));
        assert_eq!(step("", true), Some(1));
    }
}

struct Box_ {
    h: Harness,
    threads: Entity<ThreadsBox>,
}

fn open(cx: &mut TestAppContext, state: AppState) -> (Box_, VisualTestContext) {
    let slot: Rc<std::cell::RefCell<Option<Entity<ThreadsBox>>>> = Rc::default();
    let keep = slot.clone();
    let (h, mut cx) = open_with(cx, 1400., state, move |grace, window, cx| {
        let threads = cx.new(|cx| ThreadsBox::new(grace.clone(), window, cx));
        *keep.borrow_mut() = Some(threads.clone());
        Box::new(move |_, _, _| div().p_4().child(threads.clone()).into_any_element())
    });
    let threads = slot.borrow().clone().unwrap();
    // a box hears its focus only in the active window
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (Box_ { h, threads }, cx)
}

fn loaded(count: u32) -> AppState {
    let mut s = project("D:/p", Layout::Standalone, 2);
    s.threads = Some(ThreadsView { count, generation: 0 });
    s
}

impl Box_ {
    fn text(&self, cx: &mut VisualTestContext) -> String {
        cx.update(|_, cx| self.threads.read(cx).input.read(cx).value().to_string())
    }

    fn disabled(&self, cx: &mut VisualTestContext) -> bool {
        cx.update(|_, cx| {
            let s = &self.h.grace.read(cx).state;
            s.run.active || s.busy || s.threads.is_none()
        })
    }

    fn focus(&self, cx: &mut VisualTestContext) {
        let input = cx.update(|_, cx| self.threads.read(cx).input.clone());
        cx.update(|window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }

    fn blur(&self, cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.blur(cx));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }

    /// Selects what the box holds and types `text` over it.
    fn type_over(&self, cx: &mut VisualTestContext, text: &str) {
        cx.simulate_keystrokes("ctrl-a");
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
            .filter(|c| c.starts_with("set_threads"))
            .collect()
    }

    /// The runner's answer: the count in force, as a new `threads` event.
    fn echo(&self, cx: &mut VisualTestContext, count: u32) {
        self.h.update(cx, |s| {
            let generation = s.threads.map_or(0, |t| t.generation + 1);
            s.threads = Some(ThreadsView { count, generation });
        });
    }
}

#[gpui::test]
fn shows_the_runners_count_and_1_disabled_with_no_project(cx: &mut TestAppContext) {
    let mut s = loaded(1);
    s.threads = None;
    let (b, mut cx) = open(cx, s);
    assert!(b.disabled(&mut cx));
    assert_eq!(b.text(&mut cx), "1");
    b.echo(&mut cx, 3);
    assert!(!b.disabled(&mut cx));
    assert_eq!(b.text(&mut cx), "3");
}

#[gpui::test]
fn sends_a_committed_value_once(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(1));
    b.focus(&mut cx);
    b.type_over(&mut cx, "16");
    assert!(b.sent().is_empty(), "typing alone sends nothing");
    b.key(&mut cx, "enter");
    assert_eq!(b.sent(), ["set_threads 16"]);
    b.blur(&mut cx);
    assert_eq!(b.sent(), ["set_threads 16"], "leaving after it sends nothing more");
}

#[gpui::test]
fn leaving_the_box_commits_it(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(1));
    b.focus(&mut cx);
    b.type_over(&mut cx, "2");
    b.blur(&mut cx);
    assert_eq!(b.sent(), ["set_threads 2"]);
}

#[gpui::test]
fn a_refused_value_stays_until_the_runner_resends_its_count(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(2));
    b.focus(&mut cx);
    b.type_over(&mut cx, "0");
    b.key(&mut cx, "enter");
    assert_eq!(b.sent(), ["set_threads 0"]);
    assert_eq!(b.text(&mut cx), "0");
    b.echo(&mut cx, 2);
    assert_eq!(b.text(&mut cx), "2");
}

#[gpui::test]
fn sends_null_for_an_empty_box_and_leaves_the_check_to_the_runner(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(2));
    b.focus(&mut cx);
    b.key(&mut cx, "ctrl-a");
    b.key(&mut cx, "backspace");
    b.key(&mut cx, "enter");
    assert_eq!(b.sent(), ["set_threads null"]);
}

#[gpui::test]
fn takes_only_what_a_number_box_takes(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(2));
    b.focus(&mut cx);
    b.type_over(&mut cx, "abc");
    assert_eq!(b.text(&mut cx), "2");
}

#[gpui::test]
fn an_arrow_key_steps_and_sends_at_once(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(3));
    b.focus(&mut cx);
    b.key(&mut cx, "up");
    assert_eq!(b.text(&mut cx), "4");
    assert_eq!(b.sent(), ["set_threads 4"]);
    b.key(&mut cx, "down");
    assert_eq!(b.sent(), ["set_threads 4", "set_threads 3"]);
    b.blur(&mut cx);
    assert_eq!(b.sent().len(), 2, "a step is committed already");
}

#[gpui::test]
fn disabled_while_busy_and_sends_nothing(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(2));
    b.h.update(&mut cx, |s| s.busy = true);
    assert!(b.disabled(&mut cx));
    b.focus(&mut cx);
    b.key(&mut cx, "up");
    assert!(b.sent().is_empty());
    b.h.update(&mut cx, |s| s.busy = false);
    assert!(!b.disabled(&mut cx));
}

#[gpui::test]
fn a_new_project_shows_its_own_count(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(3));
    // a new project: no count until the runner sends its own
    b.h.update(&mut cx, |s| s.threads = None);
    assert!(b.disabled(&mut cx));
    b.h.update(&mut cx, |s| {
        s.threads = Some(ThreadsView {
            count: 1,
            generation: 0,
        })
    });
    assert_eq!(b.text(&mut cx), "1");
    assert!(!b.disabled(&mut cx));
}

#[gpui::test]
fn below_1280_px_the_word_threads_goes_from_sight_and_says_itself_in_a_tooltip(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(1));
    let wide = cx.debug_bounds("threads-input").unwrap();
    cx.simulate_resize(gpui::size(px(1000.), px(700.)));
    cx.run_until_parked();
    let narrow = cx.debug_bounds("threads-input").unwrap();
    assert!(narrow.origin.x < wide.origin.x, "the word no longer takes room");
    cx.simulate_mouse_move(narrow.center(), None, Modifiers::none());
    cx.executor()
        .advance_clock(crate::tooltip::DELAY + std::time::Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(b.h.tip(&mut cx).as_deref(), Some(LABEL));
}

#[gpui::test]
fn a_press_elsewhere_commits_before_its_click(cx: &mut TestAppContext) {
    let (b, mut cx) = open(cx, loaded(1));
    b.focus(&mut cx);
    b.type_over(&mut cx, "5");
    // the press alone, before the focus moves at the next frame: the click it starts must see the count
    cx.simulate_mouse_down(
        gpui::point(px(600.), px(600.)),
        gpui::MouseButton::Left,
        Modifiers::none(),
    );
    assert_eq!(b.sent(), ["set_threads 5"]);
}

#[gpui::test]
fn a_new_box_shows_the_count_in_force(cx: &mut TestAppContext) {
    // Electron Grace's reloaded window asked again; a new box reads the state the runner left
    let (b, mut cx) = open(cx, loaded(7));
    assert_eq!(b.text(&mut cx), "7");
    assert!(!b.disabled(&mut cx));
}
