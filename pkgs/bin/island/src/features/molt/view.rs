//! Which of molt's screens this island shows, and the one it is leaving: a tool grows out of its
//! card on Home and shrinks back into it.

use std::{cell::Cell, rc::Rc, time::Instant};

use molt_core::build::Target;

use crate::{features::Local, spring::Spring, stage::GROW_SPRING};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Home,
    Inputs,
    Watchlist,
    Search,
    Cleanup,
    /// A build of the target, and once it is built, its review.
    Build(Target),
    /// A changed package's release notes, over the target's review.
    Notes(Target),
    /// A changed file's diff, by its place in the review's files.
    Diff(Target, usize),
}

impl Screen {
    /// Home's card it grows out of, by the name the card is drawn under.
    pub fn card(self) -> Option<&'static str> {
        match self {
            Screen::Home | Screen::Notes(_) | Screen::Diff(..) => None,
            Screen::Inputs => Some("molt-home-card-Inputs"),
            Screen::Watchlist => Some("molt-home-card-Watchlist"),
            Screen::Search => Some("molt-home-card-Search"),
            Screen::Cleanup => Some("molt-home-card-Cleanup"),
            Screen::Build(Target::NixOs) => Some("molt-home-card-Target(NixOs)"),
            Screen::Build(Target::HomeManager) => Some("molt-home-card-Target(HomeManager)"),
        }
    }

    /// How far in it lies: Home, a tool on it, or something a tool opens.
    pub const fn depth(self) -> u8 {
        match self {
            Screen::Home => 0,
            Screen::Notes(_) | Screen::Diff(..) => 2,
            _ => 1,
        }
    }

    /// Molt's window size for it, narrowed to fit the island's surface.
    pub fn size(self) -> (f32, f32) {
        match self {
            Screen::Home => (900.0, 636.0),
            _ => (1192.0, 780.0),
        }
    }
}

/// Where typing goes: nowhere until a field is clicked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    #[default]
    None,
    Filter,
    Pin,
    /// The field to track another pull request.
    Track,
    /// The journal history cleanup keeps.
    Journals,
}

pub struct Passing {
    pub from: Option<Screen>,
    pub at: Spring,
    pub width: Rc<Cell<Option<f32>>>,
}

impl Default for Passing {
    fn default() -> Self {
        Self {
            from: None,
            at: Spring::new(1.0, GROW_SPRING, 0.001),
            width: Rc::default(),
        }
    }
}

#[derive(Default)]
pub struct View {
    pub screen: Screen,
    pub passing: Passing,
    pub focus: Focus,
}

impl View {
    pub fn open(&mut self, screen: Screen) {
        if screen == self.screen {
            return;
        }
        let back = self.passing.from == Some(screen);
        let at = if back {
            1.0 - self.passing.at.value()
        } else {
            0.0
        };
        self.passing.from = Some(self.screen);
        self.passing.width.set(None);
        self.screen = screen;
        self.focus = Focus::None;
        self.passing.at.reset(at);
        self.passing.at.retarget(1.0, GROW_SPRING);
    }
}

impl Local for View {
    /// Back to Home, unless on a build or its review, which the droplet opens again.
    fn panel_closed(&mut self) {
        let kept = match self.screen {
            Screen::Build(target) | Screen::Notes(target) | Screen::Diff(target, _) => {
                Screen::Build(target)
            }
            _ => Screen::Home,
        };
        *self = View::default();
        self.screen = kept;
    }

    fn panel_size(&self) -> Option<(f32, f32)> {
        Some(self.screen.size())
    }

    /// Molt is worked with the pointer: it takes the keys only while a clicked field takes typing.
    fn keyboard(&self) -> bool {
        self.focus != Focus::None
    }

    fn let_go(&mut self) {
        self.focus = Focus::None;
    }

    fn advance(&mut self, dt: f32, now: Instant) {
        self.passing.at.advance(dt, now);
        if self.passing.at.is_settled() {
            self.passing.from = None;
        }
    }

    fn is_settled(&self) -> bool {
        self.passing.at.is_settled()
    }
}
