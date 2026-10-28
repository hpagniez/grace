---
status: accepted
date: 2026-10-27
decision-makers: Tusquito
---

# Rename the app from karate-kid to Grace

## Context and Problem Statement

The app is called karate-kid. THE KARATE KID is a registered US trademark of Columbia Pictures Industries (Reg. 6102180), a famous mark the name points straight at. Karate Labs' website terms (§8.1) claim "Karate" as their trademark, and a product name containing it reads as an official Karate Labs tool. Saying the app runs Karate tests is fine; naming it after either is not. The repo also has no LICENSE file, so the code is not open source yet. Research of 2026-10-27; not legal advice.

Releases 1.0.0 and 1.1.0 keep settings, the projects list, run history and saved secrets in `%APPDATA%\karate-kid`, a folder Electron names after `productName`.

## Considered Options

1. Grace, after Grace Hopper, whose team taped the first computer "bug" (a moth) into a logbook in 1947.
2. A martial-arts word without "Karate": Bunkai, Makiwara, Kiai.
3. A plain testing word: Greenbar, Runthrough, Rehearsal.
4. Keep karate-kid.

## Decision Outcome

Chosen: option 1. The maintainer wanted a first name like Claude or Bruno, with an easter egg. Option 2 was turned down, option 3 is not a name, option 4 keeps the legal risk. No testing tool was found under the name Grace (GitHub and web search, 2026-10-27; trademark registers not searched).

- `package.json`: `name` `grace`, `productName` `Grace`. The app becomes `Grace.exe` in `Grace-win32-x64-<version>.zip`, with user data in `%APPDATA%\Grace`.
- Nothing moves the old `%APPDATA%\karate-kid` folder: the app is still in development, so it keeps no backward compatibility (AGENTS.md). Grace starts with empty settings, projects list, run history and secrets.
- `KARATE_KID_USER_DATA` becomes `GRACE_USER_DATA`; the `karate-kid.panelHeight` localStorage key becomes `grace.panelHeight`.
- Window title, startup screen, `README.md`, `AGENTS.md` and `docs/features.md` say Grace. Dated specs, plans, checks and research notes keep the old name: they record what was true then.
- Add `LICENSE` (MIT, matching `package.json`).
- `README.md` says the app is not affiliated with or endorsed by Karate Labs Inc., and that Karate is their trademark. It lists the bundled Java runtime (GPLv2 with the Classpath Exception) and Karate jar (MIT), and says where the runtime's source is: the repository and commit in `resources/jre/release` (`SOURCE_REPO`, `SOURCE`), which every Temurin build ships.
- The maintainer renames the GitHub repository and the local folder; GitHub redirects the old URLs.

### Icon and mascot

Built on its own branch after the rename; the rename ships with Electron's default icon.

The story: Grace, a made-up martial artist, fights bugs. The bug is a moth, after the one Hopper's team taped into the Harvard Mark II logbook on 9 September 1947. Grace is not a likeness of Hopper, who was a Navy officer, not a fighter.

- **App icon:** a moth cut in two by one green diagonal strike, on a dark rounded square. Green is the colour of a passing test. The moth is off-white, like logbook paper. Two shapes only, so it reads at 16 px in the taskbar.
- **Sizes:** one SVG source, exported to a Windows `.ico` with 16, 24, 32, 48, 64, 128 and 256 px. The 16 and 32 px images are drawn by hand: no antennae, no wing spots, a thicker strike. Forge takes it through `packagerConfig.icon`, the window through `BrowserWindow`'s `icon`.
- **Mascot:** Grace mid-strike, side-on, plain white gi, hair tied back, a green belt that matches the strike. The moth is large and cartoonish, with a strip of tape on one wing. She appears on the home screen when no project is open and at the top of the README. Flat vector, thick outlines, 3 colours plus neutrals, legible on the light and dark themes.
- **Avoid:** the Karate Kid crane-kick pose, a headband with a rising sun, bonsai, anything close to Karate Labs' logo, Japanese text or kanji, and a sexualised or stereotyped character.
- **Who draws it:** the icon is simple enough to draw as SVG in the repo. The mascot can start as a simple SVG and later be replaced by an illustrator's work. Commissioned art needs a licence that allows redistribution in an MIT project (for example CC BY 4.0), recorded next to the files in `assets/`.
- Not planned: a mascot that changes with the run result, or any animation.

### Consequences

- Good: no film or Karate Labs mark in the product name.
- Bad: users of 1.0.0 and 1.1.0 lose their settings, projects list, run history and secrets. The old folder stays on disk until they delete it.
- Bad: "Grace" is a common word, so it is weak as a trademark and harder to search for.
- Neutral: links in old records to `github.com/Tusquito/karate-kid` rely on GitHub's redirect.
