//! `panelHeight.ts`: the bottom panel's height rules (spec UI polish R1), in px. Electron Grace kept the height in
//! the renderer's `localStorage`; GPUI Grace has none, so it is `panelHeight` in the settings file
//! (`settings::{load_panel_height, save_panel_height}`).

pub const PANEL_DEFAULT: f32 = 192.0;
pub const PANEL_MIN: f32 = 96.0;
/// The least the tree and details area keeps.
pub const MAIN_MIN: f32 = 120.0;
pub const PANEL_STEP: f32 = 16.0;

/// A key the splitter uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelKey {
    Up,
    Down,
    Home,
    End,
}

/// The largest panel height when the tree/details area and the panel share `room` px; never below the minimum.
pub fn max_panel(room: f32) -> f32 {
    PANEL_MIN.max(room - MAIN_MIN)
}

pub fn clamp_panel(h: f32, max: f32) -> f32 {
    h.round().max(PANEL_MIN).min(max)
}

/// The height a key on the splitter asks for.
pub fn key_height(key: PanelKey, shown: f32, max: f32) -> f32 {
    let next = match key {
        PanelKey::Up => shown + PANEL_STEP,
        PanelKey::Down => shown - PANEL_STEP,
        PanelKey::Home => PANEL_MIN,
        PanelKey::End => max,
    };
    clamp_panel(next, max)
}

#[cfg(test)]
mod tests;
