//! Grace's look (premium UI spec, docs/specs/2026-10-29-premium-ui.md): the semantic colour tokens of
//! `src/index.css`, light and dark, and the sizes both themes share. The two themes differ only in colours: a
//! `Theme` is one shared `Metrics` plus one `Tokens`, so a size cannot differ between them (R1, `tokens.test.ts`).
//!
//! Colours reach the views only through here: no `rgb(...)` or `hsla(...)` outside this module (R1.1, R9.3;
//! `palette.test.ts`, ported as `tests::colours_come_only_from_the_theme`).
use gpui::{App, Global, Hsla, Pixels, Rgba, Window, WindowAppearance, px, rgb, rgba};
use gpui_component::{Theme as KitTheme, ThemeMode};

/// The colour tokens of `src/index.css`, by their CSS names with `-` as `_`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub ground: u32,
    pub surface: u32,
    pub raised: u32,
    pub sunken: u32,
    pub hover: u32,
    pub selected: u32,
    pub line: u32,
    pub control: u32,
    pub ink: u32,
    pub muted: u32,
    pub accent: u32,
    pub on_accent: u32,
    pub primary: u32,
    pub on_primary: u32,
    pub primary_hover: u32,
    pub on_danger: u32,
    pub focus: u32,
    pub passed: u32,
    pub failed: u32,
    pub failed_hover: u32,
    pub skipped: u32,
    pub running: u32,
    pub cancelled: u32,
    pub passed_tint: u32,
    pub failed_tint: u32,
    pub skipped_tint: u32,
    pub running_tint: u32,
    pub cancelled_tint: u32,
    pub found: u32,
    pub found_current: u32,
    /// The floating shadow's two colours, `rgba`; its geometry is in `Metrics` (R3.5).
    pub shadow_float: [u32; 2],
    /// The dialog backdrop, `rgba`.
    pub backdrop: u32,
}

impl Tokens {
    /// Every token by its CSS name, for the tests that compare them with `src/index.css` and check contrast.
    pub fn named(&self) -> [(&'static str, u32); 30] {
        [
            ("ground", self.ground),
            ("surface", self.surface),
            ("raised", self.raised),
            ("sunken", self.sunken),
            ("hover", self.hover),
            ("selected", self.selected),
            ("line", self.line),
            ("control", self.control),
            ("ink", self.ink),
            ("muted", self.muted),
            ("accent", self.accent),
            ("on-accent", self.on_accent),
            ("primary", self.primary),
            ("on-primary", self.on_primary),
            ("primary-hover", self.primary_hover),
            ("on-danger", self.on_danger),
            ("focus", self.focus),
            ("passed", self.passed),
            ("failed", self.failed),
            ("failed-hover", self.failed_hover),
            ("skipped", self.skipped),
            ("running", self.running),
            ("cancelled", self.cancelled),
            ("passed-tint", self.passed_tint),
            ("failed-tint", self.failed_tint),
            ("skipped-tint", self.skipped_tint),
            ("running-tint", self.running_tint),
            ("cancelled-tint", self.cancelled_tint),
            ("found", self.found),
            ("found-current", self.found_current),
        ]
    }
}

pub const LIGHT: Tokens = Tokens {
    ground: 0xf3efe6,
    surface: 0xffffff,
    raised: 0xffffff,
    sunken: 0xf8f6f1,
    hover: 0xf5f2eb,
    selected: 0xece7dc,
    line: 0xe5e0d5,
    control: 0x8a8478,
    ink: 0x151821,
    muted: 0x5b5e66,
    accent: 0x151821,
    on_accent: 0xffffff,
    primary: 0x1b7a4c,
    on_primary: 0xffffff,
    primary_hover: 0x156640,
    on_danger: 0xffffff,
    focus: 0x151821,
    passed: 0x1b7a4c,
    failed: 0xc22a2f,
    failed_hover: 0xa3242a,
    skipped: 0x64676f,
    running: 0x2a68b0,
    cancelled: 0x8f5a00,
    passed_tint: 0xe4efea,
    failed_tint: 0xf8e5e6,
    skipped_tint: 0xecedee,
    running_tint: 0xe5edf6,
    cancelled_tint: 0xf2ebe0,
    found: 0xf2e3a6,
    found_current: 0xe8b84a,
    // rgb(21 24 33 / 0.14), rgb(21 24 33 / 0.06)
    shadow_float: [0x15182124, 0x1518210f],
    // the ink at 40 %
    backdrop: 0x15182166,
};

pub const DARK: Tokens = Tokens {
    ground: 0x151821,
    surface: 0x101217,
    raised: 0x1b1e26,
    sunken: 0x0b0d11,
    hover: 0x191c23,
    selected: 0x1f232c,
    line: 0x262a33,
    control: 0x6a707e,
    ink: 0xf1efe9,
    muted: 0x9fa2ac,
    accent: 0x3fcf6e,
    on_accent: 0x06210f,
    primary: 0x3fcf6e,
    on_primary: 0x151821,
    primary_hover: 0x5fd98a,
    on_danger: 0x151821,
    focus: 0x3fcf6e,
    passed: 0x3fcf6e,
    failed: 0xf2666f,
    failed_hover: 0xf5838a,
    skipped: 0x9a9ea8,
    running: 0x62a8f2,
    cancelled: 0xe5aa45,
    passed_tint: 0x183025,
    failed_tint: 0x341f25,
    skipped_tint: 0x26282e,
    running_tint: 0x1d2a3a,
    cancelled_tint: 0x322a1e,
    found: 0x3d3519,
    found_current: 0x6b5216,
    // rgb(0 0 0 / 0.5), rgb(0 0 0 / 0.3)
    shadow_float: [0x00000080, 0x0000004d],
    // rgb(0 0 0 / 0.6)
    backdrop: 0x00000099,
};

/// A text size and its line height (R2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSize {
    pub size: f32,
    pub line_height: f32,
}

/// What both themes share: type, sizes, radii, motion. One value per theme is impossible by construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub text_2xs: TextSize,
    pub text_xs: TextSize,
    pub text_sm: TextSize,
    pub text_base: TextSize,
    pub text_lg: TextSize,
    pub text_xl: TextSize,
    /// Hover, press, colour (`--duration-fast`).
    pub duration_fast_ms: u64,
    /// Chevron turn, tab line, dialog and popover enter (`--duration-base`).
    pub duration_base_ms: u64,
    /// The sheet under the toolbar (R3.3).
    pub sheet_inset: f32,
    pub sheet_radius: f32,
    pub sheet_side: f32,
    /// The focus ring (R10.1): 2 px in the focus colour, 2 px off the control.
    pub focus_ring_width: f32,
    pub focus_ring_offset: f32,
    /// `rounded-md` and `rounded-lg`, Tailwind's 6 and 8 px; a dialog is `rounded-xl`, 12 px.
    pub radius: f32,
    pub radius_lg: f32,
    pub radius_xl: f32,
    /// A button, an input: 28 px (R3.1); a small one 24 px.
    pub control_height: f32,
    pub control_height_small: f32,
    /// The floating shadow's two layers: y offset and blur, in px (R3.5).
    pub shadow_float: [(f32, f32); 2],
}

pub const METRICS: Metrics = Metrics {
    text_2xs: TextSize {
        size: 11.,
        line_height: 16.,
    },
    text_xs: TextSize {
        size: 12.,
        line_height: 16.,
    },
    text_sm: TextSize {
        size: 13.,
        line_height: 20.,
    },
    text_base: TextSize {
        size: 14.,
        line_height: 20.,
    },
    text_lg: TextSize {
        size: 16.,
        line_height: 24.,
    },
    text_xl: TextSize {
        size: 20.,
        line_height: 28.,
    },
    duration_fast_ms: 100,
    duration_base_ms: 160,
    sheet_inset: 8.,
    sheet_radius: 16.,
    sheet_side: 1.,
    focus_ring_width: 2.,
    focus_ring_offset: 2.,
    radius: 6.,
    radius_lg: 8.,
    radius_xl: 12.,
    control_height: 28.,
    control_height_small: 24.,
    shadow_float: [(12., 32.), (1., 2.)],
};

/// IBM Plex, as Grace bundles it (ADR 0021).
pub const SANS: &str = "IBM Plex Sans";
pub const MONO: &str = "IBM Plex Mono";

/// The theme of the app: dark or light, its colours, and the shared metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub dark: bool,
    pub tokens: &'static Tokens,
    pub metrics: &'static Metrics,
}

impl Global for Theme {}

impl Theme {
    pub fn new(dark: bool) -> Self {
        Self {
            dark,
            tokens: if dark { &DARK } else { &LIGHT },
            metrics: &METRICS,
        }
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// A token as a GPUI colour.
    pub fn color(c: u32) -> Hsla {
        rgb(c).into()
    }

    /// A token with alpha (`0xrrggbbaa`) as a GPUI colour.
    pub fn color_alpha(c: u32) -> Hsla {
        rgba(c).into()
    }

    pub fn px(v: f32) -> Pixels {
        px(v)
    }
}

/// Which theme a window gets: `GRACE_THEME=dark|light` pins it (for screenshots and tests), else it follows the
/// system's, as `nativeTheme` does in Electron Grace.
pub fn wants_dark(appearance: WindowAppearance, pinned: Option<&str>) -> bool {
    match pinned {
        Some("dark") => true,
        Some("light") => false,
        _ => matches!(appearance, WindowAppearance::Dark | WindowAppearance::VibrantDark),
    }
}

/// Sets the theme from the window's appearance, and keeps following it.
pub fn follow_system(window: &mut Window, cx: &mut App) {
    let pinned = std::env::var("GRACE_THEME").ok();
    apply(wants_dark(window.appearance(), pinned.as_deref()), Some(window), cx);
    window
        .observe_window_appearance(move |window, cx| {
            let pinned = std::env::var("GRACE_THEME").ok();
            apply(wants_dark(window.appearance(), pinned.as_deref()), Some(window), cx);
        })
        .detach();
}

/// Sets Grace's theme, and maps its tokens onto GPUI Kit's, so GPUI Kit's controls draw in Grace's colours.
pub fn apply(dark: bool, window: Option<&mut Window>, cx: &mut App) {
    let theme = Theme::new(dark);
    cx.set_global(theme);
    KitTheme::change(if dark { ThemeMode::Dark } else { ThemeMode::Light }, window, cx);
    let t = theme.tokens;
    let m = theme.metrics;
    let c = Theme::color;
    let selection: Hsla = Rgba {
        a: 0.35,
        ..rgb(t.primary)
    }
    .into();
    KitTheme::update(cx, |k| {
        k.font_family = SANS.into();
        k.mono_font_family = MONO.into();
        k.font_size = px(m.text_sm.size);
        k.mono_font_size = px(m.text_xs.size);
        k.radius = px(m.radius);
        k.radius_lg = px(m.radius_lg);
        k.shadow = true;
        let k = &mut k.colors;
        k.background = c(t.surface);
        k.foreground = c(t.ink);
        k.muted = c(t.sunken);
        k.muted_foreground = c(t.muted);
        k.border = c(t.line);
        k.input = c(t.control);
        k.ring = c(t.focus);
        k.caret = c(t.ink);
        k.selection = selection;
        k.primary = c(t.primary);
        k.primary_hover = c(t.primary_hover);
        k.primary_active = c(t.primary_hover);
        k.primary_foreground = c(t.on_primary);
        k.button_primary = c(t.primary);
        k.button_primary_hover = c(t.primary_hover);
        k.button_primary_active = c(t.primary_hover);
        k.button_primary_foreground = c(t.on_primary);
        k.secondary = c(t.surface);
        k.secondary_hover = c(t.hover);
        k.secondary_active = c(t.selected);
        k.secondary_foreground = c(t.ink);
        k.button = c(t.surface);
        k.button_hover = c(t.hover);
        k.button_active = c(t.selected);
        k.button_foreground = c(t.ink);
        k.accent = c(t.hover);
        k.accent_foreground = c(t.ink);
        k.danger = c(t.failed);
        k.danger_hover = c(t.failed_hover);
        k.danger_active = c(t.failed_hover);
        k.danger_foreground = c(t.on_danger);
        k.button_danger = c(t.failed);
        k.button_danger_hover = c(t.failed_hover);
        k.button_danger_active = c(t.failed_hover);
        k.button_danger_foreground = c(t.on_danger);
        k.success = c(t.passed);
        k.popover = c(t.raised);
        k.popover_foreground = c(t.ink);
        k.list = c(t.surface);
        k.list_hover = c(t.hover);
        k.list_active = c(t.selected);
        k.list_active_border = c(t.selected);
        k.tab_bar = c(t.surface);
        k.tab = c(t.surface);
        k.tab_active = c(t.surface);
        k.tab_active_foreground = c(t.ink);
        k.tab_foreground = c(t.muted);
        k.scrollbar = Hsla { a: 0., ..c(t.surface) };
        k.scrollbar_thumb = Hsla { a: 0.6, ..c(t.control) };
        k.overlay = Theme::color_alpha(t.backdrop);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn without_comments(css: &str) -> String {
        let mut out = String::new();
        let mut rest = css;
        while let Some(start) = rest.find("/*") {
            out.push_str(&rest[..start]);
            rest = rest[start..].find("*/").map_or("", |end| &rest[start + end + 2..]);
        }
        out + rest
    }

    /// `--color-<name>: #rrggbb;` declarations of a block of CSS, as name to colour.
    fn colours(css: &str) -> BTreeMap<String, u32> {
        css.split(';')
            .filter_map(|decl| {
                let (name, value) = decl.trim().split_once(':')?;
                let name = name.trim().strip_prefix("--color-")?;
                let hex = value.trim().strip_prefix('#')?;
                Some((name.to_string(), u32::from_str_radix(hex, 16).ok()?))
            })
            .collect()
    }

    /// The light tokens (the `@theme` block) and the dark ones (the one `prefers-color-scheme: dark` block) of
    /// Electron Grace's `src/index.css`. Until the cut-over both apps must draw the same colours (guide R9).
    fn index_css() -> (BTreeMap<String, u32>, BTreeMap<String, u32>, String) {
        let css = without_comments(&std::fs::read_to_string(repo().join("src/index.css")).unwrap());
        let (before, dark) = css.split_once("prefers-color-scheme: dark").unwrap();
        let theme = &before[before.find("@theme").unwrap()..];
        let dark = &dark[..dark.find("html {").unwrap()];
        (colours(theme), colours(dark), css)
    }

    #[test]
    fn matches_the_light_tokens_of_index_css() {
        let (light, _, _) = index_css();
        for (name, value) in LIGHT.named() {
            assert_eq!(light.get(name), Some(&value), "--color-{name}");
        }
    }

    #[test]
    fn matches_the_dark_tokens_of_index_css() {
        let (_, dark, _) = index_css();
        for (name, value) in DARK.named() {
            assert_eq!(dark.get(name), Some(&value), "--color-{name}");
        }
        // and the dark block holds no token Grace lacks
        let names: Vec<_> = DARK.named().iter().map(|(n, _)| n.to_string()).collect();
        assert_eq!(dark.keys().cloned().collect::<Vec<_>>().len(), names.len());
    }

    #[test]
    fn matches_the_sizes_of_index_css() {
        let (_, _, css) = index_css();
        let m = METRICS;
        for (name, t) in [
            ("2xs", m.text_2xs),
            ("xs", m.text_xs),
            ("sm", m.text_sm),
            ("base", m.text_base),
            ("lg", m.text_lg),
            ("xl", m.text_xl),
        ] {
            assert!(css.contains(&format!("--text-{name}: {}px;", t.size)), "--text-{name}");
            assert!(
                css.contains(&format!("--text-{name}--line-height: {}px;", t.line_height)),
                "--text-{name} line"
            );
        }
        assert!(css.contains(&format!("--duration-fast: {}ms;", m.duration_fast_ms)));
        assert!(css.contains(&format!("--duration-base: {}ms;", m.duration_base_ms)));
        assert!(css.contains(&format!("--sheet-inset: {}px;", m.sheet_inset)));
        assert!(css.contains(&format!("--sheet-radius: {}px;", m.sheet_radius)));
        assert!(css.contains(&format!("--sheet-side: {}px;", m.sheet_side)));
        assert!(css.contains(&format!(
            "outline: {}px solid var(--color-focus);\n    outline-offset: {}px;",
            m.focus_ring_width, m.focus_ring_offset
        )));
        for (y, blur) in m.shadow_float {
            assert!(
                css.contains(&format!("0 {y}px {blur}px rgb")),
                "shadow layer {y} {blur}"
            );
        }
    }

    /// WCAG 2 relative luminance.
    fn luminance(c: u32) -> f64 {
        let ch = |shift: u32| {
            let v = ((c >> shift) & 0xff) as f64 / 255.;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(16) + 0.7152 * ch(8) + 0.0722 * ch(0)
    }

    /// WCAG 2 contrast ratio.
    fn contrast(a: u32, b: u32) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    fn token(t: &Tokens, name: &str) -> u32 {
        t.named().iter().find(|(n, _)| *n == name).unwrap().1
    }

    const STATUSES: [&str; 5] = ["passed", "failed", "skipped", "running", "cancelled"];

    fn each_theme(check: impl Fn(&str, &Tokens)) {
        check("light", &LIGHT);
        check("dark", &DARK);
    }

    #[test]
    fn keeps_text_at_4_5_to_1_on_every_background() {
        each_theme(|theme, t| {
            for bg in ["ground", "surface", "sunken", "hover", "selected", "raised"] {
                for fg in ["ink", "muted"] {
                    let r = contrast(token(t, fg), token(t, bg));
                    assert!(r >= 4.5, "{theme}: {fg} on {bg} is {r:.2}");
                }
            }
        });
    }

    #[test]
    fn keeps_each_status_readable_on_surface_its_tint_and_a_selected_row() {
        each_theme(|theme, t| {
            for s in STATUSES {
                let c = token(t, s);
                assert!(contrast(c, t.surface) >= 4.5, "{theme}: {s} on surface");
                assert!(
                    contrast(c, token(t, &format!("{s}-tint"))) >= 4.5,
                    "{theme}: {s} on its tint"
                );
                assert!(contrast(c, t.selected) >= 3., "{theme}: {s} on selected");
            }
        });
    }

    #[test]
    fn fills_the_primary_button_with_the_passed_green() {
        each_theme(|theme, t| assert_eq!(t.primary, t.passed, "{theme}"));
    }

    #[test]
    fn keeps_the_filled_buttons_text_readable() {
        each_theme(|theme, t| {
            for (fg, bg) in [
                (t.on_accent, t.accent),
                (t.on_primary, t.primary),
                (t.on_danger, t.failed),
                (t.on_primary, t.primary_hover),
                (t.on_danger, t.failed_hover),
            ] {
                assert!(contrast(fg, bg) >= 4.5, "{theme}: {fg:06x} on {bg:06x}");
            }
        });
    }

    #[test]
    fn paints_console_matches_readably_the_current_one_stronger() {
        each_theme(|theme, t| {
            assert!(contrast(t.ink, t.found) >= 4.5, "{theme}: ink on found");
            assert!(contrast(t.ink, t.found_current) >= 4.5, "{theme}: ink on found-current");
            assert!(
                contrast(t.found_current, t.sunken) > contrast(t.found, t.sunken),
                "{theme}"
            );
        });
    }

    #[test]
    fn keeps_control_borders_and_the_focus_ring_at_3_to_1() {
        each_theme(|theme, t| {
            for bg in [t.surface, t.ground] {
                for fg in [t.control, t.focus] {
                    assert!(contrast(fg, bg) >= 3., "{theme}: {fg:06x} on {bg:06x}");
                }
            }
        });
    }

    #[test]
    fn picks_the_theme_from_the_system_unless_pinned() {
        assert!(wants_dark(WindowAppearance::Dark, None));
        assert!(!wants_dark(WindowAppearance::Light, None));
        assert!(wants_dark(WindowAppearance::Light, Some("dark")));
        assert!(!wants_dark(WindowAppearance::Dark, Some("light")));
        assert!(wants_dark(WindowAppearance::VibrantDark, Some("other")));
    }

    /// `palette.test.ts`: colours come from the tokens. In Rust, a colour literal is `rgb(`, `rgba(`, `hsla(` or
    /// a `Hsla { .. }`; only this file may build one from a token.
    #[test]
    fn colours_come_only_from_the_theme() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        let mut files = 0;
        for entry in walk(&src) {
            if entry.file_name().is_some_and(|n| n == "theme.rs") {
                continue;
            }
            files += 1;
            let text = std::fs::read_to_string(&entry).unwrap();
            for (i, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap();
                if ["rgb(", "rgba(", "hsla(", "Hsla {", "black()", "white()"]
                    .iter()
                    .any(|p| code.contains(p))
                {
                    offenders.push(format!("{}:{}: {}", entry.display(), i + 1, line.trim()));
                }
            }
        }
        assert!(files > 3, "scans the app's files");
        assert_eq!(offenders, Vec::<String>::new());
    }

    fn walk(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
        out
    }
}
