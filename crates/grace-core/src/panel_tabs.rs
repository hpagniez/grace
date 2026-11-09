//! `panelTabs.ts`: the order of the bottom panel's Console, History and HTTP tabs (spec reorder-panel-tabs R1, R4).
//! Electron Grace kept it in the renderer's `localStorage`; GPUI Grace keeps it as `panelTabs` in the settings file
//! (`settings::{load_tab_order, save_tab_order}`), the same JSON list.

/// A tab that moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabId {
    Console,
    History,
    Http,
}

pub const TAB_IDS: [TabId; 3] = [TabId::Console, TabId::History, TabId::Http];

impl TabId {
    pub fn name(self) -> &'static str {
        match self {
            TabId::Console => "console",
            TabId::History => "history",
            TabId::Http => "http",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        TAB_IDS.into_iter().find(|t| t.name() == name)
    }
}

/// The order with `id` moved to place `to`, held between the first and the last; None if it stays.
pub fn move_tab(order: &[TabId; 3], id: TabId, to: isize) -> Option<[TabId; 3]> {
    let from = order.iter().position(|t| *t == id)?;
    let place = to.clamp(0, order.len() as isize - 1) as usize;
    if place == from {
        return None;
    }
    let mut next: Vec<TabId> = order.iter().copied().filter(|t| *t != id).collect();
    next.insert(place, id);
    Some([next[0], next[1], next[2]])
}

/// The order a saved list gives: the default unless it is exactly the three tabs once each.
pub fn parse_tab_order(saved: Option<&serde_json::Value>) -> [TabId; 3] {
    let names = saved.and_then(|v| v.as_array()).filter(|a| a.len() == TAB_IDS.len());
    let tabs: Option<Vec<TabId>> = names.and_then(|a| a.iter().map(|v| TabId::from_name(v.as_str()?)).collect());
    match tabs {
        Some(t) if TAB_IDS.iter().all(|id| t.contains(id)) => [t[0], t[1], t[2]],
        _ => TAB_IDS,
    }
}

#[cfg(test)]
mod tests;
