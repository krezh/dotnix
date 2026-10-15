#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub id: i64,
    pub occupied: bool,
    pub urgent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Monitor {
    pub name: String,
    /// Where the monitor sits on the desktop, in logical pixels.
    pub x: i32,
    pub y: i32,
    /// Its logical width, after scaling and rotation.
    pub width: i32,
    pub focused: bool,
    pub active: Option<i64>,
    pub fullscreen: bool,
    pub workspaces: Vec<Workspace>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub monitors: Vec<Monitor>,
}

impl Snapshot {
    pub fn focused(&self) -> Option<&Monitor> {
        self.monitors.iter().find(|monitor| monitor.focused)
    }
}
