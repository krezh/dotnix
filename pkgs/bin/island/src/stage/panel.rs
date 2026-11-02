//! What an island can show and how it asks for the input it needs.

use crate::features;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    /// The dashboard's overview: every feature's switches and cards together.
    Dashboard,
    System,
    Feature(&'static str),
    /// Every feature pop-up with something to show, together, such as an OSD below notification
    /// cards.
    Popups,
}

/// What an island needs covers over the outputs for while something of it is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grab {
    None,
    /// Clicks outside the island, leaving the keyboard with the focused window.
    Clicks,
    Keyboard,
}

impl Panel {
    /// Feature panels' layers follow these.
    const ALL: [Panel; 3] = [Panel::System, Panel::Popups, Panel::Dashboard];

    pub fn name(self) -> &'static str {
        match self {
            Panel::Dashboard => "dashboard",
            Panel::System => "system",
            Panel::Popups => "popups",
            Panel::Feature(name) => features::panel_of(name).unwrap_or(name),
        }
    }

    pub(super) fn takes_keyboard(self) -> bool {
        matches!(self, Panel::Feature(name) if features::keyboard(name))
    }

    pub fn parse(name: &str) -> Option<Self> {
        [Panel::Dashboard, Panel::System]
            .into_iter()
            .find(|panel| panel.name() == name)
            .or_else(|| features::panel_named(name).map(Panel::Feature))
    }

    /// Island's own panels showing what the system sampler reads: System and the dashboard.
    pub fn shows_stats(self) -> bool {
        matches!(self, Panel::Dashboard | Panel::System)
    }

    pub fn layer(self) -> usize {
        let feature = |name| {
            features::NAMES
                .iter()
                .position(|feature| *feature == name)
                .unwrap_or(0)
        };
        let index = match self {
            Panel::Feature(name) => Self::ALL.len() + feature(name),
            _ => Self::ALL.iter().position(|kind| *kind == self).unwrap_or(0),
        };
        index + 1
    }

    pub fn from_layer(layer: usize) -> Option<Self> {
        let index = layer.checked_sub(1)?;
        match index.checked_sub(Self::ALL.len()) {
            None => Self::ALL.get(index).copied(),
            Some(feature) => features::NAMES
                .get(feature)
                .map(|name| Panel::Feature(name)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Collapsed,
    Open(Panel),
}

pub const LAYERS: usize = Panel::ALL.len() + features::NAMES.len() + 1;
pub const COLLAPSED_LAYER: usize = 0;
