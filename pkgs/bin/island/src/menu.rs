//! The island's own menus, such as a tray item's: they hang below the bar instead of morphing the
//! island into a panel. Only the island opens them; panels and features never use their rows.

pub const MAX_W: f32 = 280.0;
pub const FILLET: f32 = 10.0;

#[derive(Clone, Debug)]
pub struct MenuNode {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub separator: bool,
    pub toggle: Option<bool>,
    pub children: Vec<MenuNode>,
}
