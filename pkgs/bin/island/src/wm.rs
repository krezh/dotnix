#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub id: i64,
    pub occupied: bool,
    pub urgent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Monitor {
    pub name: String,
    /// Logical pixels.
    pub x: i32,
    pub y: i32,
    /// Logical, after scaling and rotation.
    pub width: i32,
    pub height: i32,
    pub focused: bool,
    pub active: Option<i64>,
    pub fullscreen: bool,
    pub workspaces: Vec<Workspace>,
}

/// Where a window is: a numbered workspace, or a special one by its name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    Workspace(i64),
    Special(String),
}

impl Place {
    pub fn label(&self) -> String {
        match self {
            Place::Workspace(id) => format!("Workspace {id}"),
            Place::Special(name) if name == "special" => "Special workspace".to_owned(),
            Place::Special(name) => format!("Special workspace · {name}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    /// The compositor's own name for it, such as `0x55d0`.
    pub address: String,
    /// What it keeps for its whole life, as other protocols know it.
    pub id: String,
    pub class: String,
    pub title: String,
    pub place: Place,
    pub monitor: String,
    /// Logical pixels from the monitor's top left.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub floating: bool,
    pub urgent: bool,
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
