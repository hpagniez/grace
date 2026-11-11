//! `Progress.test.tsx` and the loading spec's scenarios (LV-01 to LV-08), headless.
use super::*;
use crate::testing::{Harness, open, project, run};
use gpui::{AppContext as _, TestAppContext, VisualTestContext};
use grace_core::model::events::{BuildTool, Paused, Summary};
use grace_core::model::node::Layout;

fn loaded(tests: usize) -> AppState {
    project("D:\\work\\api", Layout::Maven, tests)
}

/// A run of `total` tests started now.
fn started(s: &mut AppState, total: u32) {
    s.run = Default::default();
    s.run.active = true;
    s.run.total = total;
    s.run.run_id = Some("r1".into());
    s.run.started_at = Some(now_ms());
}

fn result(s: &mut AppState, passed: bool) {
    s.run.done += 1;
    if passed {
        s.run.passed += 1;
    } else {
        s.run.failed += 1;
    }
}

fn finished(s: &mut AppState, outcome: RunOutcome) {
    s.run.active = false;
    s.run.outcome = Some(outcome);
}

fn paused() -> Paused {
    serde_json::from_value(serde_json::json!({
        "key": "s:a.feature#1", "file": "a.feature", "line": 2, "depth": 0,
        "prefix": "*", "keyword": "def", "text": "a = 1", "vars": []
    }))
    .unwrap()
}

/// The progress view in a window.
fn view(cx: &mut TestAppContext, state: AppState) -> (Harness, Entity<Progress>, VisualTestContext) {
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<Progress>>>> = Default::default();
    let keep = slot.clone();
    let (h, vcx) = open(cx, 1200., state, move |grace, _, cx| {
        let p = keep
            .borrow_mut()
            .get_or_insert_with(|| cx.new(|cx| Progress::new(grace.clone(), cx)))
            .clone();
        p.into_any_element()
    });
    vcx.run_until_parked();
    let p = slot.borrow().clone().unwrap();
    (h, p, vcx)
}

mod before_any_run_r14_3 {
    use super::*;

    #[test]
    fn counts_the_projects_tests_and_says_not_run_yet_with_no_zero_counts() {
        let s = loaded(24);
        assert!(idle(&s));
        assert_eq!(count_text(&s), "24 tests · Not run yet");
    }

    #[gpui::test]
    fn draws_no_counts_and_no_time_before_a_run(cx: &mut TestAppContext) {
        let (_, _, mut cx) = view(cx, loaded(24));
        assert!(cx.debug_bounds("progress-text").is_some());
        for id in [
            "progress-passed",
            "progress-failed",
            "progress-elapsed",
            "run-outcome",
            "run-env",
        ] {
            assert!(cx.debug_bounds(id).is_none(), "{id}");
        }
    }

    #[test]
    fn says_test_for_one() {
        assert_eq!(count_text(&loaded(1)), "1 test · Not run yet");
    }

    #[test]
    fn only_counts_the_tests_once_the_project_has_runs_in_its_history() {
        let mut s = loaded(24);
        s.history = vec![run("r1", 24, 0)];
        assert_eq!(count_text(&s), "24 tests");
        s.history.clear();
        assert_eq!(count_text(&s), "24 tests · Not run yet");
    }

    #[test]
    fn keeps_the_progress_bar_and_no_outcome() {
        let s = loaded(3);
        assert_eq!(outcome(&s), None);
        assert!(!waiting(&s));
    }

    #[test]
    fn is_over_once_a_run_has_started_and_also_once_an_outcome_is_in() {
        let mut s = loaded(4);
        started(&mut s, 4);
        assert_eq!(count_text(&s), "0 / 4");
        let mut refused = loaded(4);
        finished(&mut refused, RunOutcome::Failed); // refused before it started
        assert_eq!(count_text(&refused), "0 / 0");
        assert_eq!(outcome(&refused).unwrap().text, "Run failed");
    }
}

mod after_a_run_r14_2 {
    use super::*;

    #[gpui::test]
    fn shows_done_of_total_and_each_count_behind_its_mark_and_word(cx: &mut TestAppContext) {
        let mut s = loaded(4);
        started(&mut s, 4);
        result(&mut s, true);
        result(&mut s, false);
        assert_eq!(count_text(&s), "2 / 4");
        let (_, _, mut cx) = view(cx, s);
        for id in ["progress-passed", "progress-failed", "progress-elapsed"] {
            assert!(cx.debug_bounds(id).is_some(), "{id}");
        }
    }

    #[test]
    fn shows_the_elapsed_time_on_two_digits_each() {
        assert_eq!(mmss(0), "00:00");
        assert_eq!(mmss(3_999), "00:03");
        assert_eq!(mmss(61_000), "01:01");
        assert_eq!(mmss(3_600_000), "60:00");
    }

    #[test]
    fn names_the_outcome_with_its_icon() {
        let mut s = loaded(2);
        started(&mut s, 2);
        assert_eq!(
            outcome(&s).unwrap(),
            Outcome {
                text: "Running…".into(),
                icon: Icon::LoaderCircle,
                tone: Tone::Running,
                turning: true
            }
        );
        for (o, word, icon, tone) in [
            (RunOutcome::Finished, "Finished", Icon::CircleCheck, Tone::Muted),
            (RunOutcome::Cancelled, "Cancelled", Icon::CircleSlash, Tone::Cancelled),
            (RunOutcome::Failed, "Run failed", Icon::CircleAlert, Tone::Failed),
        ] {
            started(&mut s, 2);
            finished(&mut s, o);
            let got = outcome(&s).unwrap();
            assert_eq!(
                (got.text.as_str(), got.icon, got.tone, got.turning),
                (word, icon, tone, false)
            );
        }
    }

    #[test]
    fn shows_finding_tests_with_the_turning_icon_while_the_runner_discovers() {
        let mut s = loaded(2);
        s.busy = true;
        let o = outcome(&s).unwrap();
        assert_eq!((o.text.as_str(), o.turning), ("Finding tests…", true));
    }

    #[test]
    fn says_which_build_tool_lists_the_libraries_then_finding_tests() {
        let mut s = loaded(2);
        s.busy = true;
        s.listing = Some(BuildTool::Gradle);
        assert_eq!(
            outcome(&s).unwrap().text,
            "Listing the project's libraries with Gradle…"
        );
        s.listing = None;
        assert_eq!(outcome(&s).unwrap().text, "Finding tests…");
    }

    #[test]
    fn says_when_the_build_tool_builds_the_projects_java_code() {
        let mut s = loaded(2);
        s.busy = true;
        s.listing = Some(BuildTool::Maven);
        s.building = true;
        assert_eq!(
            outcome(&s).unwrap().text,
            "Building the project's Java code with Maven…"
        );
    }

    #[gpui::test]
    fn shows_the_runs_environment_chip_hides_it_when_there_is_none_and_explains_it(cx: &mut TestAppContext) {
        let mut s = loaded(2);
        started(&mut s, 2);
        let (h, _, mut cx) = view(cx, s);
        assert!(cx.debug_bounds("run-env").is_none());
        h.update(&mut cx, |s| s.run.env = Some("qa".into()));
        let chip = cx.debug_bounds("run-env").unwrap();
        cx.simulate_mouse_move(chip.center(), None, gpui::Modifiers::none());
        cx.executor()
            .advance_clock(crate::tooltip::DELAY + Duration::from_millis(1));
        cx.run_until_parked();
        assert_eq!(h.tip(&mut cx).as_deref(), Some(ENV_TIP));
    }
}

mod the_bar_r14_1 {
    use super::*;

    #[gpui::test]
    fn is_6_px_with_2_px_gaps_between_passed_failed_and_cancelled(cx: &mut TestAppContext) {
        let mut s = loaded(4);
        started(&mut s, 4);
        for passed in [true, false] {
            result(&mut s, passed);
        }
        s.run.interrupted = 1;
        let (_, _, mut cx) = view(cx, s);
        // the springs settle
        cx.executor().advance_clock(Duration::from_secs(2));
        cx.run_until_parked();
        let bar = cx.debug_bounds("progress-bar").unwrap();
        assert_eq!(bar.size.height, px(6.));
        let seg: Vec<_> = (0..3)
            .map(|i| {
                cx.debug_bounds(Box::leak(format!("progress-segment-{i}").into_boxed_str()))
                    .unwrap()
            })
            .collect();
        assert_eq!(seg[1].left() - seg[0].right(), px(2.));
        assert_eq!(seg[2].left() - seg[1].right(), px(2.));
    }

    /// The fill ends with the track: a full bar draws only its non-empty segments, the first from the track's left
    /// end and the last to its right end, where the rounded ends are (#321).
    #[gpui::test]
    fn a_full_bar_fills_the_track_from_end_to_end(cx: &mut TestAppContext) {
        let mut s = loaded(4);
        started(&mut s, 4);
        for passed in [true, true, true, false] {
            result(&mut s, passed);
        }
        finished(&mut s, RunOutcome::Finished);
        let (_, _, mut cx) = view(cx, s);
        cx.executor().advance_clock(Duration::from_secs(2));
        cx.run_until_parked();
        let bar = cx.debug_bounds("progress-bar").unwrap();
        let passed = cx.debug_bounds("progress-segment-0").unwrap();
        let failed = cx.debug_bounds("progress-segment-1").unwrap();
        assert!(
            cx.debug_bounds("progress-segment-2").is_none(),
            "no empty cancelled segment"
        );
        assert_eq!(passed.left(), bar.left());
        assert_eq!(failed.left() - passed.right(), px(2.));
        assert!(
            (f32::from(failed.right() - bar.right())).abs() < 0.5,
            "{failed:?} ends at {bar:?}"
        );
        assert!(
            passed.size.width > failed.size.width * 2.5,
            "{passed:?} is about three times {failed:?}"
        );
    }

    /// #144: on a big run the passed part grows with each update, every 100 ms, though each one moves its target.
    #[gpui::test]
    fn the_passed_part_fills_during_a_big_run(cx: &mut TestAppContext) {
        let mut s = loaded(10_000);
        started(&mut s, 10_000);
        let (h, _, mut cx) = view(cx, s);
        let passed = |cx: &mut VisualTestContext| {
            cx.debug_bounds("progress-segment-0")
                .map_or(0., |b| f32::from(b.size.width))
        };
        let mut last = 0.;
        for _ in 0..20 {
            h.update(&mut cx, |s| {
                for i in 0..200 {
                    result(s, i % 50 != 0);
                }
            });
            // 100 ms of frames, as the window draws them while a spring moves
            for _ in 0..6 {
                cx.executor().advance_clock(Duration::from_millis(16));
                cx.update(|window, _| window.refresh());
                cx.run_until_parked();
            }
            let now = passed(&mut cx);
            assert!(now > last, "the passed part grew from {last} to {now}");
            last = now;
        }
        let bar = f32::from(cx.debug_bounds("progress-bar").unwrap().size.width);
        assert!(last > bar * 0.3, "{last} of {bar} px for 39 % passed");
    }

    /// #320: the springs move by the time between the view's frames and say whether they still move, so the bar
    /// asks for a next frame only then; a first frame, and every frame with reduced motion, draws the ends.
    #[test]
    fn the_springs_move_by_the_time_between_frames_until_they_rest() {
        let ms = Duration::from_millis;
        let t0 = Instant::now();
        let mut bar = Bar::default();
        assert!(!bar.step([10., 0., 0.], t0, false), "a first frame draws the ends");
        assert_eq!(bar.springs[0].position, 10.);
        assert!(bar.step([20., 5., 0.], t0, false), "new ends: the segments move");
        bar.step([20., 5., 0.], t0 + ms(100), false);
        let (passed, failed) = (bar.springs[0].position, bar.springs[1].position);
        assert!(passed > 10. && passed < 20., "{passed}");
        assert!(failed > 0. && failed < 5., "{failed}");
        let speed = bar.springs[0].velocity;
        bar.step([30., 5., 0.], t0 + ms(100), false);
        assert_eq!(bar.springs[0].velocity, speed, "a new end keeps the speed");
        assert!(!bar.step([30., 5., 0.], t0 + ms(2_100), false), "at rest");
        assert_eq!(bar.springs[0].position, 30.);
        assert!(!bar.step([50., 0., 0.], t0 + ms(2_150), true), "reduced motion");
        assert_eq!(bar.springs[0].position, 50.);
    }

    /// #320: a moving bar asks for its frame at the next step of the marks' clock, and a bar at rest for none.
    #[gpui::test]
    fn a_moving_bar_asks_for_its_next_step_and_a_resting_one_for_none(cx: &mut TestAppContext) {
        let mut s = loaded(4);
        started(&mut s, 4);
        let (h, _, mut cx) = view(cx, s);
        h.update(&mut cx, |s| {
            for passed in [true, true, false, true] {
                result(s, passed);
            }
            finished(s, RunOutcome::Finished);
        });
        assert_eq!(cx.update(|_, cx| crate::spin::asked(cx)), 1, "the bar moves");
        for _ in 0..40 {
            cx.executor().advance_clock(crate::spin::TURN / crate::spin::STEPS);
            cx.run_until_parked();
        }
        assert_eq!(cx.update(|_, cx| crate::spin::asked(cx)), 0, "at rest, no frame asked");
        let bar = cx.debug_bounds("progress-bar").unwrap();
        let passed = cx.debug_bounds("progress-segment-0").unwrap();
        assert!(
            (f32::from(passed.size.width) - f32::from(bar.size.width) * 0.75).abs() < 4.,
            "{passed:?} is three quarters of {bar:?}"
        );
    }

    #[test]
    fn widths_are_each_counts_share_of_the_total() {
        let mut s = loaded(4);
        started(&mut s, 4);
        result(&mut s, true);
        result(&mut s, false);
        s.run.interrupted = 2;
        assert_eq!(widths(&s), [25., 25., 50.]);
        assert_eq!(widths(&loaded(4)), [0., 0., 0.]);
    }

    /// LV-01, LV-03: finding tests shows the stripe; found, it goes.
    #[gpui::test]
    fn finding_tests_shows_the_stripe_until_the_runner_is_no_longer_busy(cx: &mut TestAppContext) {
        let mut s = loaded(2);
        s.busy = true;
        assert!(waiting(&s));
        let (h, _, mut cx) = view(cx, s);
        assert!(cx.debug_bounds("progress-stripe").is_some());
        h.update(&mut cx, |s| s.busy = false);
        assert!(cx.debug_bounds("progress-stripe").is_none());
    }

    /// LV-04: the stripe never covers a run's results; they show again after.
    #[test]
    fn the_stripe_hides_the_last_runs_segments_and_they_show_again_after() {
        let mut s = loaded(2);
        started(&mut s, 2);
        result(&mut s, true);
        result(&mut s, false);
        finished(&mut s, RunOutcome::Finished);
        assert!(!waiting(&s));
        s.busy = true;
        assert!(waiting(&s));
        assert_eq!(widths(&s), [50., 50., 0.]); // kept, under the stripe
        s.busy = false;
        assert!(!waiting(&s));
    }

    /// LV-05, LV-07: a run waits for its first result; cancelled, the stripe stops.
    #[gpui::test]
    fn the_stripe_stays_until_the_first_result_and_a_cancel_stops_it(cx: &mut TestAppContext) {
        let mut s = loaded(4);
        started(&mut s, 4);
        let (h, _, mut cx) = view(cx, s);
        assert!(cx.debug_bounds("progress-stripe").is_some());
        h.update(&mut cx, |s| result(s, true));
        assert!(cx.debug_bounds("progress-stripe").is_none());
        assert_eq!(count_text(&h.grace.read_with(&cx, |g, _| g.state.clone())), "1 / 4");
        h.update(&mut cx, |s| started(s, 4));
        assert!(cx.debug_bounds("progress-stripe").is_some());
        h.update(&mut cx, |s| finished(s, RunOutcome::Cancelled));
        assert!(cx.debug_bounds("progress-stripe").is_none());
    }

    /// LV-08: with reduced motion the stripe sits still at the start of the track.
    #[gpui::test]
    fn reduced_motion_keeps_the_stripe_still(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_reduce_motion(true));
        let mut s = loaded(2);
        s.busy = true;
        let (_, _, mut cx) = view(cx, s);
        let bar = cx.debug_bounds("progress-bar").unwrap();
        let first = cx.debug_bounds("progress-stripe").unwrap();
        assert_eq!(first.left(), bar.left());
        cx.executor().advance_clock(Duration::from_millis(700));
        cx.run_until_parked();
        assert_eq!(cx.debug_bounds("progress-stripe").unwrap().left(), bar.left());
    }
}

mod the_clock {
    use super::*;

    #[gpui::test]
    fn runs_only_during_a_run_and_shows_the_summarys_time_after(cx: &mut TestAppContext) {
        let (h, p, mut cx) = view(cx, loaded(2));
        p.read_with(&cx, |p, _| assert!(!p.clock_running()));
        h.update(&mut cx, |s| {
            started(s, 2);
            s.run.started_at = Some(now_ms() - 3_000.);
        });
        p.read_with(&cx, |p, cx| {
            assert!(p.clock_running());
            let state = p.grace.read(cx).state.clone();
            assert!((3_000..4_000).contains(&p.elapsed(&state)), "{}", p.elapsed(&state));
        });
        h.update(&mut cx, |s| {
            finished(s, RunOutcome::Finished);
            s.run.summary = Some(Summary {
                scenario_count: 2,
                scenarios_passed: 2,
                scenarios_failed: 0,
                duration_millis: 61_500.,
            });
        });
        p.read_with(&cx, |p, cx| {
            assert!(!p.clock_running());
            let state = p.grace.read(cx).state.clone();
            assert_eq!(mmss(p.elapsed(&state)), "01:01");
        });
    }

    /// Step debugger R7.2: paused, it says Paused, turns nothing, and holds its time.
    #[gpui::test]
    fn a_paused_debug_run_says_paused_turns_nothing_and_stops_the_clock(cx: &mut TestAppContext) {
        let mut s = loaded(2);
        started(&mut s, 2);
        s.run.debug = true;
        let (h, p, mut cx) = view(cx, s);
        p.read_with(&cx, |p, _| assert!(p.clock_running()));
        h.update(&mut cx, |s| s.paused = Some(paused()));
        let state = h.grace.read_with(&cx, |g, _| g.state.clone());
        let o = outcome(&state).unwrap();
        assert_eq!((o.text.as_str(), o.icon, o.turning), ("Paused", Icon::Pause, false));
        assert!(!waiting(&state));
        assert!(cx.debug_bounds("progress-stripe").is_none());
        p.read_with(&cx, |p, _| assert!(!p.clock_running()));
        h.update(&mut cx, |s| s.paused = None);
        p.read_with(&cx, |p, _| assert!(p.clock_running()));
        assert_eq!(
            outcome(&h.grace.read_with(&cx, |g, _| g.state.clone())).unwrap().text,
            "Running…"
        );
    }
}
