//! How island sees notifications: the cards each island keeps, the widget, the panel, the
//! pop-ups, and `island ctl dnd|private|clear`.

use gpui::AnyElement;

use super::{
    cards::{self, Cards},
    commands,
    runtime::Notifications,
    settings,
    state::{Event, State},
};
use crate::{
    control::{Command, Report, Row},
    features::{Feature, Feed, GlyphSpec, IslandInfo, Islands, PanelCx, WidgetCx},
    panel::PanelView,
};

impl Feature for Notifications {
    const NAME: &'static str = "notifications";
    const PANEL: Option<&'static str> = Some("notifications");
    const TAB: Option<(&'static str, u8)> = Some(("Notifications", 0));
    const CONTROLS: &'static [Command] = commands::CONTROLS;
    const GLYPHS: &'static [GlyphSpec] = &[
        (
            "tab-notifications",
            &["notifications-symbolic", "preferences-system-notifications-symbolic"],
        ),
        (
            "notifications-now",
            &["preferences-system-notifications-symbolic", "notifications-symbolic"],
        ),
        ("notifications-earlier", &["document-open-recent-symbolic"]),
    ];

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

    fn command(&self, state: &State, args: &[&str]) -> Report {
        self.command(state, args)
    }

    fn status(&self, state: &State) -> Vec<Row> {
        self.status(state)
    }
}
