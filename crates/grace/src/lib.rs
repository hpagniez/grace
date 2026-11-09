//! GPUI Grace (ADR 0031). Phase P0 (#279) builds the foundation: the theme, the embedded assets, the shared
//! elements, accessibility and an empty window. The views come in phases P2 to P4 (migration guide, Phases).
pub mod a11y;
pub mod assets;
#[cfg(feature = "e2e")]
pub mod e2e;
pub mod elements;
pub mod theme;
pub mod window;

use gpui::App;

/// What every window of the app needs: GPUI Kit, IBM Plex and the theme.
pub fn init(cx: &mut App) {
    gpui_component::init(cx);
    assets::load_fonts(cx);
    theme::apply(false, None, cx);
}
