//! `e2e/look.spec.ts`'s focus ring and target size (PU-23, PU-24, E-look-11, E-look-12), through the accessibility
//! tree, with the ring read off the window's pixels where there is a pixel reader (Linux, through ImageMagick).
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Key, Node, Rect, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);

/// The tree item whose name starts with `label` and a comma.
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

fn focused(tree: &Node) -> Option<Node> {
    tree.all().into_iter().find(|n| n.states.focused).cloned()
}

fn key(n: &Node) -> String {
    format!("{:?} {}", n.role, n.name)
}

/// How far out from a control's edge the band read for its ring goes, and how far in: GPUI Kit's ring is 3 px
/// outside the control, the tree's 2 px inside its row.
const OUT: i32 = 4;
const IN: i32 = 2;

/// The window's pixels in `r` grown by `OUT`, row by row, as `rrggbb`; `None` where there is no pixel reader.
#[cfg(target_os = "linux")]
fn band(r: Rect) -> Option<Vec<u32>> {
    use std::process::Command;
    let id = Command::new("xdotool")
        .args(["search", "--onlyvisible", "--name", "^Grace$"])
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&id.stdout).lines().next()?.to_string();
    let (w, h) = (r.width + 2 * OUT, r.height + 2 * OUT);
    // a crop that starts left of or above the window is cut there: outside it the pixels stay 0, on both sides
    let (x, y) = (r.x - OUT, r.y - OUT);
    let out = Command::new("import")
        .args([
            "-window",
            &id,
            "-crop",
            &format!("{}x{}+{}+{}", w + x.min(0), h + y.min(0), x.max(0), y.max(0)),
            "+repage",
            "txt:-",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    let mut px = vec![0u32; (w * h) as usize];
    for line in text.lines().skip(1) {
        let Some((at, rest)) = line.split_once(':') else {
            continue;
        };
        let mut xy = at.split(',').map(|v| v.trim().parse::<i32>().ok());
        let (Some(Some(a)), Some(Some(b))) = (xy.next(), xy.next()) else {
            continue;
        };
        let (a, b) = (a - x.min(0), b - y.min(0));
        // `#f3f3efefe6e6`: 16 bits a channel, of which the high byte is the 8-bit value
        let Some(hash) = rest.find('#') else { continue };
        let hex: String = rest[hash + 1..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        let step = hex.len() / 3;
        if step != 2 && step != 4 {
            continue;
        }
        let ch = |i: usize| u32::from_str_radix(&hex[i * step..i * step + 2], 16).unwrap_or(0);
        if (0..w).contains(&a) && (0..h).contains(&b) {
            px[(b * w + a) as usize] = (ch(0) << 16) | (ch(1) << 8) | ch(2);
        }
    }
    Some(px)
}

#[cfg(not(target_os = "linux"))]
fn band(_: Rect) -> Option<Vec<u32>> {
    None
}

/// Of each side of `r` (top, right, bottom, left), the share of its length where its band, from `OUT` out to `IN`
/// in, changed between `before` and `after`.
fn changed_sides(r: Rect, before: &[u32], after: &[u32]) -> [f32; 4] {
    let w = r.width + 2 * OUT;
    let differs = |x: i32, y: i32| {
        let (a, b) = (before[(y * w + x) as usize], after[(y * w + x) as usize]);
        let ch = |v: u32, s: u32| ((v >> s) & 0xff) as i32;
        [16, 8, 0].iter().any(|&s| (ch(a, s) - ch(b, s)).abs() > 8)
    };
    let (bottom, right) = (OUT + r.height, OUT + r.width);
    let across = |ys: std::ops::Range<i32>| {
        let n = (OUT..right).filter(|&x| ys.clone().any(|y| differs(x, y))).count();
        n as f32 / r.width.max(1) as f32
    };
    let down = |xs: std::ops::Range<i32>| {
        let n = (OUT..bottom).filter(|&y| xs.clone().any(|x| differs(x, y))).count();
        n as f32 / r.height.max(1) as f32
    };
    [
        across(0..OUT + IN),
        down(right - IN..right + OUT),
        across(bottom - IN..bottom + OUT),
        down(0..OUT + IN),
    ]
}

/// A ring changes the band round a control on three sides, or along the whole of two opposite ones: a tooltip,
/// which shows at once on keyboard focus, changes one side only. A ring is cut where the control meets the edge of
/// its pane or the window (the splitter's ends), and goes round a box wider than the control's node (the thread
/// count's buttons), so one or two sides may show none.
/// A checkbox's box, at the left of its row and on its middle: the ring goes round the box, as a browser's
/// `:focus-visible` goes round the `<input>` (`elements::checkbox`), where the node is the whole row with its label.
const CHECKBOX: i32 = 13;

/// Where a stop's ring is: a checkbox's round its box, any other control's round the control.
fn ringed(stop: &Node, r: Rect) -> Rect {
    if stop.role != Role::CheckBox {
        return r;
    }
    Rect {
        x: r.x,
        y: r.y + (r.height - CHECKBOX) / 2,
        width: CHECKBOX,
        height: CHECKBOX,
    }
}

fn shows_a_ring(sides: [f32; 4]) -> bool {
    let [top, right, bottom, left] = sides;
    sides.iter().filter(|s| **s >= 0.25).count() >= 3
        || (top >= 0.75 && bottom >= 0.75)
        || (left >= 0.75 && right >= 0.75)
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn every_control_shows_the_focus_ring_in_light_mode() {
    every_control_shows_the_focus_ring("light");
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn every_control_shows_the_focus_ring_in_dark_mode() {
    every_control_shows_the_focus_ring("dark");
}

/// `look.spec.ts` "Every control shows the focus ring in ${scheme} mode": Tab through the whole window after a run,
/// with the details holding buttons; every stop shows a ring, the tree is one of them. Done twice: the stops first,
/// then each one's band before and after Tab reaches it.
fn every_control_shows_the_focus_ring(theme: &str) {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(
        &json!({ "lastProject": dir.to_string_lossy(), "appearance": { "theme": theme } }),
        &[],
    )
    .unwrap();
    app.wait_for("the tests", WAIT, |t| {
        t.all().into_iter().any(|n| n.name == "9 tests · Not run yet")
    })
    .unwrap();
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    app.press_named(Role::Button, "Run all").unwrap();
    let tree = app
        .wait_for("the run to finish", RUN, |t| {
            t.all().into_iter().any(|n| n.name.starts_with("Finished"))
                && row(t, "failing match").is_some_and(|r| r.name.contains(", failed"))
        })
        .unwrap();
    // the details have buttons; the click leaves the tree's ring off, so Tab must bring it back (PU-23)
    app.click(row(&tree, "failing match").unwrap()).unwrap();
    app.wait_for("the details", WAIT, |t| {
        t.all().into_iter().any(|n| n.name == "tests/basic.feature:15")
    })
    .unwrap();

    // the stops, once round
    let mut stops: Vec<Node> = Vec::new();
    let mut at = String::new();
    for _ in 0..80 {
        app.key(Key::Tab).unwrap();
        let tree = app
            .wait_for("the focus to move", WAIT, |t| focused(t).is_some_and(|f| key(&f) != at))
            .unwrap();
        let now = focused(&tree).unwrap();
        at = key(&now);
        if stops.first().is_some_and(|f| key(f) == at) {
            break;
        }
        stops.push(now);
    }
    let names: Vec<String> = stops.iter().map(key).collect();
    assert!(stops.len() > 17, "the whole window, not the first few: {names:#?}");
    assert_eq!(
        stops.iter().filter(|s| s.role == Role::TreeItem).count(),
        1,
        "the tree is one stop: {names:#?}"
    );
    // a stop in each area: the toolbar, the tree, the panel's splitter and tabs, the status bar
    for (role, name) in [
        (Role::Button, "Home"),
        (Role::Button, "Run all"),
        (Role::TreeItem, "failing match, "),
        (Role::Other("Separator".into()), "Resize bottom panel"),
        (Role::Tab, "Console"),
        (Role::Button, "Settings"),
    ] {
        assert!(
            stops.iter().any(|s| s.role == role && s.name.starts_with(name)),
            "{role:?} {name} in {names:#?}"
        );
    }
    assert!(
        stops.iter().any(|s| s.role == Role::Button && s.name.starts_with('@')),
        "a tag chip in {names:#?}"
    );

    // each stop's band before Tab reaches it and once it has: the focus is on the first stop again
    if band(stops[0].rect.unwrap()).is_none() {
        return; // no pixel reader here
    }
    let mut missing = Vec::new();
    for i in (1..=stops.len()).map(|k| k % stops.len()) {
        let stop = &stops[i];
        let r = ringed(stop, stop.rect.unwrap_or_else(|| panic!("{} has no place", key(stop))));
        let before = band(r).unwrap();
        app.key(Key::Tab).unwrap();
        app.wait_for(&format!("the focus on {}", key(stop)), WAIT, |t| {
            focused(t).is_some_and(|f| key(&f) == key(stop))
        })
        .unwrap();
        // the frame with the ring follows the focus: wait for it, and keep the last count for the message
        let last = std::cell::Cell::new([0.; 4]);
        let shown = app
            .wait_until(&format!("the ring on {}", key(stop)), Duration::from_secs(3), || {
                let sides = changed_sides(r, &before, &band(r).unwrap());
                last.set(sides);
                shows_a_ring(sides)
            })
            .is_ok();
        if !shown {
            missing.push(format!(
                "{} {r:?}: changed {:?} (top, right, bottom, left)",
                key(stop),
                last.get()
            ));
        }
    }
    assert!(missing.is_empty(), "no ring on: {missing:#?}");
}

/// `look.spec.ts` "Targets are at least 24 by 24": the recent list's remove buttons, the tree's run buttons, the
/// panel's tabs and the tag chips.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn targets_are_at_least_24_by_24() {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "recentProjects": [dir.to_string_lossy()] }), &[]).unwrap();
    let at_least_24 = |what: &str, tree: &Node, pick: &dyn Fn(&Node) -> bool| {
        let all: Vec<&Node> = tree.all().into_iter().filter(|n| pick(n)).collect();
        assert!(!all.is_empty(), "no {what}: {}", tree.dump());
        for n in all {
            let r = n.rect.unwrap_or_else(|| panic!("{} has no place", key(n)));
            assert!(r.width >= 24 && r.height >= 24, "{what}: {} is {r:?}", key(n));
        }
    };
    let tree = app
        .wait_for("the recent list", WAIT, |t| {
            t.find(&Role::Button, "Remove from list").is_some()
        })
        .unwrap();
    at_least_24("remove buttons", &tree, &|n| {
        n.role == Role::Button && n.name == "Remove from list"
    });

    let path = dir.to_string_lossy().to_string();
    let recent = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.ends_with(&path))
        .cloned()
        .unwrap_or_else(|| panic!("{}", tree.dump()));
    app.press(&recent).unwrap();
    app.wait_for("the tests", WAIT, |t| {
        t.all().into_iter().any(|n| n.name == "9 tests · Not run yet")
    })
    .unwrap();
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    let tree = app
        .wait_for("the scenarios", WAIT, |t| row(t, "failing match").is_some())
        .unwrap();
    at_least_24("run buttons", &tree, &|n| {
        n.role == Role::Button && n.name.starts_with("Run ") && n.name != "Run all"
    });
    at_least_24("tabs", &tree, &|n| n.role == Role::Tab);
    at_least_24("tag chips", &tree, &|n| {
        n.role == Role::Button && n.name.starts_with('@')
    });
}
