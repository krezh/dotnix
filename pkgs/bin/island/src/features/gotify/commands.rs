use super::{
    runtime::{Gotify, Request},
    state::State,
};
use crate::control::{Command, Group, Report, Row, Shown, Takes, Value, form};

pub const CONTROLS: &[Command] = &[Command {
    name: "gotify",
    about: "Gotify messages",
    group: Group::System,
    takes: Takes::Forms {
        forms: &[
            form("refresh", Value::Nothing, "Refresh messages and reconnect"),
            form("clear", Value::Nothing, "Delete all messages on the server"),
        ],
        bare: Some("Show the Gotify status"),
    },
    examples: &["island ctl gotify refresh", "island ctl gotify clear"],
}];

pub fn status(state: &State) -> Vec<Row> {
    let Some(server) = &state.server else {
        return Vec::new();
    };
    let text = match &state.error {
        Some(error) => format!("{server} ({error})"),
        None => format!("{server} ({} messages)", state.messages),
    };
    vec![Row::new("Gotify", Shown::Text(text))]
}

impl Gotify {
    pub(super) fn handle_command(&self, state: &State, args: &[&str]) -> Report {
        match args {
            ["gotify"] => Report::shows(status(state)),
            ["gotify", "refresh"] => {
                self.request(Request::Refresh);
                Report::did(Row::said("Refreshing Gotify"))
            }
            ["gotify", "clear"] => {
                self.request(Request::Clear);
                Report::did(Row::said("Cleared Gotify messages"))
            }
            _ => Report::default(),
        }
    }
}
