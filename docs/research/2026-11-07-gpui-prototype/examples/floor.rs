//! The floor of GPUI itself: one 1400 x 900 window with one line of text, nothing else (#152 asks for each stack's floor).
use gpui::{App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size};

struct Floor;

impl Render for Floor {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(rgb(0xffffff)).child("Grace")
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::new(gpui::point(px(0.), px(0.)), size(px(1400.), px(900.)));
        cx.open_window(
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            |_, cx| cx.new(|_| Floor),
        )
        .unwrap();
    });
}
