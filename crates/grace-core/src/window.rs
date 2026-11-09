//! `src/main/window.ts`: where the saved window goes on the screens there are now. Pure: the saved bounds and the
//! work areas come in as plain data, so the rule runs and tests without a window. Electron's `Rectangle` is
//! `Rectangle` here, and the numbers stay `f64` like JavaScript's: a file can hold fractions, the answer never does.

/// A window's or a work area's bounds, in the screen's logical pixels (Electron's DIP). `y` grows downwards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rectangle {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }
}

/// Windows' invisible resize border, in DIP.
const BORDER: f64 = 8.0;

/// JavaScript's `Math.round`: halves go up (`-2.5` is `-2`), where `f64::round` takes them away from zero.
fn round(n: f64) -> f64 {
    let floor = n.floor();
    if n - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// The area two rectangles share, 0 when they do not touch.
fn overlap(a: Rectangle, b: Rectangle) -> f64 {
    let width = ((a.x + a.width).min(b.x + b.width) - a.x.max(b.x)).max(0.0);
    let height = ((a.y + a.height).min(b.y + b.height) - a.y.max(b.y)).max(0.0);
    width * height
}

/// Where the saved window goes: onto the work area it overlaps most (the first on a tie), shrunk to fit and moved
/// fully inside it. `None` when it overlaps none: the window opens as on a first start.
pub fn place_window(saved: Rectangle, work_areas: &[Rectangle]) -> Option<Rectangle> {
    let mut best: Option<Rectangle> = None;
    let mut most = 0.0;
    for &area in work_areas {
        let o = overlap(saved, area);
        if o > most {
            best = Some(area);
            most = o;
        }
    }
    let a = best?;
    // Windows' invisible resize border: a window flush with a screen edge (snapped, say) sticks out
    // by up to 8 DIP on the left, right and bottom. It stays where it is.
    let flush = saved.x >= a.x - BORDER
        && saved.x + saved.width <= a.x + a.width + BORDER
        && saved.y >= a.y
        && saved.y + saved.height <= a.y + a.height + BORDER;
    if flush {
        return Some(Rectangle::new(
            round(saved.x),
            round(saved.y),
            round(saved.width),
            round(saved.height),
        ));
    }
    let width = round(saved.width).min(a.width);
    let height = round(saved.height).min(a.height);
    let inside = |pos: f64, size: f64, start: f64, room: f64| round(pos).max(start).min(start + room - size);
    Some(Rectangle::new(
        inside(saved.x, width, a.x, a.width),
        inside(saved.y, height, a.y, a.height),
        width,
        height,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn rect(x: f64, y: f64, width: f64, height: f64) -> Rectangle {
        Rectangle::new(x, y, width, height)
    }
    const MAIN: Rectangle = rect(0.0, 0.0, 1920.0, 1040.0);
    const SECOND: Rectangle = rect(1920.0, 0.0, 2560.0, 1400.0);
    const AREAS: [Rectangle; 2] = [MAIN, SECOND];

    /// One test per row of the TypeScript's `it.each` ("places %o at %o on two screens").
    mod places_on_two_screens {
        use super::*;

        fn check(saved: Rectangle, placed: Option<Rectangle>) {
            assert_eq!(place_window(saved, &AREAS), placed);
        }

        #[test]
        fn a_window_inside_the_second_screen_stays() {
            check(
                rect(2000.0, 100.0, 1200.0, 800.0),
                Some(rect(2000.0, 100.0, 1200.0, 800.0)),
            );
        }

        #[test]
        fn a_window_past_the_right_edge_moves_left() {
            check(
                rect(3900.0, 100.0, 1200.0, 800.0),
                Some(rect(3280.0, 100.0, 1200.0, 800.0)),
            );
        }

        #[test]
        fn a_window_over_the_seam_goes_to_the_screen_it_overlaps_most() {
            check(
                rect(1800.0, 100.0, 1200.0, 800.0),
                Some(rect(1920.0, 100.0, 1200.0, 800.0)),
            );
        }

        #[test]
        fn a_window_below_the_bottom_edge_moves_up() {
            check(
                rect(2000.0, 900.0, 1200.0, 800.0),
                Some(rect(2000.0, 600.0, 1200.0, 800.0)),
            );
        }

        #[test]
        fn a_window_bigger_than_the_screen_shrinks_to_it() {
            check(
                rect(2000.0, 100.0, 3000.0, 1600.0),
                Some(rect(1920.0, 0.0, 2560.0, 1400.0)),
            );
        }

        #[test]
        fn a_window_right_of_every_screen_is_not_placed() {
            check(rect(5000.0, 100.0, 1200.0, 800.0), None);
        }

        #[test]
        fn a_window_above_every_screen_is_not_placed() {
            check(rect(100.0, -900.0, 1200.0, 800.0), None);
        }
    }

    #[test]
    fn keeps_a_window_on_a_screen_left_of_the_main_one() {
        let left = rect(-1920.0, 0.0, 1920.0, 1040.0);
        assert_eq!(
            place_window(rect(-1800.0, 50.0, 700.0, 500.0), &[MAIN, left]),
            Some(rect(-1800.0, 50.0, 700.0, 500.0))
        );
    }

    #[test]
    fn keeps_a_window_flush_with_the_screen_edges_invisible_borders_included() {
        // A window snapped to the left half: its invisible resize border sticks out left, right and below.
        let snapped = rect(-7.0, 0.0, 974.0, 1047.0);
        assert_eq!(place_window(snapped, &AREAS), Some(snapped));
        assert_eq!(
            place_window(rect(-9.0, 0.0, 974.0, 1040.0), &AREAS),
            Some(rect(0.0, 0.0, 974.0, 1040.0))
        );
    }

    #[test]
    fn equal_overlap_picks_the_first_work_area() {
        assert_eq!(
            place_window(rect(1620.0, 100.0, 600.0, 400.0), &AREAS),
            Some(rect(1320.0, 100.0, 600.0, 400.0))
        );
    }

    #[test]
    fn rounds_fractional_bounds() {
        let placed = place_window(rect(100.4, 50.6, 700.5, 500.2), &[MAIN]).unwrap();
        for n in [placed.x, placed.y, placed.width, placed.height] {
            assert_eq!(n.fract(), 0.0, "{n}");
        }
    }

    #[test]
    fn no_work_areas_gives_first_start_bounds() {
        assert_eq!(place_window(rect(0.0, 0.0, 700.0, 500.0), &[]), None);
    }

    // Not in the TypeScript tests: `Math.round` takes halves up, `f64::round` away from zero. A window left of the
    // main screen is where they differ.
    #[test]
    fn rounds_halves_up_as_math_round_does() {
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-0.4), 0.0);
        let left = rect(-1920.0, 0.0, 1920.0, 1040.0);
        assert_eq!(
            place_window(rect(-1800.5, 50.5, 700.5, 500.5), &[left]),
            Some(rect(-1800.0, 51.0, 701.0, 501.0))
        );
    }
}
