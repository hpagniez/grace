//! Grace's look in GPUI: IBM Plex, the Lucide icons GPUI Kit bundles, and the two themes from the semantic tokens of
//! src/index.css (spec docs/specs/2026-10-29-premium-ui.md). The themes differ only in colours, as in Grace.
use std::borrow::Cow;

use gpui::{App, AssetSource, Hsla, Result, SharedString, Window, WindowAppearance, px, rgb};
use gpui_component::{Theme, ThemeMode};

/// IBM Plex, embedded as in Grace (which bundles the woff2 files; GPUI needs TrueType).
const FONTS: [&[u8]; 3] = [
    include_bytes!("../assets/fonts/IBMPlexSans-Variable.ttf"),
    include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf"),
    include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf"),
];

pub const SANS: &str = "IBM Plex Sans";
pub const MONO: &str = "IBM Plex Mono";

// Only the icons Grace's screen uses are embedded, beside the ones GPUI Kit's own components draw.
gpui_kit_assets::icon_assets!(GraceIcons, [
    Play, ChevronRight, ChevronDown, Circle, CircleDashed, LoaderCircle, CircleCheck, CircleX, CircleSlash, FolderOpen,
    Square, Settings2, Search, Tag, Plus, Close, SquareTerminal, Network, Inbox, Globe
]);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match GraceIcons.load(path)? {
            Some(b) => Ok(Some(b)),
            None => gpui_kit_assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut all = GraceIcons.list(path)?;
        all.extend(gpui_kit_assets::Assets.list(path)?);
        Ok(all)
    }
}

pub fn load_fonts(cx: &mut App) {
    cx.text_system().add_fonts(FONTS.iter().map(|f| Cow::Borrowed(*f)).collect()).expect("IBM Plex loads");
}

/// The colours GPUI Kit has no slot for: statuses and their tints.
#[derive(Clone, Copy)]
pub struct Palette {
    pub ground: Hsla,
    pub surface: Hsla,
    pub sunken: Hsla,
    pub hover: Hsla,
    pub selected: Hsla,
    pub line: Hsla,
    pub ink: Hsla,
    pub muted: Hsla,
    pub primary: Hsla,
    pub passed: Hsla,
    pub failed: Hsla,
    pub skipped: Hsla,
    pub running: Hsla,
    pub failed_tint: Hsla,
}

/// The tokens, light then dark, in the order of `Palette`'s fields.
const TOKENS: [[u32; 14]; 2] = [
    [0xf3efe6, 0xffffff, 0xf8f6f1, 0xf5f2eb, 0xece7dc, 0xe5e0d5, 0x151821, 0x5b5e66, 0x1b7a4c, 0x1b7a4c, 0xc22a2f, 0x64676f, 0x2a68b0, 0xf8e5e6],
    [0x151821, 0x101217, 0x0b0d11, 0x191c23, 0x1f232c, 0x262a33, 0xf1efe9, 0x9fa2ac, 0x3fcf6e, 0x3fcf6e, 0xf2666f, 0x9a9ea8, 0x62a8f2, 0x341f25],
];

pub fn palette(cx: &App) -> Palette {
    let t = TOKENS[Theme::global(cx).is_dark() as usize].map(|c| -> Hsla { rgb(c).into() });
    Palette {
        ground: t[0],
        surface: t[1],
        sunken: t[2],
        hover: t[3],
        selected: t[4],
        line: t[5],
        ink: t[6],
        muted: t[7],
        primary: t[8],
        passed: t[9],
        failed: t[10],
        skipped: t[11],
        running: t[12],
        failed_tint: t[13],
    }
}

/// Dark when GRACE_THEME=dark, light when GRACE_THEME=light, else as the system says.
pub fn wants_dark(window: &Window) -> bool {
    match std::env::var("GRACE_THEME").as_deref() {
        Ok("dark") => true,
        Ok("light") => false,
        _ => matches!(window.appearance(), WindowAppearance::Dark | WindowAppearance::VibrantDark),
    }
}

/// Maps Grace's tokens onto GPUI Kit's theme slots.
pub fn apply(dark: bool, window: Option<&mut Window>, cx: &mut App) {
    Theme::change(if dark { ThemeMode::Dark } else { ThemeMode::Light }, window, cx);
    let p = palette(cx);
    let on_primary: Hsla = if dark { rgb(0x151821).into() } else { rgb(0xffffff).into() };
    let primary_hover: Hsla = if dark { rgb(0x5fd98a).into() } else { rgb(0x156640).into() };
    // Through `update`, which carries the colours into the tokens GPUI Kit's components read.
    Theme::update(cx, |t| {
        t.font_family = SANS.into();
        t.mono_font_family = MONO.into();
        t.font_size = px(13.);
        t.mono_font_size = px(12.);
        t.radius = px(6.);
        t.radius_lg = px(10.);
        t.shadow = true;
        let c = &mut t.colors;
        c.background = p.surface;
        c.foreground = p.ink;
        c.muted = p.sunken;
        c.muted_foreground = p.muted;
        c.border = p.line;
        c.input = p.line;
        c.ring = if dark { p.primary } else { p.ink };
        c.caret = p.ink;
        c.selection = if dark { rgb(0x2a4a3a).into() } else { rgb(0xcfe3d8).into() };
        c.primary = p.primary;
        c.primary_hover = primary_hover;
        c.primary_active = primary_hover;
        c.primary_foreground = on_primary;
        c.button_primary = p.primary;
        c.button_primary_hover = primary_hover;
        c.button_primary_active = primary_hover;
        c.button_primary_foreground = on_primary;
        c.secondary = p.surface;
        c.secondary_hover = p.hover;
        c.secondary_active = p.selected;
        c.secondary_foreground = p.ink;
        c.button = p.surface;
        c.button_hover = p.hover;
        c.button_active = p.selected;
        c.button_foreground = p.ink;
        c.accent = p.hover;
        c.accent_foreground = p.ink;
        c.danger = p.failed;
        c.success = p.passed;
        c.popover = p.surface;
        c.popover_foreground = p.ink;
        c.list = p.surface;
        c.list_hover = p.hover;
        c.list_active = p.selected;
        c.list_active_border = p.selected;
        c.tab_bar = p.surface;
        c.tab = p.surface;
        c.tab_active = p.surface;
        c.tab_active_foreground = p.ink;
        c.tab_foreground = p.muted;
        c.scrollbar = Hsla { a: 0., ..p.surface };
        c.overlay = if dark { Hsla { h: 0., s: 0., l: 0., a: 0.6 } } else { Hsla { h: 0., s: 0., l: 0.08, a: 0.35 } };
    });
}
