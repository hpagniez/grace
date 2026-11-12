---
paths:
  - "src/index.css"
  - "src/renderer/**"
  - "crates/grace/src/theme.rs"
  - "crates/grace/assets/**"
---

# Rules that bind both apps until the cut-over

Electron Grace ships until the GPUI cut-over, and GPUI Grace must look the same: change both sides together.

- Colours only through `grace::theme` (`Theme::global(cx).tokens`, `Theme::color`): no `rgb(`, `rgba(`, `hsla(` or `Hsla {` elsewhere; `theme::tests::colours_come_only_from_the_theme` enforces it. Until the cut-over the tokens must equal `src/index.css`'s, which the theme's tests check: change both together.
- Icons only through `grace::assets::Icon`, the same Lucide files as Electron Grace's `lucide` version; a new icon in `src/renderer` fails `has_the_icons_of_electron_grace` until it is added to `crates/grace/assets/icons/`. GPUI Kit's own `IconName` holds only the icons its controls use.
