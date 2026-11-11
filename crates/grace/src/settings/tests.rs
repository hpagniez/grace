//! `Settings.test.tsx`'s Appearance and page tests, headless on the fake runner, with a data folder of their own for
//! the saved theme. Its Updates tests come with phase P5; what a screen reader reads, Escape and a restart are
//! `grace-e2e/tests/appearance.rs`'s.
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{AppContext as _, Bounds, Modifiers, Pixels, TestAppContext, VisualTestContext, div};
use grace_core::model::node::Layout;
use grace_core::settings::load_appearance;

use super::*;
use crate::testing::{self, Harness};

fn data_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("grace-settings-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Page {
    h: Harness,
    cx: VisualTestContext,
    page: Entity<SettingsPage>,
    backs: Rc<Cell<u32>>,
}

/// The page on a project, with `saved` as the theme `settings.json` held at start.
fn setup(cx: &mut TestAppContext, dir: PathBuf, saved: Appearance) -> Page {
    cx.update(|cx| cx.set_global(SavedAppearance(saved)));
    let slot: Rc<RefCell<Option<Entity<SettingsPage>>>> = Rc::default();
    let backs = Rc::new(Cell::new(0));
    let (s, b) = (Rc::clone(&slot), Rc::clone(&backs));
    let state = testing::project("/p", Layout::Standalone, 3);
    let (h, cx) = testing::open_in(cx, dir, 1000., state, move |grace, _, cx| {
        let page = cx.new(|cx| SettingsPage::new(grace.clone(), move |_, _| b.set(b.get() + 1), cx));
        *s.borrow_mut() = Some(page.clone());
        Box::new(move |_, _, _| div().size_full().flex().child(page.clone()).into_any_element())
    });
    let page = slot.borrow().clone().unwrap();
    Page { h, cx, page, backs }
}

fn at(cx: &mut VisualTestContext, id: &str) -> Bounds<Pixels> {
    cx.debug_bounds(Box::leak(id.to_string().into_boxed_str()))
        .unwrap_or_else(|| panic!("{id} is drawn"))
}

fn click(cx: &mut VisualTestContext, id: &str) {
    let b = at(cx, id);
    cx.simulate_click(b.center(), Modifiers::default());
    cx.run_until_parked();
}

fn dark(cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| Theme::global(cx).dark)
}

mod the_page {
    use super::*;

    #[gpui::test]
    fn opens_on_appearance_with_its_entry_selected(cx: &mut TestAppContext) {
        let mut p = setup(cx, data_dir("opens"), Appearance::System);
        assert_eq!(Section::ALL, [Section::Appearance], "Updates joins in P5");
        assert_eq!(p.cx.update(|_, cx| p.page.read(cx).section), Section::Appearance);
        for id in [
            "settings-back",
            "settings-section-appearance",
            "settings-appearance",
            "theme-group",
        ] {
            at(&mut p.cx, id);
        }
        // the list left of the section
        assert!(at(&mut p.cx, "settings-section-appearance").right() <= at(&mut p.cx, "settings-appearance").left());
    }

    #[gpui::test]
    fn puts_the_sections_on_a_sheet_under_the_header(cx: &mut TestAppContext) {
        let mut p = setup(cx, data_dir("sheet"), Appearance::System);
        let (back, sheet) = (at(&mut p.cx, "settings-back"), at(&mut p.cx, "settings-sheet"));
        assert!(back.bottom() <= sheet.top());
        let m = METRICS;
        let page = at(&mut p.cx, "settings");
        assert_eq!(sheet.left() - page.left(), gpui::px(m.sheet_inset));
    }

    #[gpui::test]
    fn the_back_button_goes_back(cx: &mut TestAppContext) {
        let mut p = setup(cx, data_dir("back"), Appearance::System);
        click(&mut p.cx, "settings-back");
        assert_eq!(p.backs.get(), 1);
    }

    #[gpui::test]
    fn shows_the_apps_notice(cx: &mut TestAppContext) {
        let mut p = setup(cx, data_dir("notice"), Appearance::System);
        assert!(p.cx.debug_bounds("notice").is_none());
        p.h.update(&mut p.cx, |s| {
            s.reduce(&grace_core::model::events::AppEvent::notice("Invalid request"))
        });
        assert!(p.cx.debug_bounds("notice").is_some());
    }
}

mod appearance_r8 {
    use super::*;

    #[gpui::test]
    fn lists_the_three_themes_in_a_theme_group(cx: &mut TestAppContext) {
        let mut p = setup(cx, data_dir("list"), Appearance::System);
        assert_eq!(THEMES.map(|(_, l)| l), ["Follow Windows", "Light", "Dark"]);
        assert_eq!(
            THEMES.map(|(t, _)| t),
            [Appearance::System, Appearance::Light, Appearance::Dark]
        );
        let tops = ["system", "light", "dark"].map(|t| at(&mut p.cx, &format!("theme-{t}")));
        assert!(tops.windows(2).all(|w| w[0].top() < w[1].top()), "one under the other");
        assert!(tops.iter().all(|b| b.size.height == gpui::px(24.)), "24 px rows");
        let group = at(&mut p.cx, "theme-group");
        assert!(tops.iter().all(|b| group.contains(&b.center())));
    }

    /// #321: the checked radio is a ring with a dot, as a browser draws it, not GPUI Kit's filled circle with a
    /// check mark: a 13 px circle 8 px from its text, a 7 px dot in its middle, and no dot in the others.
    #[gpui::test]
    fn the_checked_radio_is_a_ring_with_a_dot_as_in_a_browser(cx: &mut TestAppContext) {
        use crate::elements::{RADIO, RADIO_DOT};
        let mut p = setup(cx, data_dir("ring-dot"), Appearance::Light);
        for t in ["system", "light", "dark"] {
            let circle = at(&mut p.cx, &format!("theme-{t}-circle"));
            assert_eq!(circle.size, gpui::size(gpui::px(RADIO), gpui::px(RADIO)), "{t}");
            let row = at(&mut p.cx, &format!("theme-{t}"));
            assert_eq!(circle.center().y, row.center().y, "{t}: on the middle of its row");
            let text = at(&mut p.cx, &format!("theme-{t}-label"));
            assert_eq!(text.left() - circle.right(), gpui::px(8.), "{t}: 8 px from its text");
        }
        assert!(p.cx.debug_bounds("theme-system-dot").is_none());
        assert!(p.cx.debug_bounds("theme-dark-dot").is_none());
        let dot = at(&mut p.cx, "theme-light-dot");
        assert_eq!(dot.size, gpui::size(gpui::px(RADIO_DOT), gpui::px(RADIO_DOT)));
        assert_eq!(dot.center(), at(&mut p.cx, "theme-light-circle").center());
        click(&mut p.cx, "theme-dark");
        assert!(p.cx.debug_bounds("theme-light-dot").is_none());
        assert!(p.cx.debug_bounds("theme-dark-dot").is_some());
    }

    #[gpui::test]
    fn choosing_a_theme_applies_it_at_once_and_saves_it(cx: &mut TestAppContext) {
        let dir = data_dir("choose");
        let mut p = setup(cx, dir.clone(), Appearance::Light);
        assert!(!dark(&mut p.cx));
        click(&mut p.cx, "theme-dark");
        assert!(dark(&mut p.cx));
        assert_eq!(p.cx.update(|_, cx| saved(cx)), Appearance::Dark);
        assert_eq!(load_appearance(&dir), Appearance::Dark);
        click(&mut p.cx, "theme-light");
        assert!(!dark(&mut p.cx));
        assert_eq!(load_appearance(&dir), Appearance::Light);
        click(&mut p.cx, "theme-system");
        assert_eq!(load_appearance(&dir), Appearance::System);
        // nothing went back on the way
        assert_eq!(p.backs.get(), 0);
    }

    #[gpui::test]
    fn choosing_the_theme_in_force_saves_nothing(cx: &mut TestAppContext) {
        let dir = data_dir("saved");
        let mut p = setup(cx, dir.clone(), Appearance::Dark);
        click(&mut p.cx, "theme-dark");
        assert!(!dir.join("settings.json").exists());
    }

    #[gpui::test]
    fn a_change_keeps_the_rest_of_the_views_state(cx: &mut TestAppContext) {
        let mut p = setup(cx, data_dir("keeps"), Appearance::Light);
        p.h.update(&mut p.cx, |s| s.selected = Some("s:a.feature#1".into()));
        click(&mut p.cx, "theme-dark");
        let selected = p.h.grace.read_with(&p.cx, |g, _| g.state.selected.clone());
        assert_eq!(selected.as_deref(), Some("s:a.feature#1"));
    }
}
