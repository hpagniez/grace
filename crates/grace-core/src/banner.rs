//! `bannerText.ts`: what the console's banner says while the console has no output (F24 spec R2.1). The finding
//! line's words are `grace::progress::finding_text`, shared with the progress line.
use crate::model::node::{NodeKind, TestNode};
use crate::model::tags::matching_leaves;
use crate::rows::leaves;
use crate::state::AppState;

/// What the console's state line says, while it has no output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerState {
    None,
    Finding,
    Starting,
    Ready,
}

fn features(n: &TestNode) -> usize {
    usize::from(n.kind == NodeKind::Feature) + n.children.iter().map(features).sum::<usize>()
}

fn count(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

/// The banner's tests line, None for no line. Counts the whole tree, not the tag filter.
pub fn tests_text(s: &AppState) -> Option<String> {
    s.project.as_ref()?;
    if s.error.is_some() {
        return Some("could not be listed: see the message in the tree".into());
    }
    let Some(tree) = &s.tree else {
        return s.busy.then(|| "finding tests…".into());
    };
    let n = leaves(tree).len();
    if n == 0 {
        return Some("No feature files found".into());
    }
    Some(format!(
        "{} in {}",
        count(n, "scenario"),
        count(features(tree), "feature")
    ))
}

/// None once the console has output: the lines then follow the banner. None too when Run all would be disabled
/// (the toolbar's rule): "Ready" would then point at a button that does nothing.
pub fn banner_state(s: &AppState) -> Option<BannerState> {
    if !s.console.is_empty() {
        return None;
    }
    if s.run.active {
        return Some(BannerState::Starting);
    }
    if s.busy {
        return Some(BannerState::Finding);
    }
    if s.project.is_none() {
        return Some(BannerState::None);
    }
    let ready = s.error.is_none()
        && s.tree
            .as_ref()
            .is_some_and(|t| !matching_leaves(t, &s.filter).is_empty());
    ready.then_some(BannerState::Ready)
}

#[cfg(test)]
mod tests;
