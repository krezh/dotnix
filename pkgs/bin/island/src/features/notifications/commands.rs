use super::{Event, Notifications, State};
use crate::control::{self, Command, Group, Report, Row, SWITCH, Shown, Switch, Takes};

const SHOW: &str = "Show whether it is on";

pub(super) const CONTROLS: &[Command] = &[
    Command {
        name: "dnd",
        about: "Do not disturb",
        group: Group::Notifications,
        takes: Takes::Forms {
            forms: SWITCH,
            bare: Some(SHOW),
        },
        examples: &["island ctl dnd on"],
    },
    Command {
        name: "private",
        about: "Hide what notifications say",
        group: Group::Notifications,
        takes: Takes::Forms {
            forms: SWITCH,
            bare: Some(SHOW),
        },
        examples: &["island ctl private toggle"],
    },
    Command {
        name: "clear",
        about: "Dismiss every notification",
        group: Group::Notifications,
        takes: Takes::Nothing,
        examples: &[],
    },
];

const DND: &str = "Do not disturb";
const PRIVATE: &str = "Private";

fn rows(state: &State) -> Vec<Row> {
    vec![
        Row::new(DND, Shown::Switch(state.dnd)),
        Row::new(PRIVATE, Shown::Switch(state.private)),
        Row::new("Missed", Shown::Count(state.missed.len() as u64)),
    ]
}

impl Notifications {
    pub(super) fn set_dnd(&self, switch: Switch) {
        self.change(|store| store.dnd = switch.apply(store.dnd));
    }

    pub(super) fn set_private(&self, switch: Switch) {
        self.change(|store| store.private = switch.apply(store.private));
    }

    pub(super) fn command(&self, state: &State, args: &[&str]) -> Report {
        match args {
            ["clear"] => {
                self.feed.send(Event::Clear);
                Report::did(Row::said("Dismissed every notification"))
            }
            [_] => Report::shows(rows(state)),
            [name, ..] => {
                let Ok(switch) = control::switch(&args[1..]) else {
                    return Report::default();
                };
                let (label, now) = if *name == "dnd" {
                    self.set_dnd(switch);
                    (DND, switch.apply(state.dnd))
                } else {
                    self.set_private(switch);
                    (PRIVATE, switch.apply(state.private))
                };
                Report::did(Row::new(label, Shown::Switch(now)))
            }
            [] => Report::default(),
        }
    }

    pub(super) fn status(&self, state: &State) -> Vec<Row> {
        let on = |on| if on { "on" } else { "off" };
        vec![Row::new(
            "Notifications",
            Shown::Text(format!(
                "{} missed · do not disturb {} · private {}",
                state.missed.len(),
                on(state.dnd),
                on(state.private)
            )),
        )]
    }
}
