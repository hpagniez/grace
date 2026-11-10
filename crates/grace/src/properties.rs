//! `PropertiesDialog.tsx`: the Properties… button and its dialog, the run properties of every list: "All
//! environments" and each environment's own. The rules of the draft are `grace_core::properties_draft`'s; this is
//! the view on it. Save sends the draft and closes the dialog once the runner has applied it, so a refused save loses
//! nothing; Cancel, Escape and Save drop the draft with the dialog, so a typed secret does not linger.
//!
//! The import step, `ImportStep.tsx`, is [`import_step`].
pub mod import_step;

use crate::elements::Off as _;
use gpui_component::scroll::ScrollableElement as _;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription,
    Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::WindowExt as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{InputEvent, InputState};
use grace_core::model::env::{EnvView, ImportEntry, SecretState};
use grace_core::properties_draft::{Draft, Row};

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, button, icon_button};
use crate::theme::{METRICS, Theme};
use crate::tooltip::{Side, cut, tooltip};
use crate::ui::{FieldKind, field_message, field_message_marked};

use import_step::{ImportStep, Imported};

pub const LABEL: &str = "Properties…";
pub const TITLE: &str = "Run properties";
pub const ALL: &str = "All environments";
pub const ENVIRONMENTS: &str = "Environments";
pub const PROPERTIES: &str = "Properties";
pub const NEW_ENV: &str = "New environment";
pub const SECRETS_NOTE: &str = "Secrets are hidden in this app, except inside encoded text such as a Basic auth header. Karate's HTML report and event file can still contain them unless the project sets configure logging mask. Run history keeps the reports of the last 20 runs; Clear history deletes them.";
pub const UNAVAILABLE: &str = "Secret storage is not available on this system";
pub const UNREADABLE: &str = "Can't be read, enter it again";
pub const SHORT: &str = "Too short to hide in the console";
/// The dialog's width: Tailwind's `w-[48rem]`.
pub const WIDTH: f32 = 768.;

/// A scope's name in the dialog: "" is every environment's list.
pub fn scope_name(scope: &str) -> &str {
    if scope.is_empty() { ALL } else { scope }
}

/// Opens the dialog on the lists in force. Nothing opens with no project.
pub fn open(grace: &Entity<Grace>, window: &mut Window, cx: &mut App) {
    let Some(env) = grace.read(cx).state.env.clone() else {
        return;
    };
    let view = cx.new(|cx| PropertiesView::new(grace.clone(), &env, window, cx));
    // Save once applied, and Cancel, close it; Escape closes it by itself
    window
        .subscribe(&view, cx, |_, _: &Close, window, cx| window.close_dialog(cx))
        .detach();
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(div().text_size(px(METRICS.text_lg.size)).child(TITLE))
            .w(px(WIDTH))
            .child(view.clone())
    });
}

/// A row's two boxes.
struct Fields {
    name: Entity<InputState>,
    value: Entity<InputState>,
    /// Whether the value box hides its text, as the row's Secret says.
    masked: bool,
    _subscriptions: [Subscription; 2],
}

pub struct PropertiesView {
    grace: Entity<Grace>,
    pub draft: Draft,
    /// Whether the system can store secrets: without it no row can become one, and no file can be imported.
    available: bool,
    fields: HashMap<u64, Fields>,
    new_env: Entity<InputState>,
    env_error: Option<String>,
    import: Option<Entity<ImportStep>>,
    import_error: Option<String>,
    saving: bool,
    focus: FocusHandle,
    /// The Import from file… button, which the focus goes back to after the import step.
    import_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

/// The dialog's end: Save, once the runner has applied it, or Cancel.
pub struct Close;

impl EventEmitter<Close> for PropertiesView {}

impl Focusable for PropertiesView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl PropertiesView {
    pub fn new(grace: Entity<Grace>, env: &EnvView, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let new_env = cx.new(|cx| InputState::new(window, cx).placeholder("+ environment"));
        let subscriptions = vec![
            cx.subscribe_in(&new_env, window, |this, _, e: &InputEvent, window, cx| match e {
                InputEvent::Change => {
                    this.env_error = None;
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.add_env(window, cx),
                _ => {}
            }),
        ];
        let mut me = Self {
            grace,
            draft: Draft::new(env),
            available: env.secrets_available,
            fields: HashMap::new(),
            new_env,
            env_error: None,
            import: None,
            import_error: None,
            saving: false,
            focus: cx.focus_handle(),
            import_focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        };
        me.sync_fields(window, cx);
        me
    }

    /// Gives each row its boxes and drops those of rows gone.
    fn sync_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows: Vec<Row> = self.draft.rows().to_vec();
        self.fields.retain(|id, _| rows.iter().any(|r| r.id == *id));
        for r in rows {
            if self.fields.contains_key(&r.id) {
                continue;
            }
            let id = r.id;
            let name = cx.new(|cx| {
                let mut s = InputState::new(window, cx);
                s.set_value(r.name.clone(), window, cx);
                s
            });
            let value = cx.new(|cx| {
                let mut s = InputState::new(window, cx).masked(r.secret);
                s.set_value(r.value.clone(), window, cx);
                s
            });
            let subscriptions = [
                cx.subscribe_in(&name, window, move |this, input, e: &InputEvent, window, cx| {
                    if let InputEvent::Change = e {
                        let text = input.read(cx).value().to_string();
                        if this.draft.row(id).is_some_and(|r| r.name != text) {
                            this.draft.set_name(id, &text);
                            this.show_value(id, window, cx);
                            cx.notify();
                        }
                    }
                }),
                cx.subscribe_in(&value, window, move |this, input, e: &InputEvent, _, cx| {
                    if let InputEvent::Change = e {
                        let text = input.read(cx).value().to_string();
                        if this.draft.row(id).is_some_and(|r| r.value != text) {
                            this.draft.set_value(id, &text);
                            cx.notify();
                        }
                    }
                }),
            ];
            self.fields.insert(
                id,
                Fields {
                    name,
                    value,
                    masked: r.secret,
                    _subscriptions: subscriptions,
                },
            );
        }
    }

    /// Puts the draft's value of a row in its box, and hides it if the row is a secret.
    fn show_value(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(row), Some(f)) = (self.draft.row(id), self.fields.get_mut(&id)) else {
            return;
        };
        let (value, secret) = (row.value.clone(), row.secret);
        if f.value.read(cx).value() != value.as_str() {
            f.value.update(cx, |i, cx| i.set_value(value, window, cx));
        }
        if f.masked != secret {
            f.masked = secret;
            f.value.update(cx, |i, cx| i.set_masked(secret, window, cx));
        }
    }

    fn choose(&mut self, scope: String, cx: &mut Context<Self>) {
        self.draft.scope = scope;
        cx.notify();
    }

    fn add_env(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.new_env.read(cx).value().to_string();
        match self.draft.add_env(&typed) {
            Ok(()) => {
                self.env_error = None;
                self.new_env.update(cx, |i, cx| i.set_value("", window, cx));
            }
            Err(e) => self.env_error = Some(e.to_string()),
        }
        cx.notify();
    }

    fn delete_env(&mut self, cx: &mut Context<Self>) {
        self.draft.delete_env();
        cx.notify();
    }

    /// Adds a row to the list shown and puts the focus in its name.
    fn add_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.draft.add_row() {
            Ok(id) => {
                self.sync_fields(window, cx);
                if let Some(f) = self.fields.get(&id) {
                    f.name.update(cx, |i, cx| i.focus(window, cx));
                }
            }
            Err(e) => self.env_error = Some(e.to_string()),
        }
        cx.notify();
    }

    fn delete_row(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.draft.delete_row(id);
        self.sync_fields(window, cx);
        cx.notify();
    }

    /// Replace: the saved secret's row takes a new value, typed in its box.
    fn replace(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.draft.replace(id);
        self.show_value(id, window, cx);
        if let Some(f) = self.fields.get(&id) {
            f.value.update(cx, |i, cx| i.focus(window, cx));
        }
        cx.notify();
    }

    fn set_secret(&mut self, id: u64, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.draft.set_secret(id, on);
        self.show_value(id, window, cx);
        cx.notify();
    }

    fn set_env_var(&mut self, id: u64, on: bool, cx: &mut Context<Self>) {
        self.draft.set_env_var(id, on);
        cx.notify();
    }

    /// Import from file…: the file's keys open the import step; no file picked changes nothing.
    fn pick_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.import_error = None;
        let task = self.grace.update(cx, |g, cx| g.import_properties(cx));
        cx.spawn_in(window, async move |this, cx| {
            let Some(picked) = task.await else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                match picked {
                    grace_core::model::env::ImportResult::Error(e) => this.import_error = Some(e),
                    grace_core::model::env::ImportResult::File { file, entries } => {
                        this.start_import(file, entries, window, cx)
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn start_import(&mut self, file: String, entries: Vec<ImportEntry>, window: &mut Window, cx: &mut Context<Self>) {
        let draft = Rc::new(self.draft.clone());
        let step = cx.new(|cx| ImportStep::new(file, entries, draft, window, cx));
        self._subscriptions.push(
            cx.subscribe_in(&step, window, |this, _, e: &Imported, window, cx| match e {
                Imported::Rows { target, env_var, rows } => {
                    if this.draft.import(target, *env_var, rows).is_ok() {
                        this.sync_fields(window, cx);
                        let ids: Vec<u64> = this.draft.rows().iter().map(|r| r.id).collect();
                        for id in ids {
                            this.show_row(id, window, cx);
                        }
                    }
                    this.end_import(window, cx);
                }
                Imported::Back => this.end_import(window, cx),
            }),
        );
        step.update(cx, |s, cx| s.focus(window, cx));
        self.import = Some(step);
    }

    /// Puts a row's name and value back in its boxes, after an import replaced it in place.
    fn show_row(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if let (Some(row), Some(f)) = (self.draft.row(id), self.fields.get(&id)) {
            let name = row.name.clone();
            if f.name.read(cx).value() != name.as_str() {
                f.name.update(cx, |i, cx| i.set_value(name, window, cx));
            }
        }
        self.show_value(id, window, cx);
    }

    /// Closes the import step; the focus goes back to the button that opened it.
    fn end_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.import = None;
        window.focus(&self.import_focus, cx);
        cx.notify();
    }

    /// Sends the draft; the dialog closes once the runner has applied it, and stays as it is if refused.
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.saving = true;
        let edit = self.draft.edit();
        let answer = self.grace.update(cx, |g, cx| g.save_properties(edit, cx));
        cx.spawn_in(window, async move |this, cx| {
            let applied = answer.recv().await.unwrap_or(false);
            let _ = this.update_in(cx, |this, _, cx| {
                if applied {
                    cx.emit(Close);
                } else {
                    this.saving = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn render_scopes(&self, invalid: &std::collections::HashSet<String>, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let items = self.draft.scopes().into_iter().enumerate().map(|(i, scope)| {
            let selected = scope == self.draft.scope;
            let bad = invalid.contains(&scope);
            let name = scope_name(&scope).to_string();
            let spoken = if bad {
                format!("{name} has errors")
            } else {
                name.clone()
            };
            let choose = scope.clone();
            let b = Button::new(SharedString::from(format!("scope-item-{i}")))
                .ghost()
                .w_full()
                .h(px(METRICS.control_height))
                .px_2()
                .role(Role::Tab)
                .accessibility_label(spoken)
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap_1()
                        .when(selected, |d| d.font_weight(gpui::FontWeight::MEDIUM))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(cut(SharedString::from(format!("cut-scope-{i}")), name)),
                        )
                        .when(bad, |d| {
                            d.child(crate::elements::icon(Icon::CircleAlert, 14., Theme::color(t.failed)))
                        }),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.choose(choose.clone(), cx)));
            div()
                .debug_selector(move || format!("scope-item-{i}"))
                .rounded(px(METRICS.radius))
                .when(selected, |d| {
                    d.bg(Theme::color(t.selected))
                        .border_l_2()
                        .border_color(Theme::color(t.accent))
                })
                .child(crate::a11y::aria(b, |a| a.aria_selected(selected)))
        });
        div()
            .id("scopes")
            .role(Role::TabList)
            .aria_label(ENVIRONMENTS)
            .flex()
            .flex_col()
            .gap(px(2.))
            .children(items)
            .into_any_element()
    }

    fn render_row(&self, row: &Row, error: Option<&String>, cx: &mut Context<Self>) -> AnyElement {
        let id = row.id;
        let Some(f) = self.fields.get(&id) else {
            return div().into_any_element();
        };
        let label = if row.name.is_empty() {
            "property".to_string()
        } else {
            row.name.clone()
        };
        let value: AnyElement = match row.saved {
            Some(state) => div()
                .flex()
                .flex_1()
                .items_center()
                .gap_2()
                .child(match state {
                    SecretState::Set => div()
                        .id(SharedString::from(format!("prop-dots-{id}")))
                        .debug_selector(move || format!("prop-dots-{id}"))
                        .role(Role::Label)
                        .aria_value("Saved secret")
                        .child("••••••••")
                        .into_any_element(),
                    SecretState::Unreadable => field_message(
                        SharedString::from(format!("prop-unreadable-{id}")),
                        FieldKind::Warning,
                        UNREADABLE,
                        cx,
                    ),
                })
                .child(
                    div().debug_selector(move || format!("prop-replace-{id}")).child(
                        button(
                            SharedString::from(format!("prop-replace-{id}")),
                            "Replace",
                            ButtonKind::Plain,
                            ButtonSize::Medium,
                        )
                        .off(!self.available)
                        .on_click(cx.listener(move |this, _, window, cx| this.replace(id, window, cx))),
                    ),
                )
                .into_any_element(),
            None => div()
                .debug_selector(move || format!("prop-value-{id}"))
                .flex_1()
                .min_w(px(80.))
                .child(crate::elements::input(&f.value, METRICS.control_height).aria_label("Property value"))
                .into_any_element(),
        };
        let delete = format!("Delete {label}");
        let on_delete = cx.listener(move |this, _, window, cx| this.delete_row(id, window, cx));
        div()
            .id(SharedString::from(format!("property-row-{id}")))
            .debug_selector(move || format!("property-row-{id}"))
            .role(Role::ListItem)
            .aria_label(if row.name.is_empty() {
                "New property".to_string()
            } else {
                row.name.clone()
            })
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .py_1()
            .child(
                div()
                    .debug_selector(move || format!("prop-name-{id}"))
                    .w(px(160.))
                    .child(crate::elements::input(&f.name, METRICS.control_height).aria_label("Property name")),
            )
            .child(value)
            .child(
                div().debug_selector(move || format!("prop-envvar-{id}")).child(
                    Checkbox::new(SharedString::from(format!("prop-envvar-{id}")))
                        .label("Env variable")
                        .checked(row.env_var)
                        .text_size(px(METRICS.text_sm.size))
                        .on_click(cx.listener(move |this, on: &bool, _, cx| this.set_env_var(id, *on, cx))),
                ),
            )
            .child(
                div().debug_selector(move || format!("prop-secret-{id}")).child(
                    Checkbox::new(SharedString::from(format!("prop-secret-{id}")))
                        .label("Secret")
                        .checked(row.secret)
                        .off(!self.available && !row.secret)
                        .text_size(px(METRICS.text_sm.size))
                        .on_click(cx.listener(move |this, on: &bool, window, cx| this.set_secret(id, *on, window, cx))),
                ),
            )
            .child(
                div().debug_selector(move || format!("prop-delete-{id}")).child(
                    tooltip(
                        SharedString::from(format!("tip-delete-{id}")),
                        Some(delete.clone().into()),
                    )
                    .side(Side::Left)
                    .focusable(move |focus| {
                        icon_button(
                            SharedString::from(format!("prop-delete-{id}")),
                            Icon::Trash2,
                            delete,
                            true,
                        )
                        .track_focus(focus)
                        .on_click(on_delete)
                    }),
                ),
            )
            .when_some(error.cloned(), |d, e| {
                d.child(div().w_full().child(field_message(
                    SharedString::from(format!("row-error-{id}")),
                    FieldKind::Error,
                    e,
                    cx,
                )))
            })
            .when(row.short_secret(), |d| {
                d.child(div().w_full().child(field_message(
                    SharedString::from(format!("row-hint-{id}")),
                    FieldKind::Hint,
                    SHORT,
                    cx,
                )))
            })
            .into_any_element()
    }
}

impl Render for PropertiesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let errors = self.draft.errors();
        let invalid = self.draft.invalid_scopes();
        let blocked = self.saving || self.import.is_some() || !errors.is_empty();
        let rows: Vec<Row> = self.draft.visible().cloned().collect();
        let list_height = window.viewport_size().height * 0.5;

        let left = div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(176.))
            .gap_2()
            .child(self.render_scopes(&invalid, cx))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .debug_selector(|| "add-env-input".into())
                            .flex_1()
                            .min_w_0()
                            .child(crate::elements::input(&self.new_env, METRICS.control_height).aria_label(NEW_ENV)),
                    )
                    .child(
                        div().debug_selector(|| "add-env".into()).child(
                            button("add-env", "Add", ButtonKind::Plain, ButtonSize::Medium)
                                .on_click(cx.listener(|this, _, window, cx| this.add_env(window, cx))),
                        ),
                    ),
            )
            .when_some(self.env_error.clone(), |d, e| {
                d.child(field_message("add-env-error", FieldKind::Error, e, cx))
            })
            .child(
                div().debug_selector(|| "delete-env".into()).child(
                    button(
                        "delete-env",
                        "Delete environment",
                        ButtonKind::Plain,
                        ButtonSize::Medium,
                    )
                    .off(!self.draft.can_delete_env())
                    .on_click(cx.listener(|this, _, _, cx| this.delete_env(cx))),
                ),
            );

        let main: AnyElement = match &self.import {
            Some(step) => step.clone().into_any_element(),
            None => {
                let items: Vec<AnyElement> = rows.iter().map(|r| self.render_row(r, errors.get(&r.id), cx)).collect();
                let import_focus = self.import_focus.clone();
                let available = self.available;
                let on_import = cx.listener(|this, _, window, cx| this.pick_file(window, cx));
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .id("property-rows")
                            .role(Role::List)
                            .aria_label(PROPERTIES)
                            .max_h(list_height)
                            .overflow_y_scrollbar()
                            .px_1()
                            .children(items),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div().debug_selector(|| "add-property".into()).child(
                                    button("add-property", "Add property", ButtonKind::Plain, ButtonSize::Medium)
                                        .icon(Icon::Plus)
                                        .on_click(cx.listener(|this, _, window, cx| this.add_row(window, cx))),
                                ),
                            )
                            .child(
                                div().debug_selector(|| "import-file".into()).child(
                                    button(
                                        "import-file",
                                        "Import from file…",
                                        ButtonKind::Plain,
                                        ButtonSize::Medium,
                                    )
                                    .icon(Icon::FileUp)
                                    .track_focus(&import_focus)
                                    .off(!available)
                                    .on_click(on_import),
                                ),
                            ),
                    )
                    .when_some(self.import_error.clone(), |d, e| {
                        d.child(field_message("import-error", FieldKind::Error, e, cx))
                    })
                    .into_any_element()
            }
        };

        let section = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap_2()
            .child(main)
            .when(!self.available, |d| {
                d.child(field_message(
                    "secrets-unavailable",
                    FieldKind::Warning,
                    UNAVAILABLE,
                    cx,
                ))
            })
            .when(self.draft.has_secret(), |d| {
                d.child(field_message_marked(
                    "secrets-note",
                    FieldKind::Hint,
                    Some(Icon::Lock),
                    SECRETS_NOTE,
                    cx,
                ))
            });

        let saving = self.saving;
        div()
            .id("properties-dialog")
            .debug_selector(|| "properties-dialog".into())
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .gap_5()
            .text_size(px(METRICS.text_sm.size))
            .child(div().flex().gap_4().child(left).child(section))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        div().debug_selector(|| "save-properties".into()).child(
                            button(
                                "save-properties",
                                if saving { "Saving…" } else { "Save" },
                                ButtonKind::Primary,
                                ButtonSize::Dialog,
                            )
                            .off(blocked)
                            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                        ),
                    )
                    .child(
                        div().debug_selector(|| "cancel-properties".into()).child(
                            button("cancel-properties", "Cancel", ButtonKind::Plain, ButtonSize::Dialog)
                                .off(saving)
                                .on_click(cx.listener(|_, _, _, cx| cx.emit(Close))),
                        ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests;
