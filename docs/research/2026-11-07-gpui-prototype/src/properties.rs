//! The Properties dialog (PropertiesDialog.tsx, reduced): the project's environments on the left, the chosen one's
//! properties as name and value fields on the right, rows added and removed. Nothing is written to disk.
use gpui::{Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, prelude::*, px};
use gpui_component::{
    Icon, Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};

use crate::look::palette;
use gpui_kit_assets::IconName;

pub struct Row {
    pub name: Entity<InputState>,
    pub value: Entity<InputState>,
}

pub struct PropertiesForm {
    pub envs: Vec<SharedString>,
    pub chosen: usize,
    pub rows: Vec<Row>,
    next: usize,
}

impl PropertiesForm {
    pub fn new(envs: Vec<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut envs: Vec<SharedString> = envs.into_iter().map(Into::into).collect();
        envs.insert(0, "(default)".into());
        let mut f = PropertiesForm { envs, chosen: 0, rows: vec![], next: 0 };
        f.add_row(window, cx);
        f
    }

    fn add_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("name"));
        let value = cx.new(|cx| InputState::new(window, cx).placeholder("value"));
        self.rows.push(Row { name, value });
        self.next += 1;
        cx.notify();
    }
}

impl Render for PropertiesForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let envs = div()
            .w(px(160.))
            .flex()
            .flex_col()
            .gap_0p5()
            .pr_3()
            .border_r_1()
            .border_color(p.line)
            .children(self.envs.iter().enumerate().map(|(i, e)| {
                div()
                    .id(("env", i))
                    .px_2()
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .rounded_md()
                    .cursor_pointer()
                    .when(i == self.chosen, |d| d.bg(p.selected))
                    .hover(|s| s.bg(p.hover))
                    .child(Icon::new(IconName::Globe).small().text_color(p.muted))
                    .child(e.clone())
                    .on_click(cx.listener(move |f, _, _, cx| {
                        f.chosen = i;
                        cx.notify();
                    }))
            }));
        let rows = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_2()
            .pl_3()
            .children(self.rows.iter().enumerate().map(|(i, r)| {
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(150.)).child(Input::new(&r.name).small().aria_label("Property name")))
                    .child(div().flex_1().child(Input::new(&r.value).small().aria_label("Property value")))
                    .child(
                        Button::new(("delete-row", i))
                            .ghost()
                            .small()
                            .icon(Icon::new(IconName::Close))
                            .tooltip("Delete property")
                            .on_click(cx.listener(move |f, _, _, cx| {
                                f.rows.remove(i);
                                cx.notify();
                            })),
                    )
            }))
            .child(
                div().child(
                    Button::new("add-row")
                        .ghost()
                        .small()
                        .icon(Icon::new(IconName::Plus))
                        .label("Add property")
                        .on_click(cx.listener(|f, _, window, cx| f.add_row(window, cx))),
                ),
            );
        div().flex().min_h(px(220.)).child(envs).child(rows)
    }
}
