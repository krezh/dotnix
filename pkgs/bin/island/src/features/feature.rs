//! What a feature is: the [`Feature`] trait, with its widget, panel and commands, and each
//! island's own part of it, [`Local`].

use std::{
    any::Any,
    time::{Duration, Instant},
};

use gpui::{AnyElement, Keystroke, Window};

use super::{Battery, Feed, Islands, Osd, PanelCx, WidgetCx};
use crate::{
    cli::Client,
    control::{Command, Reply, Report, Row},
    panel::{ActionTile, Block, Mark, PanelView, Warning},
    wm::Monitor,
};

/// Whether a panel that takes the keyboard stays open after a key.
pub enum Outcome {
    Open,
    Close,
}

/// An icon's key and the theme names to try, in order.
pub type GlyphSpec = (&'static str, &'static [&'static str]);

pub trait Feature: Sized + 'static {
    const NAME: &'static str;
    const GLYPHS: &'static [GlyphSpec] = &[];

    type State: Clone + Default + PartialEq + Send + 'static;

    /// Each island's own part, such as icons animating in and out.
    type Local: Local + Default;

    /// Sent to the islands rather than to the shared state, such as a new notification.
    type Event: Send + 'static;

    fn start(feed: Feed<Self::State, Self::Event>) -> Self;

    fn event(&self, _event: Self::Event, _islands: &mut Islands<'_, Self::Local>) {}

    fn check_settings(_settings: &toml::Value) -> Result<(), String> {
        Err(format!("[features.{}] takes no settings", Self::NAME))
    }

    /// `animate` is false before the island is first laid out, so things appear in place.
    fn sync_local(
        _state: &Self::State,
        _local: &mut Self::Local,
        _island: &IslandInfo<'_>,
        _animate: bool,
    ) {
    }

    fn initial() -> Self::State {
        Self::State::default()
    }

    /// Empty means one widget, named after the feature.
    const WIDGETS: &'static [&'static str] = &[];

    /// False folds the widget away.
    fn present(_widget: &str, _: &Self::State) -> bool {
        true
    }

    fn rest_width(_widget: &str, _spacing: f32) -> f32 {
        0.0
    }

    fn widget(&self, _widget: &str, _: &Self::State, _: &mut WidgetCx<'_>) -> Option<AnyElement> {
        None
    }

    /// Live levels arrive many times a second, so they redraw only these widgets and the glow, and
    /// never re-fit the bar.
    const LIVE: &'static [&'static str] = &[];

    fn glow<'a>(_: &'a Self::State, _live: &'a [f32]) -> Option<&'a [f32]> {
        None
    }

    /// A feature with a panel runs even when the layout leaves its widget out.
    const PANEL: Option<&'static str> = None;

    /// Title and place among the tabs; island's own Dashboard tab is 4.
    const TAB: Option<(&'static str, u8)> = None;

    /// Title and place among the Dashboard tab's views, for a panel that opens from it rather than
    /// from a tab of its own; the overview is 0 and island's own System view is 2.
    const DASHBOARD_TAB: Option<(&'static str, u8)> = None;

    /// Where what it puts on the dashboard's overview sits among every feature's, lowest first.
    const DASHBOARD_ORDER: u8 = u8::MAX;

    /// What it puts on the dashboard's overview.
    fn dashboard(&self, _: &Self::State, _: &PanelCx<'_>) -> Dashboard {
        Dashboard::default()
    }

    /// Batteries it knows of, which its bar gauge can draw and the overview gathers with every
    /// other feature's, warning of any running low.
    fn batteries(&self, _: &Self::State) -> Vec<Battery> {
        Vec::new()
    }

    /// For content that moves without new state, such as a play position.
    const PANEL_REFRESH: Option<Duration> = None;

    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        None
    }

    fn panel<'a>(&self, _: &Self::State, _: &PanelCx<'a>) -> Option<PanelView> {
        None
    }

    /// Takes the bar's place while [`Local::popup`] says so; the pointer leaving does not close it.
    fn popup<'a>(&self, _: &Self::State, _: &PanelCx<'a>) -> Option<PanelView> {
        None
    }

    /// Called when [`Local::due`] comes.
    fn elapse(&self, _local: &mut Self::Local, _hovered: bool) {}

    /// The pointer came over this island or left it, as [`Local::pointer`] hears, for a feature
    /// whose shared state cares.
    fn pointer(&self, _local: &mut Self::Local, _over: bool) {}

    /// A mark it lends to another feature's rows, asked for by its name, such as what leads a
    /// notification it sent.
    fn lead(&self, _local: &Self::Local) -> Option<Mark> {
        None
    }

    fn osd(_before: &Self::State, _after: &Self::State) -> Option<Osd> {
        None
    }

    fn inhibits_idle(_: &Self::State) -> bool {
        false
    }

    /// The `island ctl` commands it answers. `island ctl` checks them against these before they
    /// reach [`Feature::command`].
    const CONTROLS: &'static [Command] = &[];

    /// Runs one of its `CONTROLS`, and answers with what it did or, given nothing after the
    /// command's name, what the state is.
    fn command(&self, _: &Self::State, _args: &[&str]) -> Report {
        Report::default()
    }

    /// [`Feature::command`], for a feature whose answer comes only once slow work is done.
    fn reply(&self, state: &Self::State, args: &[&str]) -> Reply {
        self.command(state, args).into()
    }

    /// Its rows in `island ctl`'s overview of what island is doing.
    fn status(&self, _: &Self::State) -> Vec<Row> {
        Vec::new()
    }

    /// Views its panel can be opened on, such as its tabs: `island ctl open PANEL VIEW`.
    const VIEWS: &'static [&'static str] = &[];

    /// A top-level command of its own, such as `island ai-hook`, for a caller that waits for the
    /// running island's answer.
    const CLIENT: Option<Client> = None;

    /// Runs in the calling process with the words after the client's name; the process exits
    /// with what it returns.
    fn client(_args: &[String]) -> i32 {
        0
    }

    const KEYBOARD: bool = false;

    fn key(&self, _: &Self::State, _local: &mut Self::Local, _keystroke: &Keystroke) -> Outcome {
        Outcome::Close
    }
}

/// A feature's part of the dashboard's overview.
#[derive(Default)]
pub struct Dashboard {
    /// Action tiles with their states, among every feature's under what needs a look.
    pub switches: Vec<ActionTile>,
    /// Each a [`Block::Card`], in the overview's balanced columns.
    pub cards: Vec<Block>,
    /// What needs a look now, at the overview's top.
    pub warnings: Vec<Warning>,
}

pub struct IslandInfo<'a> {
    pub monitor: Option<&'a Monitor>,
    pub panel_open: bool,
}

pub trait Local: 'static {
    fn advance(&mut self, _dt: f32, _now: Instant) {}

    /// While not settled, the island keeps drawing frames.
    fn is_settled(&self) -> bool {
        true
    }

    /// Unsettled, but only by slow motion, which the island draws at `marquee::TICK` rather than
    /// every frame the display has.
    fn is_drifting(&self) -> bool {
        false
    }

    /// How much wider the widget becomes once it settles, so the island can fit its final size
    /// ahead of the animation.
    fn pending_width(&self, _spacing: f32) -> f32 {
        0.0
    }

    fn panel_closed(&mut self) {}

    fn popup(&self) -> bool {
        false
    }

    /// While its panel shows, whether it wants the keys; a feature that takes the keyboard can let
    /// go of it on a view with nothing to type into.
    fn keyboard(&self) -> bool {
        true
    }

    /// Turns its panel to `view`, one of the feature's `VIEWS`, as it is opened on it.
    fn show(&mut self, _view: &str) {}

    fn popup_shown(&mut self, _shown: bool) {}

    fn pointer(&mut self, _over: bool) {}

    fn due(&self) -> Option<Instant> {
        None
    }

    fn osd(&mut self, _osd: Osd) {}

    /// Measures the text it lays out by, before drawing; true when that changed its layout.
    fn measure(&mut self, _window: &Window) -> bool {
        false
    }
}

impl Local for () {}

pub trait AnyLocal: Local {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Local> AnyLocal for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(super) fn widgets_of<F: Feature>() -> impl Iterator<Item = &'static str> {
    F::WIDGETS
        .iter()
        .copied()
        .chain(F::WIDGETS.is_empty().then_some(F::NAME))
}
