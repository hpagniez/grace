//! `ImportStep.tsx`: the Properties dialog's import step (spec `2026-11-04-env-var-properties.md` R5). Where the file's
//! keys go, as which kind and under which names, and what that adds, replaces and skips. It shows names and reasons,
//! never a value. The plan is `grace_core::import_plan`'s.
use crate::elements::Off as _;
use gpui_component::scroll::ScrollableElement as _;
use std::rc::Rc;

use gpui::accesskit::Role;
use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::IndexPath;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::select::{Select, SelectEvent, SelectState};
use grace_core::import_plan::{EMPTY, ImportPlan, PlanRow, import_plan};
use grace_core::model::env::{ImportEntry, MAX_ENVS, MAX_ROWS, valid_prefix};
use grace_core::properties_draft::Draft;

use super::{ALL, scope_name};
use crate::elements::{ButtonKind, ButtonSize, button};
use crate::theme::{METRICS, Theme};
use crate::ui::{FieldKind, field_message};

pub const SCOPE: &str = "Environment";
pub const KIND: &str = "Environment variables";
pub const PREFIX: &str = "Prefix";
pub const BAD_PREFIX: &str = "Letters, digits and _ only";

/// What the step asks of the dialog.
pub enum Imported {
    /// Add these secret rows of the kind to `target`'s list.
    Rows {
        target: String,
        env_var: bool,
        rows: Vec<PlanRow>,
    },
    /// Back: close the step and change nothing.
    Back,
}

pub struct ImportStep {
    file: String,
    entries: Vec<ImportEntry>,
    /// The dialog's draft as the step opened: the lists the keys can go to and the rows they would replace.
    draft: Rc<Draft>,
    target: String,
    env_var: bool,
    scope: Entity<SelectState<Vec<SharedString>>>,
    prefix: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Imported> for ImportStep {}

impl ImportStep {
    pub fn new(
        file: String,
        entries: Vec<ImportEntry>,
        draft: Rc<Draft>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let scopes = draft.scopes();
        let target = draft.scope.clone();
        let at = scopes.iter().position(|s| *s == target).unwrap_or(0);
        let labels: Vec<SharedString> = scopes.iter().map(|s| scope_name(s).to_string().into()).collect();
        let scope = cx.new(|cx| SelectState::new(labels, Some(IndexPath::new(at)), window, cx));
        let prefix = cx.new(|cx| InputState::new(window, cx));
        let subscriptions = vec![
            cx.subscribe_in(&scope, window, |this, _, e: &SelectEvent<Vec<SharedString>>, _, cx| {
                let SelectEvent::Confirm(Some(label)) = e else {
                    return;
                };
                // an environment's name has no space, so "All environments" is no environment's
                this.target = if label.as_ref() == ALL {
                    String::new()
                } else {
                    label.to_string()
                };
                cx.notify();
            }),
            cx.subscribe_in(&prefix, window, |_, _, e: &InputEvent, _, cx| {
                if let InputEvent::Change = e {
                    cx.notify();
                }
            }),
        ];
        Self {
            file,
            entries,
            draft,
            target,
            env_var: true,
            scope,
            prefix,
            _subscriptions: subscriptions,
        }
    }

    /// Puts the focus on the step's first control, as the button that opened it is gone.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        let handle: FocusHandle = gpui::Focusable::focus_handle(self.scope.read(cx), cx);
        window.focus(&handle, cx);
    }

    fn prefix_text(&self, cx: &App) -> String {
        self.prefix.read(cx).value().to_string()
    }

    fn plan(&self, cx: &App) -> ImportPlan {
        let prefix = if self.env_var {
            self.prefix_text(cx)
        } else {
            String::new()
        };
        import_plan(&self.entries, self.env_var, &prefix, &self.draft.existing(&self.target))
    }

    fn prefix_error(&self, cx: &App) -> Option<&'static str> {
        (self.env_var && !valid_prefix(&self.prefix_text(cx))).then_some(BAD_PREFIX)
    }

    fn limit(&self, plan: &ImportPlan) -> Option<String> {
        if !self.draft.can_add_list(&self.target) {
            Some(format!("At most {MAX_ENVS} environments"))
        } else if self.draft.existing(&self.target).len() + plan.adds > MAX_ROWS {
            Some(format!("At most {MAX_ROWS} properties per list"))
        } else {
            None
        }
    }
}

impl Render for ImportStep {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let plan = self.plan(cx);
        let prefix_error = self.prefix_error(cx);
        let limit = self.limit(&plan);
        let blocked = prefix_error.is_some() || limit.is_some() || plan.rows.is_empty();
        let preview = plan.rows.first().map(|r| format!("{} → {}", r.key, r.name));
        // Counted, not listed: in a file that is no .properties file each line reads as a key with no value, and that
        // key may be a token.
        let listed: Vec<_> = plan.skips.iter().filter(|s| s.reason != EMPTY).cloned().collect();
        let empty = plan.skips.len() - listed.len();
        let replaces = (!plan.replaces.is_empty())
            .then(|| format!("Replaces {}: {}", plan.replaces.len(), plan.replaces.join(", ")));
        let skips = plan.skips.len();
        let rows = plan.rows.clone();
        let env_var = self.env_var;

        let line = |id: &'static str, text: String| {
            div()
                .id(id)
                .debug_selector(move || id.into())
                .role(Role::Label)
                .aria_value(text.clone())
                .child(text)
        };

        div()
            .id("import-step")
            .debug_selector(|| "import-step".into())
            .role(Role::Group)
            .aria_label(format!("Import {}", self.file))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!("Import {}", self.file)),
            )
            .child(
                div().flex().items_center().gap_2().child(SCOPE).child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Select::new(&self.scope).accessibility_label(SCOPE)),
                ),
            )
            .child(
                crate::elements::checkbox("import-envvar")
                    .label(KIND)
                    .checked(self.env_var)
                    .text_size(px(METRICS.text_sm.size))
                    .on_click(cx.listener(|this, on: &bool, _, cx| {
                        this.env_var = *on;
                        cx.notify();
                    })),
            )
            .when(env_var, |d| {
                d.child(
                    div().flex().items_center().gap_2().child(PREFIX).child(
                        div()
                            .w(px(160.))
                            .debug_selector(|| "import-prefix".into())
                            .child(crate::elements::input(&self.prefix, METRICS.control_height).aria_label(PREFIX)),
                    ),
                )
                .when_some(prefix_error, |d, e| {
                    d.child(field_message("import-prefix-error", FieldKind::Error, e, cx))
                })
            })
            .when_some(preview.filter(|_| prefix_error.is_none()), |d, p| {
                d.child(
                    div()
                        .text_size(px(METRICS.text_xs.size))
                        .text_color(Theme::color(t.muted))
                        .child(line("import-preview", p)),
                )
            })
            .child(
                div()
                    .id("import-summary")
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(line("import-adds", format!("Adds {}", plan.adds)))
                    .when_some(replaces, |d, r| d.child(line("import-replaces", r)))
                    .when(skips > 0, |d| {
                        d.child(line("import-skips", format!("Skips {skips}:"))).child(
                            div()
                                .id("import-skip-list")
                                .role(Role::List)
                                .aria_label("Skipped keys")
                                .max_h(px(160.))
                                .overflow_y_scrollbar()
                                .text_size(px(METRICS.text_xs.size))
                                .text_color(Theme::color(t.muted))
                                .children(listed.into_iter().enumerate().map(|(i, s)| {
                                    let text = format!("{}: {}", s.key, s.reason);
                                    div()
                                        .id(SharedString::from(format!("import-skip-{i}")))
                                        .debug_selector(move || format!("import-skip-{i}"))
                                        .role(Role::ListItem)
                                        .aria_label(text.clone())
                                        .child(text)
                                }))
                                .when(empty > 0, |d| {
                                    let text = format!("{empty} with no value");
                                    d.child(
                                        div()
                                            .id("import-skip-empty")
                                            .debug_selector(|| "import-skip-empty".into())
                                            .role(Role::ListItem)
                                            .aria_label(text.clone())
                                            .child(text),
                                    )
                                }),
                        )
                    }),
            )
            .when_some(limit, |d, e| {
                d.child(field_message("import-limit", FieldKind::Error, e, cx))
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div().debug_selector(|| "import-confirm".into()).child(
                            button("import-confirm", "Import", ButtonKind::Primary, ButtonSize::Medium)
                                .off(blocked)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.emit(Imported::Rows {
                                        target: this.target.clone(),
                                        env_var,
                                        rows: rows.clone(),
                                    })
                                })),
                        ),
                    )
                    .child(
                        div().debug_selector(|| "import-back".into()).child(
                            button("import-back", "Back", ButtonKind::Plain, ButtonSize::Medium)
                                .on_click(cx.listener(|_, _, _, cx| cx.emit(Imported::Back))),
                        ),
                    ),
            )
    }
}
