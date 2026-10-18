mod detail;
mod format;
mod harness;
mod logos;
mod model;
mod notify;
mod overview;
mod panel;
mod runtime;
mod settings;
mod sources;
mod state;
mod view;
mod widget;

use std::time::Duration;

use gpui::AnyElement;

use super::{Feature, Feed, GlyphSpec, IslandInfo, PanelCx, WidgetCx};
use crate::panel::{IconButton, PanelView};
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
    ];
    const USAGE: &'static str = "  ai refresh";

    type State = State;
    type Local = View;
    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        runtime::start(feed)
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

    fn system_button(&self, state: &State, p: &PanelCx<'_>) -> Option<IconButton> {
        Some(panel::system_button(self, state, p))
    }

    fn check(args: &[&str]) -> Result<(), String> {
        runtime::check(args)
    }

    fn command(&self, _: &State, _: &[&str]) {
        self.refresh();
    }
}
