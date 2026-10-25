//! How island sees the AI feature: usage and Claude Code sessions, each island's view with Jelly,
//! the widget, the panel and pop-ups, `island ctl ai`, and the `island ai-hook` client.

use std::any::Any;

use gpui::{AnyElement, Keystroke};

use super::{
    hook, keys, panel,
    runtime::{self, Ai},
    session::Event,
    settings,
    state::State,
    view::View,
    widget,
};
use crate::{
    cli::Client,
    control::{Command, Group, Report, Row, Takes, Value, form},
    features::{Feature, Feed, GlyphSpec, IslandInfo, Islands, Outcome, PanelCx, WidgetCx},
    panel::{IconButton, Mark, PanelView},
};

impl Feature for Ai {
    const NAME: &'static str = "ai";
    const PANEL: Option<&'static str> = Some("ai");
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
        (
            "ai-sessions",
            &["utilities-terminal-symbolic", "view-list-symbolic"],
        ),
        (
            "ai-chat",
            &[
                "view-conversation-balloon-symbolic",
                "chat-message-new-symbolic",
            ],
        ),
        (
            "ai-usage",
            &["speedometer-symbolic", "utilities-system-monitor-symbolic"],
        ),
    ];
    const CONTROLS: &'static [Command] = &[Command {
        name: "ai",
        about: "Fetch AI usage now",
        group: Group::System,
        takes: Takes::Forms {
            forms: &[form("refresh", Value::Nothing, "Fetch AI usage now")],
            bare: None,
        },
        examples: &[],
    }];
    const VIEWS: &'static [&'static str] = &["sessions", "chat", "usage"];
    const CLIENT: Option<Client> = Some(Client {
        name: "ai-hook",
        about: "Hand island a Claude Code hook event; Claude Code's hooks run it",
        value: "event",
        values: hook::EVENTS,
        note: Some(
            "After --, a command such as a status line gets the same input. island ai-hook plugin <dir> writes the Claude Code plugin that runs these hooks.",
        ),
    });
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

    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        self.opened();
        None
    }

    fn elapse(&self, view: &mut View, _hovered: bool) {
        view.settle();
    }

    fn pointer(&self, view: &mut View, over: bool) {
        if over != view.held.is_some() {
            view.held = over.then(|| self.hold(view.number.0));
        }
    }

    fn lead(&self, view: &View) -> Option<Mark> {
        view.pose().map(Mark::Mascot)
    }

    fn system_button(&self, state: &State, p: &PanelCx<'_>) -> Option<IconButton> {
        Some(panel::system_button(self, state, p))
    }

    fn command(&self, _: &State, _: &[&str]) -> Report {
        self.refresh();
        Report::did(Row::said("Fetching AI usage"))
    }

    fn client(args: &[String]) -> i32 {
        hook::client::run(args)
    }

    fn key(&self, state: &State, view: &mut View, keystroke: &Keystroke) -> Outcome {
        keys::key(self, state, view, keystroke)
    }
}
