//! `island ctl volume …` and `island ctl mic …`.

use super::{
    Audio,
    request::{self, Request},
    state::{AudioState, Target},
};
use crate::control::{self, Change, Command, Group, LEVEL_AND_MUTE, Report, Row, Shown, Takes};

pub const CONTROLS: &[Command] = &[
    Command {
        name: "volume",
        about: "The default output's volume",
        group: Group::SoundAndScreen,
        takes: Takes::Forms {
            forms: LEVEL_AND_MUTE,
            bare: Some("Show the volume"),
        },
        examples: &["island ctl volume up 10", "island ctl volume set 40%"],
    },
    Command {
        name: "mic",
        about: "The default input's volume",
        group: Group::SoundAndScreen,
        takes: Takes::Forms {
            forms: LEVEL_AND_MUTE,
            bare: Some("Show the microphone's volume"),
        },
        examples: &["island ctl mic mute"],
    },
];

fn label(target: Target) -> &'static str {
    match target {
        Target::Output => "Volume",
        Target::Input => "Microphone",
    }
}

/// The default device's level, as it is or as `change` leaves it.
fn row(state: &AudioState, target: Target, change: Option<Change>) -> Row {
    let Some(device) = state.default_device(target) else {
        return Row::new(label(target), Shown::Text("no device".to_owned()));
    };
    let (value, muted) = match change {
        None => (device.volume, device.muted),
        Some(Change::Step(delta)) => (request::stepped(device.volume, delta), device.muted),
        Some(Change::Set(value)) => (value, device.muted),
        Some(Change::ToggleMute) => (device.volume, !device.muted),
    };
    Row::new(label(target), Shown::Level { value, muted }).detail(device.description.clone())
}

pub fn run(audio: &Audio, state: &AudioState, args: &[&str]) -> Report {
    let target = if args[0] == "mic" {
        Target::Input
    } else {
        Target::Output
    };
    if args.len() == 1 {
        return Report::shows(vec![row(state, target, None)]);
    }
    let Ok(change) = control::change(&args[1..], true) else {
        return Report::default();
    };
    audio.send(match change {
        Change::Step(delta) => Request::Step(target, delta),
        Change::Set(value) => Request::SetVolume(target, value),
        Change::ToggleMute => Request::ToggleMute(target),
    });
    Report::did(row(state, target, Some(change)))
}

pub fn status(state: &AudioState) -> Vec<Row> {
    vec![
        row(state, Target::Output, None),
        row(state, Target::Input, None),
    ]
}
