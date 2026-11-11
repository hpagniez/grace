//! `FilterBar.test.tsx`, headless: the chips and what a click asks of the runner, Clear, the counter, Hide
//! non-matching, the locks of a run, and the fold of the chips past the line with its panel. The tree's dimming and
//! hiding are `grace::tree`'s and `grace_core::rows`' tests; what assistive technology reads is
//! `grace-e2e/tests/tags.rs`'.
use std::cell::RefCell;
use std::path::Path;
use std::sync::LazyLock;

use gpui::{AppContext as _, Bounds, Modifiers, Pixels, TestAppContext, VisualTestContext, point, size};
use grace_core::karate::events::read_all_events;
use grace_core::karate::tree::build_tree;
use grace_core::model::node::{Layout, NodeKind};
use grace_core::model::tags::no_filter;

use super::*;
use crate::testing::{self, Harness};

/// The clean sample: 9 tests, tags @slow (2), @smoke (4), @wip (2); outline.feature has none.
static CLEAN: LazyLock<TestNode> = LazyLock::new(|| {
    let events = read_all_events(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/fixtures/events")
            .join("dryrun-clean.jsonl"),
    )
    .unwrap();
    build_tree(&events, "clean").unwrap()
});

fn filter(include: &[&str], exclude: &[&str], hide: bool) -> FilterState {
    FilterState {
        include: include.iter().map(|t| t.to_string()).collect(),
        exclude: exclude.iter().map(|t| t.to_string()).collect(),
        hide_non_matching: hide,
    }
}

fn smoke_not_wip(hide: bool) -> FilterState {
    filter(&["smoke"], &["wip"], hide)
}

fn clean(f: FilterState) -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 0);
    s.tree = Some(CLEAN.clone());
    s.filter = f;
    s
}

/// One feature whose scenarios carry `tags` tags, `tag-00` to `tag-NN`: too many for one line.
fn many(tags: usize) -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 0);
    let mut tree = testing::tree(tags);
    for (i, sc) in tree.children[0].children.iter_mut().enumerate() {
        sc.tags = vec![format!("tag-{i:02}")];
        sc.kind = NodeKind::Scenario;
    }
    s.tree = Some(tree);
    s
}

fn setup(cx: &mut TestAppContext, width: f32, state: AppState) -> (Harness, VisualTestContext, Entity<FilterBar>) {
    let slot: Rc<RefCell<Option<Entity<FilterBar>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let (h, vcx) = testing::open(cx, width, state, move |grace, _, cx| {
        s.borrow_mut()
            .get_or_insert_with(|| cx.new(|cx| FilterBar::new(grace.clone(), cx)))
            .clone()
            .into_any_element()
    });
    let bar = slot.borrow().clone().unwrap();
    (h, vcx, bar)
}

fn at(cx: &mut VisualTestContext, id: &str) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(Box::leak(id.to_owned().into_boxed_str()))
}

fn click(cx: &mut VisualTestContext, id: &str) {
    let b = at(cx, id).unwrap_or_else(|| panic!("{id} is drawn"));
    cx.simulate_click(b.center(), Modifiers::default());
    cx.run_until_parked();
}

fn calls(h: &Harness) -> Vec<String> {
    h.backend.calls()
}

fn sent(f: &FilterState) -> String {
    format!("set_filter {}", serde_json::to_string(f).unwrap())
}

fn folded(bar: &Entity<FilterBar>, cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, cx| bar.read(cx).folded().to_vec())
}

fn is_open(bar: &Entity<FilterBar>, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| bar.read(cx).open)
}

/// The tooltip after the pointer rests on `id`.
fn hover(h: &Harness, cx: &mut VisualTestContext, id: &str) -> Option<SharedString> {
    let b = at(cx, id).unwrap();
    cx.simulate_mouse_move(b.center(), None, Modifiers::none());
    cx.executor()
        .advance_clock(crate::tooltip::DELAY + std::time::Duration::from_millis(1));
    cx.run_until_parked();
    let tip = h.tip(cx);
    cx.simulate_mouse_move(point(px(1.), px(600.)), None, Modifiers::none());
    cx.run_until_parked();
    tip
}

mod words {
    use super::*;

    #[test]
    fn a_chip_says_its_state_in_its_name_and_its_next_click_in_its_hint() {
        let f = smoke_not_wip(false);
        assert_eq!(chip_name("smoke", ChipState::of(&f, "smoke")), "@smoke, included");
        assert_eq!(chip_name("wip", ChipState::of(&f, "wip")), "@wip, excluded");
        assert_eq!(chip_name("slow", ChipState::of(&f, "slow")), "@slow, not filtered");
        assert_eq!(ChipState::Off.hint(), "Not filtered. Click to include.");
        assert_eq!(ChipState::Include.hint(), "Included. Click to exclude.");
        assert_eq!(ChipState::Exclude.hint(), "Excluded. Click to clear.");
    }

    /// PU-10: the states differ by shape, not only by colour: "@smoke, included" has the check, "@wip, excluded"
    /// the minus and a line through it, and a tag that is off neither.
    #[test]
    fn a_chips_state_shows_in_its_shape_and_its_name_not_only_its_colour() {
        let f = smoke_not_wip(false);
        let smoke = ChipState::of(&f, "smoke");
        let wip = ChipState::of(&f, "wip");
        let slow = ChipState::of(&f, "slow");
        assert_eq!((smoke.mark(), smoke.struck()), (Some(Icon::Check), false));
        assert_eq!((wip.mark(), wip.struck()), (Some(Icon::Minus), true));
        assert_eq!((slow.mark(), slow.struck()), (None, false));
        assert_eq!(chip_name("smoke", smoke), "@smoke, included");
        assert_eq!(chip_name("wip", wip), "@wip, excluded");
    }

    #[test]
    fn the_fold_chip_counts_the_folded_tags_and_those_that_are_on() {
        assert_eq!((fold_text(2, 0), fold_name(2, 0)), ("+2".into(), "2 more tags".into()));
        assert_eq!(
            (fold_text(2, 1), fold_name(2, 1)),
            ("+2 · 1 on".into(), "2 more tags, 1 on".into())
        );
        assert_eq!(fold_name(1, 1), "1 more tag, 1 on");
    }

    #[test]
    fn the_counter_counts_the_matching_tests_of_all() {
        assert_eq!(counter(&CLEAN, &smoke_not_wip(false)), "2 of 9 tests match");
        assert_eq!(counter(&CLEAN, &filter(&["gone"], &[], false)), "0 of 9 tests match");
    }

    #[test]
    fn a_chip_folds_once_it_is_below_the_first_line() {
        let tags: Vec<String> = ["a", "b", "c", "d"].iter().map(|t| t.to_string()).collect();
        let b = |x: f32, y: f32| Bounds::new(point(px(x), px(y)), size(px(40.), px(24.)));
        assert_eq!(
            folded_tags(&tags, &[b(4., 4.), b(50., 4.), b(4., 34.), b(50., 34.)]),
            ["c", "d"]
        );
        assert!(folded_tags(&tags, &[b(4., 4.), b(50., 4.), b(96., 4.), b(142., 4.)]).is_empty());
        assert!(folded_tags(&tags, &[]).is_empty());
    }
}

mod filter_bar {
    use super::*;

    #[gpui::test]
    fn shows_a_chip_per_tag_from_a_to_z_and_no_clear_or_counter_while_off(cx: &mut TestAppContext) {
        let (_h, mut cx, bar) = setup(cx, 1200., clean(no_filter()));
        let xs: Vec<Pixels> = ["slow", "smoke", "wip"]
            .iter()
            .map(|t| at(&mut cx, &format!("tag-chip-{t}")).unwrap().origin.x)
            .collect();
        assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}");
        assert!(at(&mut cx, "tag-clear").is_none());
        assert!(at(&mut cx, "tag-more").is_none());
        assert!(folded(&bar, &mut cx).is_empty());
        // the counter's live region is there before its text
        assert!(at(&mut cx, "tag-counter").is_some());
        let hide = at(&mut cx, "hide-non-matching").unwrap();
        assert!(hide.left() > at(&mut cx, "tag-chip-wip").unwrap().right());
        assert_eq!(at(&mut cx, "filter-bar").unwrap().size.height, px(BAR_HEIGHT));
    }

    /// `chipBase` and the browser's checkbox: a chip is its 1 px border, `px-1.5`, its mark and `gap-1`, and its text
    /// in `text-2xs` medium; Hide non-matching is the browser's 13 px box, 6 px, and its text in `text-xs`. GPUI
    /// Kit's button and checkbox set them in their own 14 px text, with a 16 px box 8 px from it.
    #[gpui::test]
    fn the_chips_and_the_checkbox_are_as_wide_as_their_text_at_electron_graces_sizes(cx: &mut TestAppContext) {
        use gpui::FontWeight;
        let (_h, mut cx, _bar) = setup(cx, 1200., clean(smoke_not_wip(false)));
        let m = crate::theme::METRICS;
        let width = |cx: &mut VisualTestContext, text: &str, size: f32, weight: FontWeight| {
            cx.update(|window, _| testing::text_width(window, text, size, weight))
        };
        let near = |what: &str, got: Pixels, want: f32| {
            assert!(
                (f32::from(got) - want).abs() < 1.,
                "{what}: {got:?} wide, {want} wanted"
            );
        };
        let slow = width(&mut cx, "@slow", m.text_2xs.size, FontWeight::MEDIUM);
        near(
            "@slow",
            at(&mut cx, "tag-chip-slow").unwrap().size.width,
            2. * 7. + slow,
        );
        // included: the check mark, 14 px, then 4 px
        let smoke = width(&mut cx, "@smoke", m.text_2xs.size, FontWeight::MEDIUM);
        near(
            "@smoke",
            at(&mut cx, "tag-chip-smoke").unwrap().size.width,
            2. * 7. + 14. + 4. + smoke,
        );
        let hide = width(&mut cx, HIDE_NON_MATCHING, m.text_xs.size, FontWeight::NORMAL);
        near(
            "hide",
            at(&mut cx, "hide-non-matching").unwrap().size.width,
            crate::elements::CHECKBOX + 6. + hide,
        );
    }

    #[gpui::test]
    fn a_chip_click_asks_the_runner_for_the_next_state_and_waits_for_its_answer(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 1200., clean(no_filter()));
        click(&mut cx, "tag-chip-smoke");
        assert_eq!(calls(&h), [sent(&filter(&["smoke"], &[], false))]);
        // the runner sends it back
        h.update(&mut cx, |s| s.filter = filter(&["smoke"], &[], false));
        click(&mut cx, "tag-chip-smoke");
        assert_eq!(calls(&h)[1], sent(&filter(&[], &["smoke"], false)));
    }

    #[gpui::test]
    fn clear_keeps_hide_non_matching_and_the_counter_shows_while_on(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 1200., clean(smoke_not_wip(true)));
        assert!(at(&mut cx, "tag-counter").unwrap().size.width > px(0.));
        click(&mut cx, "tag-clear");
        assert_eq!(calls(&h), [sent(&filter(&[], &[], true))]);
    }

    #[gpui::test]
    fn toggles_hide_non_matching_through_the_runner(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 1200., clean(smoke_not_wip(false)));
        click(&mut cx, "hide-non-matching");
        assert_eq!(calls(&h), [sent(&smoke_not_wip(true))]);
    }

    #[gpui::test]
    fn shows_the_next_click_in_a_tooltip(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 1200., clean(smoke_not_wip(false)));
        assert_eq!(
            hover(&h, &mut cx, "tag-chip-smoke").as_deref(),
            Some("Included. Click to exclude.")
        );
        assert_eq!(
            hover(&h, &mut cx, "tag-chip-slow").as_deref(),
            Some("Not filtered. Click to include.")
        );
    }

    #[gpui::test]
    fn its_controls_take_no_click_while_busy_or_running(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 1200., clean(smoke_not_wip(false)));
        h.update(&mut cx, |s| s.busy = true);
        for id in ["tag-chip-smoke", "tag-clear", "hide-non-matching"] {
            click(&mut cx, id);
        }
        h.update(&mut cx, |s| {
            s.busy = false;
            s.run.active = true;
        });
        click(&mut cx, "tag-chip-slow");
        assert!(calls(&h).is_empty(), "{:?}", calls(&h));
        h.update(&mut cx, |s| s.run.active = false);
        click(&mut cx, "tag-chip-slow");
        assert_eq!(calls(&h).len(), 1);
    }

    #[gpui::test]
    fn shows_a_tag_of_the_filter_that_left_the_project(cx: &mut TestAppContext) {
        let (_, mut cx, _) = setup(cx, 1200., clean(filter(&["gone"], &[], false)));
        assert!(at(&mut cx, "tag-chip-gone").is_some());
    }

    #[gpui::test]
    fn is_absent_without_a_tree_or_without_tags(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 1200., clean(smoke_not_wip(false)));
        assert!(at(&mut cx, "filter-bar").is_some());
        // the runner still holds the old filter after a failed discovery
        h.update(&mut cx, |s| {
            s.project = None;
            s.tree = None;
        });
        assert!(at(&mut cx, "filter-bar").is_none());
    }

    #[gpui::test]
    fn is_absent_in_a_project_without_tags(cx: &mut TestAppContext) {
        let mut s = testing::project("D:/p", Layout::Standalone, 3);
        s.tree.as_mut().unwrap().children[0].children[0].tags.clear();
        let (_, mut cx, _) = setup(cx, 1200., s);
        assert!(at(&mut cx, "filter-bar").is_none());
    }
}

mod folding {
    use super::*;

    const NARROW: f32 = 700.;

    #[gpui::test]
    fn folds_the_chips_past_the_line_and_keeps_the_bar_one_line(cx: &mut TestAppContext) {
        let (_, mut cx, bar) = setup(cx, NARROW, many(40));
        let folded = folded(&bar, &mut cx);
        assert!(!folded.is_empty() && folded.len() < 40, "{folded:?}");
        // the last ones, in order
        let tags: Vec<String> = (0..40).map(|i| format!("tag-{i:02}")).collect();
        assert_eq!(folded, tags[40 - folded.len()..]);
        assert_eq!(at(&mut cx, "filter-bar").unwrap().size.height, px(BAR_HEIGHT));
        let more = at(&mut cx, "tag-more").unwrap();
        let last_shown = &tags[39 - folded.len()];
        assert!(at(&mut cx, &format!("tag-chip-{last_shown}")).unwrap().right() < more.left());
        assert!(more.right() < at(&mut cx, "hide-non-matching").unwrap().left());
    }

    #[gpui::test]
    fn has_no_fold_chip_when_every_chip_fits(cx: &mut TestAppContext) {
        let (_, mut cx, bar) = setup(cx, 1400., many(4));
        assert!(folded(&bar, &mut cx).is_empty());
        assert!(at(&mut cx, "tag-more").is_none());
    }

    #[gpui::test]
    fn opens_a_panel_of_every_tag_that_cycles_them_and_stays_open(cx: &mut TestAppContext) {
        let (h, mut cx, bar) = setup(cx, NARROW, many(40));
        assert!(at(&mut cx, "tag-panel").is_none());
        assert_eq!(hover(&h, &mut cx, "tag-more").as_deref(), Some(SHOW_ALL_TAGS));
        click(&mut cx, "tag-more");
        assert!(is_open(&bar, &mut cx));
        let panel = at(&mut cx, "tag-panel").unwrap();
        assert!(panel.top() >= at(&mut cx, "filter-bar").unwrap().bottom() - px(1.));
        for i in [0, 39] {
            assert!(at(&mut cx, &format!("tag-panel-chip-tag-{i:02}")).is_some());
        }
        click(&mut cx, "tag-panel-chip-tag-39");
        assert_eq!(calls(&h), [sent(&filter(&["tag-39"], &[], false))]);
        assert!(is_open(&bar, &mut cx));
        // a second click on the fold chip closes it, and its chips go
        click(&mut cx, "tag-more");
        assert!(!is_open(&bar, &mut cx));
        assert!(at(&mut cx, "tag-panel-chip-tag-00").is_none());
    }

    #[gpui::test]
    fn the_panel_closes_on_a_click_outside_and_on_escape(cx: &mut TestAppContext) {
        let (_, mut cx, bar) = setup(cx, NARROW, many(40));
        click(&mut cx, "tag-more");
        cx.simulate_click(point(px(5.), px(650.)), Modifiers::default());
        cx.run_until_parked();
        assert!(!is_open(&bar, &mut cx));
        click(&mut cx, "tag-more");
        assert!(is_open(&bar, &mut cx));
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        assert!(!is_open(&bar, &mut cx));
    }

    #[gpui::test]
    fn the_panel_closes_when_nothing_is_folded(cx: &mut TestAppContext) {
        let (_, mut cx, bar) = setup(cx, NARROW, many(12));
        assert!(!folded(&bar, &mut cx).is_empty());
        click(&mut cx, "tag-more");
        cx.simulate_resize(size(px(3000.), px(700.)));
        cx.run_until_parked();
        assert!(folded(&bar, &mut cx).is_empty());
        assert!(!is_open(&bar, &mut cx));
    }

    #[gpui::test]
    fn during_a_run_the_panel_opens_with_its_chips_disabled(cx: &mut TestAppContext) {
        let (h, mut cx, bar) = setup(cx, NARROW, many(40));
        h.update(&mut cx, |s| s.busy = true);
        click(&mut cx, "tag-more");
        assert!(is_open(&bar, &mut cx));
        click(&mut cx, "tag-panel-chip-tag-00");
        assert!(calls(&h).is_empty());
    }

    #[gpui::test]
    fn counts_the_folded_tags_that_are_on(cx: &mut TestAppContext) {
        let mut s = many(40);
        s.filter = filter(&[], &["tag-39"], false);
        let (_, mut cx, bar) = setup(cx, NARROW, s);
        assert!(folded(&bar, &mut cx).contains(&"tag-39".to_owned()));
        // the fold chip's name, as `render` builds it
        let n = folded(&bar, &mut cx).len();
        assert_eq!(fold_name(n, 1), format!("{n} more tags, 1 on"));
    }
}
