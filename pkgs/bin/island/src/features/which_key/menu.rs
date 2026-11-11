//! Finding where in its menus the panel is: the menu named, then the keys pressed inside it.

use super::settings::{Entry, Settings};

/// The menu shown when `island ctl open which-key` names none.
pub(super) const DEFAULT: &str = "default";

/// One level of a menu: the titles leading to it, its own last, and its entries.
pub(super) struct Level<'a> {
    pub(super) path: Vec<String>,
    pub(super) entries: &'a [Entry],
}

/// The level `keys` lead to in the menu `name`, or `None` without such a menu.
pub(super) fn level<'a>(settings: &'a Settings, name: &str, keys: &[String]) -> Option<Level<'a>> {
    let mut entries = settings.menus.get(name)?.as_slice();
    let mut path = vec![title(name)];
    for key in keys {
        let entry = entries
            .iter()
            .find(|entry| &entry.key == key && !entry.submenu.is_empty())?;
        path.push(entry.desc.clone());
        entries = &entry.submenu;
    }
    Some(Level { path, entries })
}

/// A menu's name as its title: `browser` reads `Browser`.
pub(super) fn title(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Columns for this many entries: side by side up to three, then on to further rows.
pub(super) fn columns(count: usize) -> usize {
    count.clamp(1, 3)
}

#[cfg(test)]
#[path = "menu_test.rs"]
mod tests;
