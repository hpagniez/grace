//! `Settings.tsx`, its P2 part: the page that covers the view until Back or Escape, a header on the ground and a sheet
//! with the list of sections and the open one (spec R21), and Appearance with the theme (premium UI spec R8). Updates
//! joins with the updater in phase P5, and the page then opens on it (P2 plan, Redesigns).
use gpui::accesskit::Role;
use gpui::{
    App, Context, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, actions, div, prelude::FluentBuilder as _, px,
};
use gpui_component::radio::{Radio, RadioGroup};

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, button, icon_text_button};
use crate::theme::{Appearance, METRICS, SavedAppearance, Theme};

pub const TITLE: &str = "Settings";
pub const BACK: &str = "Back";
pub const SECTIONS_NAME: &str = "Settings sections";
pub const THEME: &str = "Theme";
/// The key context the window's root takes while the page shows: Escape goes back in it.
pub const CONTEXT: &str = "Settings";

actions!(settings, [GoBack]);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([gpui::KeyBinding::new("escape", GoBack, Some(CONTEXT))]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Appearance,
}

impl Section {
    pub const ALL: [Section; 1] = [Section::Appearance];

    pub fn label(self) -> &'static str {
        match self {
            Section::Appearance => "Appearance",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Section::Appearance => "appearance",
        }
    }
}

/// The three themes, in the order the page lists them.
pub const THEMES: [(Appearance, &str); 3] = [
    (Appearance::System, "Follow Windows"),
    (Appearance::Light, "Light"),
    (Appearance::Dark, "Dark"),
];

/// The theme in force: what `settings.json` held at start or Settings chose since.
pub fn saved(cx: &App) -> Appearance {
    cx.try_global::<SavedAppearance>().copied().unwrap_or_default().0
}

/// Applies `theme` to the window at once and saves it; a save that fails is logged, and the theme stays in force
/// until Grace closes, as Electron Grace's main did.
pub fn choose(grace: &Entity<Grace>, theme: Appearance, window: &mut Window, cx: &mut App) {
    if saved(cx) == theme {
        return;
    }
    crate::theme::set_appearance(theme, window, cx);
    let dir = grace.read(cx).data_dir().to_path_buf();
    if let Err(e) = grace_core::settings::save_appearance(&dir, theme) {
        eprintln!("the theme could not be saved: {e}");
    }
    window.refresh();
}

/// What Back and Escape run: the window shows its view again.
pub type Back = std::rc::Rc<dyn Fn(&mut Window, &mut App)>;

pub struct SettingsPage {
    grace: Entity<Grace>,
    pub section: Section,
    /// What Back and Escape do: the window shows its view again.
    back: Back,
    _grace: gpui::Subscription,
}

impl SettingsPage {
    pub fn new(grace: Entity<Grace>, back: impl Fn(&mut Window, &mut App) + 'static, cx: &mut Context<Self>) -> Self {
        // the app's notice shows on the page
        let observed = cx.observe(&grace, |_, _, cx| cx.notify());
        Self {
            grace,
            _grace: observed,
            section: Section::Appearance,
            back: std::rc::Rc::new(back),
        }
    }

    pub fn open(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        cx.notify();
    }

    /// What Back and Escape run.
    pub fn back(&self) -> Back {
        self.back.clone()
    }

    fn header(&self) -> impl IntoElement {
        let back = self.back.clone();
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .min_h(px(44.))
            .px_3()
            .child(
                div().debug_selector(|| "settings-back".into()).child(
                    icon_text_button(
                        "settings-back",
                        Icon::ArrowLeft,
                        BACK,
                        ButtonKind::Ghost,
                        ButtonSize::Medium,
                    )
                    .on_click(move |_, window, cx| back(window, cx)),
                ),
            )
            .child(
                div()
                    .id("settings-title")
                    .role(Role::Heading)
                    .aria_level(1)
                    .aria_label(TITLE)
                    .text_size(px(METRICS.text_base.size))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(TITLE),
            )
    }

    fn nav(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        div()
            .id("settings-sections")
            .role(Role::Navigation)
            .aria_label(SECTIONS_NAME)
            .flex()
            .flex_col()
            .flex_none()
            .w(px(192.))
            .p_2()
            .border_r_1()
            .border_color(Theme::color(t.line))
            .children(Section::ALL.map(|s| {
                let on = self.section == s;
                let b = button(
                    SharedString::from(format!("settings-section-{}", s.id())),
                    // the label as a child that fills the row: GPUI Kit centres a button's own label
                    "",
                    ButtonKind::Ghost,
                    ButtonSize::Medium,
                )
                .accessibility_label(s.label())
                .child(
                    div()
                        .flex_1()
                        .text_size(px(METRICS.text_sm.size))
                        .when(on, |d| d.font_weight(FontWeight::MEDIUM))
                        .child(s.label()),
                )
                .w_full()
                .h(px(28.))
                .px_2()
                .on_click(cx.listener(move |this, _, _, cx| this.open(s, cx)));
                div()
                    .debug_selector(move || format!("settings-section-{}", s.id()))
                    .rounded(px(METRICS.radius))
                    .when(on, |d| {
                        d.bg(Theme::color(t.selected))
                            .border_l_2()
                            .border_color(Theme::color(t.accent))
                    })
                    .child(crate::a11y::aria(b, |a| a.aria_selected(on)))
            }))
    }

    fn appearance(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let grace = self.grace.clone();
        let current = saved(cx);
        let index = THEMES.iter().position(|(theme, _)| *theme == current);
        div()
            .id("settings-appearance")
            .debug_selector(|| "settings-appearance".into())
            .role(Role::Region)
            .aria_label(Section::Appearance.label())
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .px_6()
            .py_5()
            .child(
                div()
                    .id("appearance-heading")
                    .role(Role::Heading)
                    .aria_level(2)
                    .aria_label(Section::Appearance.label())
                    .text_size(px(METRICS.text_lg.size))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(Section::Appearance.label()),
            )
            .child(
                div()
                    .id("theme-group")
                    .debug_selector(|| "theme-group".into())
                    .role(Role::Group)
                    .aria_label(THEME)
                    .mt_4()
                    .child(
                        div()
                            .text_size(px(METRICS.text_sm.size))
                            .font_weight(FontWeight::MEDIUM)
                            .child(THEME),
                    )
                    .child(
                        RadioGroup::vertical("theme")
                            .mt_1()
                            .gap_0()
                            .selected_index(index)
                            .children(THEMES.map(|(theme, label)| {
                                Radio::new(label)
                                    .label(label)
                                    .debug_selector(move || format!("theme-{}", theme.as_str()))
                                    .h(px(24.))
                                    .items_center()
                                    .text_size(px(METRICS.text_sm.size))
                            }))
                            .on_click(move |i, window, cx| {
                                if let Some((theme, _)) = THEMES.get(*i) {
                                    choose(&grace, *theme, window, cx);
                                }
                            }),
                    ),
            )
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let m = METRICS;
        let section = match self.section {
            Section::Appearance => self.appearance(cx).into_any_element(),
        };
        let shown = self
            .grace
            .read(cx)
            .state
            .shown_notice()
            .map(|n| (n.success, n.text.clone()));
        let notice = shown.map(|(success, text)| {
            crate::ui::notice(
                "settings-notice",
                if success {
                    crate::ui::NoticeKind::Success
                } else {
                    crate::ui::NoticeKind::Warning
                },
                text,
                vec![],
                cx,
            )
        });
        div()
            .id("settings")
            .debug_selector(|| "settings".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(self.header())
            .child(
                div()
                    .id("settings-sheet")
                    .debug_selector(|| "settings-sheet".into())
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .mx(px(m.sheet_inset))
                    .mb(px(m.sheet_inset))
                    .overflow_hidden()
                    .rounded(px(m.sheet_radius))
                    .border_t_1()
                    .border_color(Theme::color(t.line))
                    .bg(Theme::color(t.surface))
                    .children(notice.map(|n| div().px_3().py_2().child(n)))
                    .child(div().flex().flex_1().min_h_0().child(self.nav(cx)).child(section)),
            )
    }
}

#[cfg(test)]
mod tests;
