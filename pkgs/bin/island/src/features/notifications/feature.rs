//! How island sees notifications: the cards each island keeps, the widget, the panel, the
//! pop-ups, and `island ctl dnd|private|clear`.

use std::time::Duration;

use gpui::AnyElement;

use super::{
    cards::{self, Cards},
    commands, panel,
    runtime::Notifications,
    settings,
    state::{Event, State},
};
use crate::{
    features::{Feature, Feed, IslandInfo, Islands, PanelCx, WidgetCx},
    panel::PanelView,
};

impl Feature for Notifications {
    const NAME: &'static str = "notifications";
    const PANEL: Option<&'static str> = Some("notifications");
    const PANEL_REFRESH: Option<Duration> = Some(panel::REFRESH);
    const COMMANDS: &'static [&'static str] = &["dnd", "private", "clear"];
    const USAGE: &'static str = "  dnd on|off|toggle
  private on|off|toggle                 hide notification contents
  clear                                 dismiss every notification";

    type State = State;
    type Local = Cards;
    type Event = Event;

    fn start(feed: Feed<State, Event>) -> Self {
        Self::start(feed)
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        settings::check(settings)
    }

    fn sync_local(state: &State, cards: &mut Cards, island: &IslandInfo<'_>, animate: bool) {
        cards::sync(state, cards, island, animate);
    }

    fn event(&self, event: Event, islands: &mut Islands<'_, Cards>) {
        self.handle_event(event, islands);
    }

    fn elapse(&self, cards: &mut Cards, hovered: bool) {
        self.elapse_cards(cards, hovered);
    }

    fn widget(&self, _: &str, state: &State, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        self.widget_view(state, w)
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        self.panel_view(state, p)
    }

    fn popup<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        self.popup_view(state, p)
    }

    fn check(args: &[&str]) -> Result<(), String> {
        commands::check(args)
    }

    fn command(&self, _: &State, args: &[&str]) {
        self.command(args);
    }
}
