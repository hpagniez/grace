//! `panelHeight.test.ts`'s rules, test for test. Its saved-height tests are `settings::tests::panel_height`;
//! "ignores other keys" is the type: `PanelKey` has only the splitter's four.
use super::*;

#[test]
fn max_panel_leaves_main_min_to_main() {
    assert_eq!(max_panel(500.0), 380.0);
    assert_eq!(max_panel(150.0), 96.0);
}

#[test]
fn clamp_panel_keeps_within_limits() {
    assert_eq!(clamp_panel(50.0, 300.0), 96.0);
    assert_eq!(clamp_panel(400.0, 300.0), 300.0);
    assert_eq!(clamp_panel(200.6, 300.0), 201.0);
}

#[test]
fn key_height_steps_and_jumps() {
    assert_eq!(key_height(PanelKey::Up, 192.0, 400.0), 208.0);
    assert_eq!(key_height(PanelKey::Down, 192.0, 400.0), 176.0);
    assert_eq!(key_height(PanelKey::Home, 192.0, 400.0), 96.0);
    assert_eq!(key_height(PanelKey::End, 192.0, 400.0), 400.0);
}

#[test]
fn key_height_stays_at_a_limit() {
    assert_eq!(key_height(PanelKey::Down, 96.0, 400.0), 96.0);
    assert_eq!(key_height(PanelKey::Up, 400.0, 400.0), 400.0);
}
