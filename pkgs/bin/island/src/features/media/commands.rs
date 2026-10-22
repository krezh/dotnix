use super::{
    Media,
    mpris::{Control, Track},
};
use crate::control::{Command, Group, Report, Row, Shown, Takes, Value, form};

pub(super) const CONTROLS: &[Command] = &[Command {
    name: "media",
    about: "Control what is playing",
    group: Group::SoundAndScreen,
    takes: Takes::Forms {
        forms: &[
            form("play-pause", Value::Nothing, "Play, or pause what is playing"),
            form("next", Value::Nothing, "Skip to the next track"),
            form("previous", Value::Nothing, "Go back to the previous track"),
        ],
        bare: Some("Show what is playing"),
    },
    examples: &["island ctl media play-pause"],
}];

fn control_of(word: &str) -> Option<Control> {
    match word {
        "play-pause" => Some(Control::PlayPause),
        "next" => Some(Control::Next),
        "previous" => Some(Control::Previous),
        _ => None,
    }
}

/// `Title · Artist`, or the player's name when it gives no title.
fn named(track: &Track) -> String {
    match (track.title.is_empty(), track.artist.is_empty()) {
        (true, _) => track.identity.clone(),
        (false, true) => track.title.clone(),
        (false, false) => format!("{} · {}", track.title, track.artist),
    }
}

pub(super) fn row(track: &Option<Track>) -> Row {
    match track {
        Some(track) => Row::new(
            "Media",
            Shown::Text(format!(
                "{} {}",
                if track.playing { "▶" } else { "⏸" },
                named(track)
            )),
        )
        .detail(track.identity.clone()),
        None => Row::new("Media", Shown::Text("nothing playing".to_owned())),
    }
}

pub(super) fn run(media: &Media, track: &Option<Track>, args: &[&str]) -> Report {
    let Some(control) = args.get(1).and_then(|word| control_of(word)) else {
        return Report::shows(vec![row(track)]);
    };
    let Some(track) = track else {
        return Report::did(Row::said("Nothing is playing to control"));
    };
    let said = match control {
        Control::PlayPause if track.playing => format!("Paused {}", named(track)),
        Control::PlayPause => format!("Playing {}", named(track)),
        Control::Next => format!("Skipped {}", named(track)),
        Control::Previous => "Went back a track".to_owned(),
        Control::Seek(_) => format!("Moved through {}", named(track)),
    };
    media.send(control);
    Report::did(Row::said(said))
}
