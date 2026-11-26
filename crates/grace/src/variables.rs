//! `Variables.tsx`: the Variables tab (step debugger R7.4, R7.5): the paused scenario's variables by name, each with its
//! kind or Java type and one cut line, a `json` or `xml` value opening into a box of selectable text with the whole of
//! it. Between pauses the last pause's variables stay, dimmed, under "Running…".
//!
//! What a row says is `grace_core::variables`'; the rows are worked out once a pause, never as the tab is drawn. The
//! values come masked from the runner.
use std::collections::HashMap;
use std::rc::Rc;

use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, prelude::FluentBuilder as _,
    px,
};
use gpui_base::SelectableText;
use gpui_component::scroll::ScrollableElement as _;
use grace_core::variables::{full_value, more_characters, opens, sorted, type_label, value_line};

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, icon_text_button};
use crate::theme::{METRICS, MONO, Theme};
use crate::tooltip::{Side, cut};

pub const WAITING: &str = "Variables show when the run pauses";
pub const RUNNING: &str = "Running…";
pub const LABEL: &str = "Variables";

/// The button's name: what a press does, and to which variable.
pub fn toggle_name(name: &str, open: bool) -> String {
    format!("{} {name}", if open { "Hide" } else { "Show" })
}

/// One variable as its row draws it.
pub struct Row {
    pub name: SharedString,
    /// The Java type's name, else the kind.
    pub kind: SharedString,
    pub line: SharedString,
    /// How much Java left out of a value over 64 KB.
    pub cut: Option<SharedString>,
    /// Whether the value opens into the whole of it.
    opens: bool,
}

pub struct Variables {
    grace: Entity<Grace>,
    /// The pause the rows are of: its run and its number.
    pause: (Option<String>, u32),
    rows: Rc<Vec<Row>>,
    /// Whether it was dimmed when it was last told to draw.
    dimmed: bool,
    /// The values open now, by name, each as its box shows it.
    open: HashMap<SharedString, SharedString>,
    _observe: Subscription,
}

fn pause_of(grace: &Entity<Grace>, cx: &App) -> (Option<String>, u32) {
    let run = &grace.read(cx).state.run;
    (run.run_id.clone(), run.pauses)
}

impl Variables {
    pub fn new(grace: Entity<Grace>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&grace, |this: &mut Self, _, cx| {
            // a run's events change Grace many times a second, most of them nothing this tab shows
            let dimmed = this.dimmed(cx);
            if this.follow(cx) || dimmed != this.dimmed {
                this.dimmed = dimmed;
                cx.notify();
            }
        });
        let mut me = Self {
            grace,
            pause: (None, 0),
            rows: Rc::default(),
            dimmed: false,
            open: HashMap::new(),
            _observe: observe,
        };
        me.follow(cx);
        me
    }

    /// The rows again on a new pause, every value closed, as Electron Grace made its rows anew; true if it is one.
    fn follow(&mut self, cx: &App) -> bool {
        let pause = pause_of(&self.grace, cx);
        if pause == self.pause {
            return false;
        }
        self.pause = pause;
        self.open.clear();
        let vars = self.grace.read(cx).state.vars.as_deref().unwrap_or_default();
        self.rows = Rc::new(
            sorted(vars)
                .into_iter()
                .map(|v| Row {
                    name: v.name.clone().into(),
                    kind: type_label(v).to_owned().into(),
                    line: value_line(&v.value).into(),
                    cut: v.cut.filter(|c| *c > 0).map(|c| more_characters(c).into()),
                    opens: opens(v.kind),
                })
                .collect(),
        );
        true
    }

    /// The rows, shared: the same `Rc` until the next pause.
    pub fn rows(&self) -> Rc<Vec<Row>> {
        Rc::clone(&self.rows)
    }

    /// The text of `name`'s box, while it is open.
    pub fn opened(&self, name: &str) -> Option<&SharedString> {
        self.open.get(name)
    }

    /// The last pause's variables show while the run goes on.
    pub fn dimmed(&self, cx: &App) -> bool {
        let state = &self.grace.read(cx).state;
        state.vars.is_some() && state.paused.is_none()
    }

    /// The tab is closed: every value closes.
    pub fn close(&mut self) {
        self.open.clear();
    }

    fn toggle(&mut self, name: &str, cx: &mut Context<Self>) {
        if self.open.remove(name).is_none() {
            let vars = self.grace.read(cx).state.vars.as_deref().unwrap_or_default();
            if let Some(v) = vars.iter().find(|v| v.name == name) {
                let full = full_value(v).into();
                self.open.insert(name.to_owned().into(), full);
            }
        }
        cx.notify();
    }

    fn row(&self, i: usize, row: &Row, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let name = row.name.clone();
        let part = |what: &str| SharedString::from(format!("{what}:{name}"));
        let full = self.open.get(&name).cloned();
        let open = full.is_some();
        let toggle = row.opens.then(|| {
            let (me, pressed) = (cx.entity(), name.clone());
            let selector = part("variable-toggle");
            crate::a11y::aria(
                icon_text_button(
                    selector.clone(),
                    if open { Icon::ChevronDown } else { Icon::ChevronRight },
                    "Value",
                    ButtonKind::Ghost,
                    ButtonSize::Small,
                )
                .accessibility_label(SharedString::from(toggle_name(&name, open)))
                .debug_selector(move || selector.to_string()),
                |a| a.aria_expanded(open),
            )
            .flex_none()
            .on_click(move |_, _, cx| me.update(cx, |v, cx| v.toggle(&pressed, cx)))
        });
        // a part a screen reader reads by itself, found by the tests by its selector
        let label = |what: &str, text: SharedString| {
            let selector = part(what);
            div()
                .id(selector.clone())
                .debug_selector(move || selector.to_string())
                .role(Role::Label)
                .aria_value(text)
        };
        let head = div()
            .flex()
            .items_center()
            .gap_2()
            .min_h(px(METRICS.control_height_small))
            .child(
                div()
                    .flex_none()
                    .font_family(MONO)
                    .text_size(px(METRICS.text_xs.size))
                    .font_weight(FontWeight::MEDIUM)
                    .child(name.clone()),
            )
            .child(
                label("variable-kind", row.kind.clone())
                    .flex_none()
                    .child(crate::ui::tag_chip(row.kind.clone(), cx)),
            )
            .child(
                label("variable-line", row.line.clone())
                    .flex_1()
                    .min_w_0()
                    .font_family(MONO)
                    .text_size(px(METRICS.text_xs.size))
                    .child(cut(part("variable-cut-line"), row.line.clone()).side(Side::Over)),
            )
            .children(row.cut.clone().map(|more| {
                label("variable-cut", more.clone())
                    .flex_none()
                    .text_size(px(METRICS.text_xs.size))
                    .text_color(Theme::color(t.muted))
                    .child(more)
            }))
            .children(toggle);
        let value = full.map(|full| {
            // a selection belongs to the pause it was made in
            let id = SharedString::from(format!("variable-text:{}:{name}", self.pause.1));
            label("variable-value", full.clone())
                .mt_1()
                .max_h(px(240.))
                .overflow_scroll()
                .rounded(px(METRICS.radius_lg))
                .bg(Theme::color(t.sunken))
                .p_2()
                .font_family(MONO)
                .text_size(px(METRICS.text_xs.size))
                .whitespace_nowrap()
                .child(SelectableText::new(id, full))
        });
        let selector = part("variable");
        div()
            .id(selector.clone())
            .debug_selector(move || selector.to_string())
            .role(Role::ListItem)
            .aria_label(name.clone())
            .flex()
            .flex_col()
            .flex_none()
            .py_1()
            .when(i > 0, |d| d.border_t_1().border_color(Theme::color(t.line)))
            .child(head)
            .children(value)
            .into_any_element()
    }
}

impl Render for Variables {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        if self.grace.read(cx).state.vars.is_none() {
            return crate::ui::empty_state("variables-empty", Icon::Braces, WAITING, None, cx).into_any_element();
        }
        let dimmed = self.dimmed(cx);
        let rows = self.rows();
        let rows: Vec<AnyElement> = rows.iter().enumerate().map(|(i, row)| self.row(i, row, cx)).collect();
        div()
            .id("variables")
            .debug_selector(|| "variables".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .px_3()
            .py_1()
            .overflow_y_scrollbar()
            .when(dimmed, |d| {
                d.opacity(0.6).child(
                    div()
                        .id("variables-running")
                        .debug_selector(|| "variables-running".into())
                        .role(Role::Label)
                        .aria_value(RUNNING)
                        .flex_none()
                        .py_1()
                        .text_size(px(METRICS.text_xs.size))
                        .text_color(Theme::color(t.muted))
                        .child(RUNNING),
                )
            })
            .child(
                div()
                    .id("variables-list")
                    .role(Role::List)
                    .aria_label(LABEL)
                    .flex()
                    .flex_col()
                    .flex_none()
                    .children(rows),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
