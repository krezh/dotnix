mod asking;
mod cache;
mod chat;
mod conversation;
pub(crate) mod converse;
mod forecast;
mod format;
mod harness;
mod history;
mod hook;
mod keys;
mod logos;
mod model;
mod notify;
mod overview;
mod panel;
mod requests;
mod runtime;
mod session;
mod sessions;
mod settings;
mod sources;
mod state;
mod view;
mod widget;

use std::time::Duration;

use gpui::{AnyElement, Keystroke};

use super::{Feature, Feed, GlyphSpec, IslandInfo, Islands, Outcome, PanelCx, WidgetCx};
use crate::panel::{IconButton, PanelView};
use session::Event;
use state::State;
use view::View;

pub use runtime::Ai;

impl Feature for Ai {
    const NAME: &'static str = "ai";
    const PANEL: Option<&'static str> = Some("ai");
    const PANEL_REFRESH: Option<Duration> = Some(Duration::from_secs(5));
    const GLYPHS: &'static [GlyphSpec] = &[
        (
            "ai",
            &[
                "applications-science-symbolic",
                "utilities-system-monitor-symbolic",
            ],
        ),
        ("ai-claude", &["claude-symbolic", "claude"]),
        ("ai-openai", &["openai-symbolic", "openai", "chatgpt"]),
        ("ai-antigravity", &["antigravity-symbolic", "antigravity"]),
        ("ai-google", &["google-gemini-symbolic", "google-gemini"]),
        (
            "ai-github",
            &["github-copilot-symbolic", "github-copilot", "copilot"],
        ),
        ("ai-send", &["go-up-symbolic", "mail-send-symbolic"]),
    ];
    const USAGE: &'static str = "  ai refresh";
    const VIEWS: &'static [&'static str] = &["sessions", "chat", "usage"];
    const CLIENT: &'static str = "ai-hook";
    const KEYBOARD: bool = true;

    type State = State;
    type Local = View;
    type Event = Event;

    fn start(feed: Feed<State, Event>) -> Self {
        runtime::start(feed)
    }

    fn event(&self, event: Event, islands: &mut Islands<'_, View>) {
        match event {
            Event::Requests(ids) => islands.update_all(|view| view.set_pending(ids.clone())),
        }
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        settings::check(settings)
    }

    fn sync_local(state: &State, view: &mut View, island: &IslandInfo<'_>, _animate: bool) {
        view.sync(state, island.panel_open);
    }

    fn widget(&self, _: &str, state: &State, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        Some(widget::widget(self, state, w))
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        panel::panel(self, state, p)
    }

    fn popup(&self, state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
        panel::popup(state, p)
    }

    fn panel_opened(&self) {
        self.opened();
    }

    fn elapse(&self, view: &mut View, _hovered: bool) {
        view.settle();
    }

    fn system_button(&self, state: &State, p: &PanelCx<'_>) -> Option<IconButton> {
        Some(panel::system_button(self, state, p))
    }

    fn check(args: &[&str]) -> Result<(), String> {
        runtime::check(args)
    }

    fn command(&self, _: &State, _: &[&str]) {
        self.refresh();
    }

    fn client(args: &[String]) -> i32 {
        hook::client::run(args)
    }

    fn key(&self, state: &State, view: &mut View, keystroke: &Keystroke) -> Outcome {
        keys::key(self, state, view, keystroke)
    }
}
