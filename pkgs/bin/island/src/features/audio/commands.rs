//! `island ctl volume …` and `island ctl mic …`.

use super::{
    Audio,
    request::Request,
    state::Target,
};
use crate::control::{self, Change};

pub const COMMANDS: &[&str] = &["volume", "mic"];

pub const USAGE: &str = "  volume up|down|mute|set PERCENT       default output
  mic up|down|mute|set PERCENT          default input";

pub fn check(args: &[&str]) -> Result<(), String> {
    control::change(&args[1..], true).map(drop)
}

pub fn run(audio: &Audio, args: &[&str]) {
    let Ok(change) = control::change(&args[1..], true) else {
        return;
    };
    let target = if args[0] == "mic" { Target::Input } else { Target::Output };
    audio.send(match change {
        Change::Step(delta) => Request::Step(target, delta),
        Change::Set(value) => Request::SetVolume(target, value),
        Change::ToggleMute => Request::ToggleMute(target),
    });
}
