//! How island sees notifications: the cards each island keeps, the widget, the panel, the
//! pop-ups, and `island ctl dnd|private|clear`.

use std::any::Any;

use gpui::AnyElement;

use super::{
    cards::{self, Cards},
    commands,
    runtime::Notifications,
    settings,
    state::{Event, State},
};
use crate::{
    control::{Command, Report, Row, Switch},
    features::{
        Dashboard, Feature, Feed, GlyphSpec, IslandInfo, Islands, PanelCx, WidgetCx, notice,
    },
    panel::{ActionTile, PanelView},
    theme::FLAMINGO,
};

impl Feature for Notifications {
    const NAME: &'static str = "notifications";
    const PANEL: Option<&'static str> = Some("notifications");
    const TAB: Option<(&'static str, u8)> = Some(("Notifications", 0));
    const DASHBOARD_ORDER: u8 = 50;
    const CONTROLS: &'static [Command] = commands::CONTROLS;
    const GLYPHS: &'static [GlyphSpec] = &[
        ("tab-notifications", &["bell"]),
        ("notifications-private", &["eye-off"]),
        ("notifications-clear", &["list-x"]),
        ("notifications-all", &["inbox"]),
        ("notifications-desktop", &["monitor"]),
        ("notifications-more", &["chevron-down"]),
        ("notifications-close", &["x"]),
        ("notifications-offline", &["cloud-off"]),
        ("notifications-dnd", &["bell-off"]),
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

    /// Features that posted can check theirs are still current, such as messages on a server.
    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        notice::looked();
        None
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        self.panel_view(state, p)
    }

    fn popup<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        self.popup_view(state, p)
    }

    fn dashboard(&self, state: &State, p: &PanelCx<'_>) -> Dashboard {
        Dashboard {
            switches: vec![ActionTile {
                id: "switch-dnd".into(),
                glyph: p.glyph("notifications-dnd"),
                label: "Do not disturb".into(),
                state: Some(if state.dnd { "On" } else { "Off" }.into()),
                color: FLAMINGO,
                armed: state.dnd,
                on_click: p
                    .call(|notifications: &Notifications| notifications.set_dnd(Switch::Toggle)),
            }],
            ..Dashboard::default()
        }
    }

    fn command(&self, state: &State, args: &[&str]) -> Report {
        self.command(state, args)
    }

    fn status(&self, state: &State) -> Vec<Row> {
        self.status(state)
    }
}
