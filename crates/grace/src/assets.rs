//! What Grace embeds in its exe: IBM Plex (ADR 0021) and the Lucide icons it draws (ADR 0022). Nothing is fetched
//! from the network. GPUI Kit's own controls draw their icons from `gpui_kit_assets::Assets`, which `Assets` also
//! serves.
use std::borrow::Cow;

use gpui::{App, AssetSource, Result, SharedString};

/// IBM Plex Sans (variable) and Mono 400 and 500, the faces Grace uses. TrueType: GPUI does not read woff2.
const FONTS: [&[u8]; 3] = [
    include_bytes!("../assets/fonts/IBMPlexSans-Variable.ttf"),
    include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf"),
    include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf"),
];

pub fn load_fonts(cx: &mut App) {
    cx.text_system()
        .add_fonts(FONTS.iter().map(|f| Cow::Borrowed(*f)).collect())
        .expect("IBM Plex loads");
}

macro_rules! icons {
    ($($variant:ident => $name:literal,)*) => {
        /// Every Lucide icon Grace draws, from `lucide-static` at the version of Electron Grace's `lucide`
        /// (1.64.0), so both apps draw the same shapes. Their files are `assets/icons/<name>.svg`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Icon {
            $($variant,)*
        }

        impl Icon {
            pub const ALL: &[Icon] = &[$(Icon::$variant,)*];

            /// Lucide's name of the icon, its file's stem.
            pub fn name(self) -> &'static str {
                match self {
                    $(Icon::$variant => $name,)*
                }
            }

            fn bytes(self) -> &'static [u8] {
                match self {
                    $(Icon::$variant => include_bytes!(concat!("../assets/icons/", $name, ".svg")),)*
                }
            }
        }
    };
}

icons! {
    ArrowDown => "arrow-down",
    ArrowDownToDot => "arrow-down-to-dot",
    ArrowLeft => "arrow-left",
    ArrowUp => "arrow-up",
    Braces => "braces",
    Bug => "bug",
    Check => "check",
    ChevronDown => "chevron-down",
    ChevronRight => "chevron-right",
    ChevronUp => "chevron-up",
    Circle => "circle",
    CircleAlert => "circle-alert",
    CircleArrowUp => "circle-arrow-up",
    CircleCheck => "circle-check",
    CircleDashed => "circle-dashed",
    CircleMinus => "circle-minus",
    CircleSlash => "circle-slash",
    CircleX => "circle-x",
    Copy => "copy",
    Download => "download",
    ExternalLink => "external-link",
    Eye => "eye",
    EyeOff => "eye-off",
    FileSearch => "file-search",
    FileText => "file-text",
    FileUp => "file-up",
    Folder => "folder",
    FolderOpen => "folder-open",
    Globe => "globe",
    History => "history",
    Info => "info",
    LoaderCircle => "loader-circle",
    Lock => "lock",
    Minus => "minus",
    MousePointerClick => "mouse-pointer-click",
    Pause => "pause",
    Play => "play",
    Plus => "plus",
    RedoDot => "redo-dot",
    RefreshCw => "refresh-cw",
    RotateCcw => "rotate-ccw",
    Search => "search",
    Settings => "settings",
    Shuffle => "shuffle",
    SlidersHorizontal => "sliders-horizontal",
    Square => "square",
    SquareTerminal => "square-terminal",
    Trash2 => "trash-2",
    TriangleAlert => "triangle-alert",
    X => "x",
}

impl Icon {
    /// The asset path GPUI's `svg()` loads.
    pub fn path(self) -> SharedString {
        format!("grace/icons/{}.svg", self.name()).into()
    }
}

/// So a GPUI Kit control (a button, an input's prefix) can draw Grace's icons.
impl gpui_component::IconNamed for Icon {
    fn path(self) -> SharedString {
        Icon::path(self)
    }
}

/// Grace's icons under `grace/icons/`, the mascot, then GPUI Kit's own.
pub struct Assets;

/// The home screen's mascot (ADR 0013), Electron Grace's own file.
const MASCOT: &[u8] = include_bytes!("../../../assets/mascot.svg");

/// The toolbar's Home button: the app's small logo, Electron Grace's own file.
const LOGO: &[u8] = include_bytes!("../../../assets/icon-small.svg");
pub const LOGO_PATH: &str = "grace/logo.svg";

impl Assets {
    pub fn load_mascot(&self) -> &'static [u8] {
        MASCOT
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == crate::home::MASCOT {
            return Ok(Some(Cow::Borrowed(MASCOT)));
        }
        if path == LOGO_PATH {
            return Ok(Some(Cow::Borrowed(LOGO)));
        }
        if let Some(name) = path.strip_prefix("grace/icons/").and_then(|p| p.strip_suffix(".svg")) {
            return Ok(Icon::ALL
                .iter()
                .find(|i| i.name() == name)
                .map(|i| Cow::Borrowed(i.bytes())));
        }
        gpui_kit_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut all: Vec<SharedString> = Icon::ALL
            .iter()
            .map(|i| i.path())
            .filter(|p| p.starts_with(path))
            .collect();
        all.extend(gpui_kit_assets::Assets.list(path)?);
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    #[test]
    fn serves_every_icon() {
        for icon in Icon::ALL {
            let bytes = Assets.load(&icon.path()).unwrap().unwrap_or_else(|| panic!("{icon:?}"));
            assert!(bytes.starts_with(b"<svg"), "{icon:?} is an svg");
        }
        assert!(Assets.load("grace/icons/nothing.svg").unwrap().is_none());
    }

    #[test]
    fn embeds_ibm_plex() {
        for font in FONTS {
            assert!(font.len() > 50_000);
        }
    }

    /// PU-07, E-look-02: the families the theme asks for, `SANS` for the window and `MONO` for the console and the
    /// details' code, are the ones embedded, so GPUI draws Grace's own faces and falls back to none it would fetch.
    /// A font file names its family in UTF-16 for Windows and in plain bytes for the Mac.
    #[test]
    fn the_theme_asks_for_the_families_it_embeds() {
        let names = |font: &[u8], family: &str| {
            let utf16: Vec<u8> = family.encode_utf16().flat_map(u16::to_be_bytes).collect();
            let has = |needle: &[u8]| font.windows(needle.len()).any(|w| w == needle);
            has(&utf16) || has(family.as_bytes())
        };
        assert!(names(FONTS[0], crate::theme::SANS));
        assert!(names(FONTS[1], crate::theme::MONO));
        assert!(names(FONTS[2], crate::theme::MONO));
        assert!(!names(FONTS[0], crate::theme::MONO));
    }

    /// Electron Grace's icons are the `lucide` imports under `src/renderer`: until the cut-over GPUI Grace has
    /// exactly those (guide R9), so a new one there fails here.
    #[test]
    fn has_the_icons_of_electron_grace() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/renderer");
        let mut electron = BTreeSet::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            let mut rest = text.as_str();
            while let Some(start) = rest.find("import {") {
                let block = &rest[start + 8..];
                let end = block.find('}').unwrap();
                if block[end..].starts_with("} from \"lucide\"") {
                    for item in block[..end].split(',').map(str::trim) {
                        if !item.is_empty() && !item.starts_with("type ") {
                            electron.insert(item.split(" as ").next().unwrap().to_string());
                        }
                    }
                }
                rest = &block[end..];
            }
        }
        let ours: BTreeSet<String> = Icon::ALL.iter().map(|i| format!("{i:?}")).collect();
        assert_eq!(ours, electron);
    }
}
