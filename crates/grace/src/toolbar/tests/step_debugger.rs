//! `Toolbar.test.tsx`'s "toolbar: step debugger" (step debugger R6): the controls in place of Run all. F5, F10 and F11
//! work anywhere in the window, so their tests are `grace::window::tests`'.
use super::*;

const PAUSED_IDS: [&str; 3] = ["debug-continue", "debug-over", "debug-into"];

/// A debug run going on, paused or not.
fn debugging(paused: bool) -> AppState {
    let mut s = idle();
    s.run.active = true;
    s.run.debug = true;
    if paused {
        s.paused = Some(crate::testing::paused("s:a.feature#0"));
    }
    s
}

fn shows(cx: &mut VisualTestContext) -> Vec<&'static str> {
    ["run-all-button", "debug-pause"]
        .into_iter()
        .chain(PAUSED_IDS)
        .filter(|id| cx.debug_bounds(id).is_some())
        .collect()
}

/// 31 (SD-20): a normal run keeps Run all; a debug run has Pause in its place until it pauses, and Cancel stays.
#[gpui::test]
fn a_normal_run_keeps_run_all_and_a_debug_run_shows_pause_until_it_pauses(cx: &mut TestAppContext) {
    let mut normal = debugging(false);
    normal.run.debug = false;
    let (h, mut cx) = toolbar(cx, WIDE, normal);
    assert_eq!(shows(&mut cx), ["run-all-button"]);
    h.set(&mut cx, debugging(false));
    assert_eq!(shows(&mut cx), ["debug-pause"]);
    assert!(at(&mut cx, "cancel-button").right() <= at(&mut cx, "debug-pause").left());
    click(&mut cx, "debug-pause");
    assert_eq!(h.backend.calls(), ["debug_command Pause"]);
}

/// 32 (R6.3): paused, Continue, Step over and Step into in that order after Cancel, each sending its command; Pause is
/// back once the run goes on, and Run all once it ends.
#[gpui::test]
fn paused_it_has_continue_step_over_and_step_into_in_place_of_run_all(cx: &mut TestAppContext) {
    let (h, mut cx) = toolbar(cx, WIDE, debugging(true));
    assert_eq!(shows(&mut cx), PAUSED_IDS);
    let mut right = at(&mut cx, "cancel-button").right();
    for id in PAUSED_IDS {
        let b = at(&mut cx, id);
        assert!(right <= b.left(), "{id} {b:?}");
        right = b.right();
        click(&mut cx, id);
    }
    assert_eq!(
        h.backend.calls(),
        ["debug_command Continue", "debug_command Over", "debug_command Into"]
    );
    h.set(&mut cx, debugging(false));
    assert_eq!(shows(&mut cx), ["debug-pause"]);
    h.set(&mut cx, idle());
    assert_eq!(shows(&mut cx), ["run-all-button"]);
}

/// SD-23 (R6.4): at 760 px the controls are 28 px icons, whole inside the window, with their labels as tooltips; wide,
/// they show their labels and no tooltip.
#[gpui::test]
fn at_760_px_the_debug_controls_are_icons_inside_the_window_that_say_their_label(cx: &mut TestAppContext) {
    let (h, mut cx) = toolbar(cx, 760., debugging(true));
    let width = px(760.);
    for (id, label) in PAUSED_IDS.into_iter().zip(["Continue", "Step over", "Step into"]) {
        let b = at(&mut cx, id);
        assert_eq!((b.size.width, b.size.height), (px(28.), px(28.)), "{id}");
        assert!(b.left() >= px(0.) && b.right() <= width, "{id} {b:?}");
        assert_eq!(hover(&h, &mut cx, id).as_deref(), Some(label));
    }
    assert!(at(&mut cx, "cancel-button").right() <= width);
    h.set(&mut cx, debugging(false));
    assert_eq!(at(&mut cx, "debug-pause").size.width, px(28.));
    assert_eq!(hover(&h, &mut cx, "debug-pause").as_deref(), Some("Pause"));
    cx.simulate_resize(gpui::size(px(WIDE), px(700.)));
    h.set(&mut cx, debugging(true));
    for id in PAUSED_IDS {
        assert!(at(&mut cx, id).size.width > px(40.), "{id} shows its label");
        assert_eq!(hover(&h, &mut cx, id), None);
    }
}
