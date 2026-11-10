//! Tooltips and cut text, as Electron Grace's `Tooltip.tsx` and `Cut.tsx` (premium UI spec R9.4; parity PU-19 to
//! PU-21). A tooltip shows after the pointer rests 500 ms on its target, at once when the target gets the focus from
//! the keyboard, and one at a time. Leaving, a press of the target, a click on the tooltip and Escape close it; Escape
//! closes it before anything else sees the key, so in a dialog the first Escape closes the tooltip and the second the
//! dialog. The tooltip goes `below` its target unless told otherwise; in a row, `left`, `right` or `over` its own cut
//! text, so it never lies on the next row.
//!
//! Each window has one [`Tips`], made by [`install`] and drawn by the window's view. A trigger is a [`Tip`] around its
//! target: the wrapper, not the target, listens, so a disabled button still shows its tooltip. Nothing here runs while
//! idle: the one timer is the 500 ms pause after the pointer comes to rest on a target.
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, AppContext as _, Bounds, Context, DispatchPhase, ElementId, Entity, FocusHandle, Global,
    InteractiveElement as _, IntoElement, MouseButton, MouseMoveEvent, ParentElement as _, Pixels, Render, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Task, WeakEntity, Window, WindowId,
    canvas, deferred, div, point, px,
};
use gpui_base::{ElementExt as _, Placement, Positioner};

use crate::theme::{METRICS, SANS, Theme};

/// How long the pointer rests on a target before its tooltip shows.
pub const DELAY: Duration = Duration::from_millis(500);
/// Between a tooltip and its target, and its least distance from the window's edges (`placeTip.ts`).
const GAP: f32 = 6.;
const EDGE: f32 = 8.;
/// Above dialogs and popovers, as GPUI Kit's own tooltips.
const PRIORITY: usize = 200;

/// Where a tooltip goes (`placeTip.ts`): below its target, or above when there is no room; beside it, flipping when
/// there is no room; or over it, on its top left, for a cut line in a row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    #[default]
    Below,
    Left,
    Right,
    Over,
}

/// The open tooltip: its text, the trigger it belongs to and where that trigger is.
#[derive(Clone)]
struct Open {
    trigger: ElementId,
    text: SharedString,
    target: Bounds<Pixels>,
    side: Side,
}

/// A window's tooltip: at most one open, at most one waiting for its pause.
pub struct Tips {
    open: Option<Open>,
    pending: Option<Task<()>>,
    /// The trigger last pressed: a press often redraws the target under a pointer that has not moved, and the
    /// hover that brings must not bring the tooltip back. Cleared when the pointer leaves it.
    pressed: Option<ElementId>,
    /// The trigger that holds the keyboard focus and has shown its tooltip for it: once only per focus, so the
    /// tooltip Escape closed does not come back while the focus stays.
    focused: Option<ElementId>,
    /// Where the open tooltip was drawn, so the pointer can move onto it without closing it.
    drawn: Rc<Cell<Option<Bounds<Pixels>>>>,
    _escape: Subscription,
}

/// The `Tips` of each window, for the triggers to find.
#[derive(Default)]
struct Windows(HashMap<WindowId, WeakEntity<Tips>>);

impl Global for Windows {}

/// Makes the window's `Tips`: the view of the window draws it as a child. Escape closes an open tooltip before any
/// other handler of the window sees the key.
pub fn install(window: &mut Window, cx: &mut App) -> Entity<Tips> {
    let id = window.window_handle().window_id();
    let tips = cx.new(|cx: &mut Context<Tips>| {
        let me = cx.weak_entity();
        let escape = cx.intercept_keystrokes(move |event, window, cx| {
            if event.keystroke.key != "escape" || window.window_handle().window_id() != id {
                return;
            }
            let Some(tips) = me.upgrade() else { return };
            if tips.read(cx).open.is_some() {
                tips.update(cx, |tips, cx| tips.hide(cx));
                cx.stop_propagation();
            }
        });
        Tips {
            open: None,
            pending: None,
            pressed: None,
            focused: None,
            drawn: Rc::default(),
            _escape: escape,
        }
    });
    cx.default_global::<Windows>().0.insert(id, tips.downgrade());
    tips
}

fn tips_of(window: &Window, cx: &App) -> Option<Entity<Tips>> {
    cx.try_global::<Windows>()?
        .0
        .get(&window.window_handle().window_id())?
        .upgrade()
}

impl Tips {
    /// The text of the open tooltip, if one is open.
    pub fn text(&self) -> Option<SharedString> {
        self.open.as_ref().map(|o| o.text.clone())
    }

    /// Closes the open tooltip and forgets one waiting for its pause.
    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.pending = None;
        if self.open.take().is_some() {
            self.drawn.set(None);
            cx.notify();
        }
    }

    fn show(&mut self, open: Open, cx: &mut Context<Self>) {
        self.pending = None;
        self.open = Some(open);
        cx.notify();
    }

    fn is_open_for(&self, trigger: &ElementId) -> bool {
        self.open.as_ref().is_some_and(|o| &o.trigger == trigger)
    }

    fn hover(&mut self, open: Open, window: &mut Window, cx: &mut Context<Self>) {
        if self.pressed.as_ref() == Some(&open.trigger) || self.is_open_for(&open.trigger) {
            return;
        }
        self.pending = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(DELAY).await;
            let _ = this.update(cx, |this, cx| this.show(open, cx));
        }));
    }

    fn leave(&mut self, trigger: &ElementId, window: &Window, cx: &mut Context<Self>) {
        if self.pressed.as_ref() == Some(trigger) {
            self.pressed = None;
        }
        self.pending = None;
        let onto_tip = self.drawn.get().is_some_and(|b| b.contains(&window.mouse_position()));
        if self.is_open_for(trigger) && !onto_tip {
            self.hide(cx);
        }
    }

    fn press(&mut self, trigger: ElementId, cx: &mut Context<Self>) {
        self.pressed = Some(trigger);
        self.hide(cx);
    }

    /// Keeps the tooltip in step with the trigger's keyboard focus: shown once when it arrives, closed when it goes.
    fn focus(&mut self, open: Open, focused: bool, cx: &mut Context<Self>) {
        let mine = self.focused.as_ref() == Some(&open.trigger);
        if focused && !mine {
            self.focused = Some(open.trigger.clone());
            self.show(open, cx);
        } else if !focused && mine {
            self.focused = None;
            if self.is_open_for(&open.trigger) {
                self.hide(cx);
            }
        }
    }
}

impl Render for Tips {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(open) = self.open.clone() else {
            return div().into_any_element();
        };
        let t = Theme::global(cx).tokens;
        let m = METRICS;
        let positioner = match open.side {
            Side::Over => Positioner::corner(gpui::Anchor::TopLeft, open.target.origin),
            side => Positioner::side(open.target)
                .placement(match side {
                    Side::Left => Placement::Left,
                    Side::Right => Placement::Right,
                    _ => Placement::Bottom,
                })
                .offset(px(GAP)),
        }
        .margin(px(EDGE));
        let drawn = self.drawn.clone();
        let target = open.target;
        let over = open.side == Side::Over;
        let me = cx.weak_entity();
        let leave = me.clone();
        let moved = me.clone();
        let from_keyboard = self.focused.as_ref() == Some(&open.trigger);
        let shown_at = self.drawn.clone();
        let shadow = m
            .shadow_float
            .iter()
            .zip(t.shadow_float)
            .map(|(&(y, blur), colour)| gpui::BoxShadow {
                color: Theme::color_alpha(colour),
                offset: point(px(0.), px(y)),
                blur_radius: px(blur),
                spread_radius: px(0.),
                inset: false,
            })
            .collect::<Vec<_>>();
        let _ = window;
        deferred(
            positioner.child(
                div()
                    .id("grace-tooltip")
                    .role(Role::Tooltip)
                    .aria_label(open.text.clone())
                    .max_w(px(320.))
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(m.radius))
                    .border_1()
                    .border_color(Theme::color(t.line))
                    .bg(Theme::color(t.raised))
                    .shadow(shadow)
                    .font_family(SANS)
                    .text_size(px(m.text_xs.size))
                    .line_height(px(m.text_xs.line_height))
                    .text_color(Theme::color(t.ink))
                    .on_prepaint(move |bounds, _, _| drawn.set(Some(bounds)))
                    .on_hover(move |hovered, window, cx| {
                        // leaving the tooltip for anything but its target closes it
                        if !*hovered && !target.contains(&window.mouse_position()) {
                            let _ = leave.update(cx, |tips, cx| tips.hide(cx));
                        }
                    })
                    // A click on it closes it rather than reaching what lies under it; one over its target lies on
                    // it, so a click within the target's box is the target's.
                    .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                        let _ = me.update(cx, |tips, cx| tips.hide(cx));
                        if !(over && target.contains(&event.position)) {
                            cx.stop_propagation();
                        }
                    })
                    .child(div().id("grace-tooltip-text").child(open.text))
                    // A pointer anywhere but on the target or the tooltip closes one it opened: its target may have
                    // gone from under it, a row scrolled away or a view closed, with no leave to tell.
                    .child(
                        canvas(
                            |_, _, _| {},
                            move |_, _, window, _| {
                                let moved = moved.clone();
                                let shown_at = shown_at.clone();
                                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                                    let on_tip = shown_at.get().is_some_and(|b| b.contains(&event.position));
                                    if phase == DispatchPhase::Bubble
                                        && !from_keyboard
                                        && !on_tip
                                        && !target.contains(&event.position)
                                    {
                                        let _ = moved.update(cx, |tips, cx| tips.hide(cx));
                                    }
                                });
                            },
                        )
                        .absolute()
                        .size_0(),
                    ),
            ),
        )
        .with_priority(PRIORITY)
        .into_any_element()
    }
}

/// What a trigger keeps from frame to frame: where it is, and for cut text whether it is cut.
#[derive(Default)]
struct Spot {
    bounds: Rc<Cell<Bounds<Pixels>>>,
    cut: Rc<Cell<bool>>,
}

enum Target {
    Element(AnyElement),
    Focusable(Box<dyn FnOnce(&FocusHandle) -> AnyElement>),
    Cut(SharedString),
    /// A line cut in the middle: `head` gives way, `tail` stays whole (`Cut.tsx`'s `Middle`).
    Middle(SharedString, SharedString),
}

/// A tooltip trigger: the wrapper around its target. Make one with [`tooltip`] or [`cut`].
#[derive(IntoElement)]
pub struct Tip {
    id: ElementId,
    text: Option<SharedString>,
    side: Side,
    target: Target,
    /// Whether the target holds the keyboard focus, for one that takes no focus of its own (`Tip::keyboard`).
    keyboard: Option<bool>,
}

/// A tooltip saying `text` for a target. No text (a disabled control with no reason to give) means no tooltip.
/// Give the target with [`Tip::child`], or with [`Tip::focusable`] for one that shows its tooltip on keyboard focus.
pub fn tooltip(id: impl Into<ElementId>, text: impl Into<Option<SharedString>>) -> Tip {
    Tip {
        id: id.into(),
        text: text.into(),
        side: Side::Below,
        target: Target::Element(div().into_any_element()),
        keyboard: None,
    }
}

/// One line of `text`, cut with an ellipsis when it does not fit, which shows the whole of it in a tooltip only
/// when it is cut, measured as it is drawn, so it follows the window (`Cut.tsx`). The wrapper takes the room it is
/// given: style it to shrink in a row.
pub fn cut(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Tip {
    let text = text.into();
    Tip {
        id: id.into(),
        text: Some(text.clone()),
        side: Side::Below,
        target: Target::Cut(text),
        keyboard: None,
    }
}

/// One line of `head` then `tail`, where only `head` is cut, with an ellipsis at its end, when the two do not fit:
/// the end of a name or a path's last folder stays in sight (`Cut.tsx`'s `Middle`, toolbar spec R1.2, R2.1). The
/// tooltip, shown only while it is cut, says the whole line; a screen reader reads it whole.
pub fn cut_middle(id: impl Into<ElementId>, head: impl Into<SharedString>, tail: impl Into<SharedString>) -> Tip {
    let (head, tail) = (head.into(), tail.into());
    Tip {
        id: id.into(),
        text: Some(format!("{head}{tail}").into()),
        side: Side::Below,
        target: Target::Middle(head, tail),
        keyboard: None,
    }
}

/// Sets `cut` while `text`, in the element's text style, is wider than the element.
fn measure(text: SharedString, cut: Rc<Cell<bool>>) -> impl Fn(Bounds<Pixels>, &mut Window, &mut App) + 'static {
    move |b, window, _| {
        let style = window.text_style();
        let size = style.font_size.to_pixels(window.rem_size());
        let run = style.to_run(text.len());
        let width = window.text_system().shape_line(text.clone(), size, &[run], None).width;
        cut.set(width > b.size.width + px(0.5));
    }
}

impl Tip {
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// What the tooltip of a cut line says in place of the line: more than it, such as a feature's file name and
    /// its title (`Tree.tsx`'s `full`).
    pub fn whole(mut self, text: impl Into<SharedString>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Shows the tooltip as keyboard focus does, while `on` holds: for the active row of a list that is one tab stop,
    /// where the list holds the focus and the row only stands for it (VL-21).
    pub fn keyboard(mut self, on: bool) -> Self {
        self.keyboard = Some(on);
        self
    }

    pub fn child(mut self, target: impl IntoElement) -> Self {
        self.target = Target::Element(target.into_any_element());
        self
    }

    /// A target that takes the focus handle the tooltip watches, such as a button's `track_focus`.
    pub fn focusable<E: IntoElement>(mut self, target: impl FnOnce(&FocusHandle) -> E + 'static) -> Self {
        self.target = Target::Focusable(Box::new(move |focus| target(focus).into_any_element()));
        self
    }
}

impl RenderOnce for Tip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let spot = window.use_keyed_state(self.id.clone(), cx, |_, _| Spot::default());
        let (bounds, cut) = {
            let s = spot.read(cx);
            (s.bounds.clone(), s.cut.clone())
        };
        let focus = matches!(self.target, Target::Focusable(_))
            .then(|| window.use_keyed_state((self.id.clone(), "focus"), cx, |_, cx| cx.focus_handle().tab_stop(true)))
            .map(|f| f.read(cx).clone());
        let is_cut = matches!(self.target, Target::Cut(_) | Target::Middle(..));
        let trigger = self.id.clone();
        let side = self.side;
        let text = self.text.clone();
        // What would show now: a cut line's text only while it is cut.
        let open = {
            let bounds = bounds.clone();
            let cut = cut.clone();
            let trigger = trigger.clone();
            move || {
                let text = text.clone().filter(|_| !is_cut || cut.get())?;
                Some(Open {
                    trigger: trigger.clone(),
                    text,
                    target: bounds.get(),
                    side,
                })
            }
        };

        // Keyboard focus shows it once; the next frame after the focus goes closes it.
        let focused = match &focus {
            Some(focus) => Some(focus.contains_focused(window, cx) && window.last_input_was_keyboard()),
            None => self.keyboard,
        };
        if let (Some(focused), Some(tips)) = (focused, tips_of(window, cx)) {
            let was = tips.read(cx).focused.as_ref() == Some(&trigger);
            if focused != was
                && let Some(open) = open().or_else(|| {
                    (!focused).then(|| Open {
                        trigger: trigger.clone(),
                        text: SharedString::default(),
                        target: bounds.get(),
                        side,
                    })
                })
            {
                window.defer(cx, move |_, cx| {
                    tips.update(cx, |tips, cx| tips.focus(open, focused, cx))
                });
            }
        }

        let target = match self.target {
            Target::Element(e) => e,
            Target::Focusable(build) => build(focus.as_ref().expect("made above")),
            Target::Cut(text) => {
                // read whole by a screen reader, cut or not
                div()
                    .id((trigger.clone(), "line"))
                    .role(Role::Label)
                    // AccessKit reads a label's text from its value
                    .aria_value(text.clone())
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .on_prepaint(measure(text.clone(), cut.clone()))
                    .child(text)
                    .into_any_element()
            }
            Target::Middle(head, tail) => div()
                .id((trigger.clone(), "line"))
                .role(Role::Label)
                .aria_value(SharedString::from(format!("{head}{tail}")))
                .flex()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(
                    div()
                        .id((trigger.clone(), "head"))
                        .min_w_0()
                        .flex_shrink(1.)
                        .overflow_hidden()
                        .text_ellipsis()
                        .on_prepaint(measure(head.clone(), cut.clone()))
                        .child(head),
                )
                .child(div().flex_none().child(tail))
                .into_any_element(),
        };

        let hover = open.clone();
        let left = trigger.clone();
        let under = bounds.clone();
        let pressed = trigger.clone();
        div()
            .id(self.id)
            .flex()
            .min_w_0()
            .on_prepaint(move |b, _, _| bounds.set(b))
            .on_hover(move |hovered, window, cx| {
                let Some(tips) = tips_of(window, cx) else { return };
                if *hovered {
                    if let Some(open) = hover() {
                        tips.update(cx, |tips, cx| tips.hover(open, window, cx));
                    }
                } else if !under.get().contains(&window.mouse_position()) {
                    // GPUI also calls a key press an un-hover, as keyboard input hides hover styles; only a pointer
                    // that left closes the tooltip
                    tips.update(cx, |tips, cx| tips.leave(&left, window, cx));
                }
            })
            // A press of the target: the person acted on it, and the tooltip would lie over what they go to next.
            .on_any_mouse_down(move |_, window, cx| {
                if let Some(tips) = tips_of(window, cx) {
                    tips.update(cx, |tips, cx| tips.press(pressed.clone(), cx));
                }
            })
            .child(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Modifiers, TestAppContext, VisualTestContext, size};

    struct View {
        tips: Option<Entity<Tips>>,
        /// Whether the keyboard stands on the `pinned` target.
        pinned: bool,
        /// Clicks that reached the window's root.
        clicks: Rc<Cell<u32>>,
    }

    impl Render for View {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let tips = self.tips.get_or_insert_with(|| install(window, cx)).clone();
            let at = |y: f32| div().absolute().left(px(0.)).top(px(y));
            let beside = |x: f32, y: f32| div().absolute().left(px(x)).top(px(y));
            let clicks = self.clicks.clone();
            div()
                .id("root")
                .size_full()
                .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
                .child(
                    beside(300., 0.).child(
                        tooltip("left", Some(SharedString::from("Left")))
                            .side(Side::Left)
                            .child(div().w(px(100.)).h(px(30.))),
                    ),
                )
                .child(
                    beside(500., 0.).child(
                        tooltip("right", Some(SharedString::from("Right")))
                            .side(Side::Right)
                            .child(div().w(px(100.)).h(px(30.))),
                    ),
                )
                .child(beside(300., 100.).child(tooltip("none", None).child(div().w(px(100.)).h(px(30.)))))
                .child(
                    beside(300., 200.).child(
                        tooltip("pinned", Some(SharedString::from("Pinned")))
                            .keyboard(self.pinned)
                            .child(div().w(px(100.)).h(px(30.))),
                    ),
                )
                .child(beside(400., 560.).child(
                    tooltip("flip-up", Some(SharedString::from("Flips up"))).child(div().w(px(100.)).h(px(30.))),
                ))
                .child(
                    beside(760., 450.).child(
                        tooltip("clamp", Some(SharedString::from("Kept inside the window")))
                            .child(div().w(px(30.)).h(px(30.))),
                    ),
                )
                .child(
                    beside(10., 500.).child(
                        tooltip("flip-right", Some(SharedString::from("Flips right")))
                            .side(Side::Left)
                            .child(div().w(px(30.)).h(px(24.))),
                    ),
                )
                .child(
                    beside(300., 300.)
                        .w(px(60.))
                        .child(cut("over", "A long line that is cut over its row").side(Side::Over)),
                )
                .child(
                    at(0.).child(
                        tooltip("refresh", Some(SharedString::from("Refresh the tree")))
                            .child(div().id("target").w(px(100.)).h(px(30.))),
                    ),
                )
                .child(
                    at(100.).child(
                        tooltip("save", Some(SharedString::from("Save")))
                            .focusable(|focus| div().id("save-target").track_focus(focus).w(px(100.)).h(px(30.))),
                    ),
                )
                .child(
                    at(200.).child(
                        tooltip("second", Some(SharedString::from("Second"))).child(div().w(px(100.)).h(px(30.))),
                    ),
                )
                .child(
                    at(300.)
                        .w(px(60.))
                        .child(cut("long", "A very long project name that is cut")),
                )
                .child(at(400.).w(px(300.)).child(cut("short", "Hi")))
                .child(tips)
        }
    }

    fn open(cx: &mut TestAppContext) -> (Entity<View>, VisualTestContext) {
        cx.update(|cx| {
            crate::init(cx);
        });
        let window = cx.add_window(|_, _| View {
            tips: None,
            pinned: false,
            clicks: Rc::default(),
        });
        let view = window.root(cx).unwrap();
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.simulate_resize(size(px(800.), px(600.)));
        vcx.run_until_parked();
        (view, vcx)
    }

    fn shown(view: &Entity<View>, cx: &mut VisualTestContext) -> Option<SharedString> {
        cx.update(|_, cx| view.read(cx).tips.as_ref().unwrap().read(cx).text())
    }

    #[gpui::test]
    fn shows_after_the_pause_not_before(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        cx.simulate_mouse_move(point(px(50.), px(15.)), None, Modifiers::none());
        cx.executor().advance_clock(DELAY - Duration::from_millis(1));
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx), None);
        cx.executor().advance_clock(Duration::from_millis(1));
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx).as_deref(), Some("Refresh the tree"));
    }

    #[gpui::test]
    fn does_not_show_when_the_pointer_leaves_first_and_hides_when_it_leaves_after(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        cx.simulate_mouse_move(point(px(50.), px(15.)), None, Modifiers::none());
        cx.simulate_mouse_move(point(px(500.), px(400.)), None, Modifiers::none());
        cx.executor().advance_clock(DELAY);
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx), None);
        cx.simulate_mouse_move(point(px(50.), px(15.)), None, Modifiers::none());
        cx.executor().advance_clock(DELAY);
        cx.run_until_parked();
        assert!(shown(&view, &mut cx).is_some());
        cx.simulate_mouse_move(point(px(500.), px(400.)), None, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx), None);
    }

    #[gpui::test]
    fn closes_on_escape_and_on_a_press_of_its_target(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        cx.simulate_mouse_move(point(px(50.), px(15.)), None, Modifiers::none());
        cx.executor().advance_clock(DELAY);
        cx.run_until_parked();
        assert!(shown(&view, &mut cx).is_some());
        cx.simulate_keystrokes("a");
        assert!(shown(&view, &mut cx).is_some(), "another key leaves it");
        cx.simulate_keystrokes("escape");
        assert_eq!(shown(&view, &mut cx), None);

        cx.simulate_mouse_move(point(px(500.), px(400.)), None, Modifiers::none());
        cx.simulate_mouse_move(point(px(50.), px(15.)), None, Modifiers::none());
        cx.executor().advance_clock(DELAY);
        cx.run_until_parked();
        assert!(shown(&view, &mut cx).is_some());
        cx.simulate_click(point(px(50.), px(15.)), Modifiers::none());
        assert_eq!(shown(&view, &mut cx), None);
        // the press keeps it closed under a pointer that has not moved
        cx.executor().advance_clock(DELAY);
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx), None);
    }

    fn rest_on(cx: &mut VisualTestContext, x: f32, y: f32) {
        cx.simulate_mouse_move(point(px(500.), px(580.)), None, Modifiers::none());
        cx.simulate_mouse_move(point(px(x), px(y)), None, Modifiers::none());
        cx.executor().advance_clock(DELAY);
        cx.run_until_parked();
    }

    /// What GPUI Kit's Root does on Tab, which the test window lacks: a key, then the next focus.
    fn tab(cx: &mut VisualTestContext) {
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| window.focus_next(cx));
        cx.run_until_parked();
    }

    #[gpui::test]
    fn shows_one_at_a_time(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        rest_on(&mut cx, 50., 15.);
        assert_eq!(shown(&view, &mut cx).as_deref(), Some("Refresh the tree"));
        rest_on(&mut cx, 50., 215.);
        assert_eq!(shown(&view, &mut cx).as_deref(), Some("Second"));
    }

    #[gpui::test]
    fn shows_at_once_on_keyboard_focus_and_once_per_focus(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        tab(&mut cx);
        assert_eq!(
            shown(&view, &mut cx).as_deref(),
            Some("Save"),
            "Tab brings the focus from the keyboard"
        );
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx), None);
        cx.simulate_keystrokes("a");
        cx.run_until_parked();
        assert_eq!(
            shown(&view, &mut cx),
            None,
            "it does not come back while the focus stays"
        );
        cx.update(|window, cx| window.blur(cx));
        cx.run_until_parked();
        tab(&mut cx);
        assert_eq!(
            shown(&view, &mut cx).as_deref(),
            Some("Save"),
            "a new focus shows it again"
        );
        cx.update(|window, cx| window.blur(cx));
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx), None, "losing the focus closes it");
    }

    #[gpui::test]
    fn does_not_show_on_a_focus_from_the_mouse(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        cx.simulate_click(point(px(50.), px(115.)), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx), None);
    }

    #[gpui::test]
    fn shows_cut_text_whole_only_when_it_is_cut(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        rest_on(&mut cx, 30., 310.);
        assert_eq!(
            shown(&view, &mut cx).as_deref(),
            Some("A very long project name that is cut")
        );
        rest_on(&mut cx, 5., 410.);
        assert_eq!(shown(&view, &mut cx), None);
    }

    /// The open tooltip's box, its 1 px border included.
    fn drawn(view: &Entity<View>, cx: &mut VisualTestContext) -> Bounds<Pixels> {
        cx.update(|_, cx| view.read(cx).tips.as_ref().unwrap().read(cx).drawn.get())
            .expect("a tooltip drawn")
            .dilate(px(1.))
    }

    fn clicks(view: &Entity<View>, cx: &mut VisualTestContext) -> u32 {
        cx.update(|_, cx| view.read(cx).clicks.get())
    }

    #[gpui::test]
    fn keeps_open_while_the_pointer_moves_from_the_target_onto_it(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        rest_on(&mut cx, 50., 15.);
        let tip = drawn(&view, &mut cx);
        assert!(tip.top() >= px(30.), "below its target: {tip:?}");
        cx.simulate_mouse_move(point(px(50.), px(29.)), None, Modifiers::none());
        cx.simulate_mouse_move(tip.center(), None, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx).as_deref(), Some("Refresh the tree"));
        cx.simulate_mouse_move(point(px(700.), px(500.)), None, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx), None, "leaving it for elsewhere closes it");
    }

    #[gpui::test]
    fn a_click_on_it_closes_it_and_reaches_nothing_under_it(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        rest_on(&mut cx, 50., 15.);
        let tip = drawn(&view, &mut cx);
        cx.simulate_click(tip.center(), Modifiers::none());
        assert_eq!(shown(&view, &mut cx), None);
        assert_eq!(clicks(&view, &mut cx), 0);
    }

    #[gpui::test]
    fn a_click_on_an_over_tooltip_within_its_target_reaches_the_target(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        rest_on(&mut cx, 330., 310.);
        assert_eq!(
            shown(&view, &mut cx).as_deref(),
            Some("A long line that is cut over its row")
        );
        let tip = drawn(&view, &mut cx);
        assert_eq!(tip.origin, point(px(300.), px(300.)), "on the target's own top-left");
        cx.simulate_click(point(px(330.), px(310.)), Modifiers::none());
        assert_eq!(shown(&view, &mut cx), None);
        assert_eq!(clicks(&view, &mut cx), 1, "the target's click");
        rest_on(&mut cx, 330., 310.);
        let tip = drawn(&view, &mut cx);
        cx.simulate_click(point(tip.right() - px(5.), tip.center().y), Modifiers::none());
        assert_eq!(clicks(&view, &mut cx), 1, "beyond the target's box it only closes");
    }

    #[gpui::test]
    fn goes_beside_its_target_on_the_side_asked_and_below_by_default(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        rest_on(&mut cx, 350., 15.);
        let left = drawn(&view, &mut cx);
        assert_eq!(left.right(), px(300. - GAP));
        assert!(left.top() < px(30.) && left.bottom() > px(0.), "on its row: {left:?}");
        rest_on(&mut cx, 550., 15.);
        let right = drawn(&view, &mut cx);
        assert_eq!(right.left(), px(600. + GAP));
        rest_on(&mut cx, 50., 15.);
        assert_eq!(drawn(&view, &mut cx).top(), px(30. + GAP));
    }

    #[gpui::test]
    fn opens_nothing_without_a_text(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        rest_on(&mut cx, 350., 115.);
        assert_eq!(shown(&view, &mut cx), None);
    }

    fn pin(view: &Entity<View>, on: bool, cx: &mut VisualTestContext) {
        view.update(cx, |v, cx| {
            v.pinned = on;
            cx.notify();
        });
        cx.run_until_parked();
    }

    /// A row the keyboard stands on (`Tip::keyboard`): its tooltip shows at once while it does, and Escape closes it
    /// for as long as it does.
    #[gpui::test]
    fn shows_at_once_while_the_keyboard_stands_on_it(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        pin(&view, true, &mut cx);
        assert_eq!(shown(&view, &mut cx).as_deref(), Some("Pinned"));
        pin(&view, false, &mut cx);
        assert_eq!(shown(&view, &mut cx), None);
        pin(&view, true, &mut cx);
        assert_eq!(shown(&view, &mut cx).as_deref(), Some("Pinned"));
        cx.simulate_keystrokes("escape");
        assert_eq!(shown(&view, &mut cx), None);
        view.update(&mut cx, |_, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(shown(&view, &mut cx), None, "not back while the keyboard stays");
    }

    /// `placeTip.test.ts`: below and centred, flipped above with no room below, kept 8 px inside the window, and a
    /// side tooltip flipped to the other side with no room on its own.
    #[gpui::test]
    fn stays_in_the_window_flipping_where_it_has_no_room(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        rest_on(&mut cx, 450., 575.);
        let up = drawn(&view, &mut cx);
        assert_eq!(up.bottom(), px(560. - GAP), "above: {up:?}");
        assert!((f32::from(up.center().x) - 450.).abs() <= 1., "centred: {up:?}");
        rest_on(&mut cx, 775., 465.);
        let clamped = drawn(&view, &mut cx);
        assert!(clamped.right() <= px(800. - EDGE), "{clamped:?}");
        rest_on(&mut cx, 25., 512.);
        let right = drawn(&view, &mut cx);
        assert_eq!(right.left(), px(40. + GAP), "flipped right: {right:?}");
        assert!(
            right.top() < px(524.) && right.bottom() > px(500.),
            "on its row: {right:?}"
        );
    }

    /// A target gone from under the tooltip (a row scrolled away, a view closed) cannot say the pointer left it: the
    /// next move of the pointer elsewhere closes the tooltip. One the keyboard opened stays.
    #[gpui::test]
    fn closes_when_the_pointer_moves_elsewhere_though_its_target_is_gone(cx: &mut TestAppContext) {
        let (view, mut cx) = open(cx);
        rest_on(&mut cx, 50., 15.);
        assert!(shown(&view, &mut cx).is_some());
        // as if the target went elsewhere: the pointer is now over nothing that tells it left
        cx.update(|window, cx| {
            let tips = view.read(cx).tips.clone().unwrap();
            tips.update(cx, |tips, _| {
                tips.open.as_mut().unwrap().target = Bounds::new(point(px(600.), px(300.)), size(px(10.), px(10.)))
            });
            window.refresh();
        });
        cx.run_until_parked();
        cx.simulate_mouse_move(point(px(52.), px(16.)), None, Modifiers::none());
        assert_eq!(shown(&view, &mut cx), None);

        pin(&view, true, &mut cx);
        cx.simulate_mouse_move(point(px(700.), px(400.)), None, Modifiers::none());
        assert_eq!(shown(&view, &mut cx).as_deref(), Some("Pinned"));
    }
}
