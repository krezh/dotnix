use super::{Media, mpris::Control};

fn control_of(word: &str) -> Option<Control> {
    match word {
        "play-pause" => Some(Control::PlayPause),
        "next" => Some(Control::Next),
        "previous" => Some(Control::Previous),
        _ => None,
    }
}

pub(super) fn check(args: &[&str]) -> Result<(), String> {
    match args {
        [_, word] if control_of(word).is_some() => Ok(()),
        _ => Err("expected play-pause, next or previous".to_owned()),
    }
}

pub(super) fn run(media: &Media, args: &[&str]) {
    if let Some(control) = args.get(1).and_then(|word| control_of(word)) {
        media.send(control);
    }
}
