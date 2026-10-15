use zbus::blocking::{Connection, Proxy};

pub const SERVICE: &str = "io.github.krezh.Island";
pub const PATH: &str = "/io/github/krezh/Island";
pub const INTERFACE: &str = "io.github.krezh.Island.Control";

use crate::features;

const USAGE: &str = "\
usage: island ctl COMMAND

  open PANEL | toggle PANEL | close     PANEL: calendar, system, audio, media, notifications, launcher
  brightness up|down|set PERCENT
  log SPEC                             change what is logged until restart, e.g. debug or audio=debug";

/// The help for `island ctl`, with every feature's commands.
pub fn usage() -> String {
    format!("{USAGE}{}", features::usage())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    Step(f32),
    Set(f32),
    ToggleMute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    On,
    Off,
    Toggle,
}

impl Switch {
    pub fn apply(self, current: bool) -> bool {
        match self {
            Switch::On => true,
            Switch::Off => false,
            Switch::Toggle => !current,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Control {
    Open(String),
    Toggle(String),
    Close,
    Brightness(Change),
    Log(String),
    /// A command for a feature, by its name.
    Feature(&'static str, Vec<String>),
}

const STEP: f32 = 0.05;

pub fn change(args: &[&str], mute: bool) -> Result<Change, String> {
    match args {
        ["up"] => Ok(Change::Step(STEP)),
        ["down"] => Ok(Change::Step(-STEP)),
        ["up", amount] => percent(amount).map(Change::Step),
        ["down", amount] => percent(amount).map(|value| Change::Step(-value)),
        ["mute"] if mute => Ok(Change::ToggleMute),
        ["set", amount] => percent(amount).map(Change::Set),
        _ => Err("expected up, down, set PERCENT or mute".to_owned()),
    }
}

fn percent(text: &str) -> Result<f32, String> {
    text.trim_end_matches('%')
        .parse::<f32>()
        .map(|value| (value / 100.0).clamp(0.0, 1.0))
        .map_err(|_| format!("'{text}' is not a percentage"))
}

pub fn switch(args: &[&str]) -> Result<Switch, String> {
    match args {
        ["on"] => Ok(Switch::On),
        ["off"] => Ok(Switch::Off),
        [] | ["toggle"] => Ok(Switch::Toggle),
        _ => Err("expected on, off or toggle".to_owned()),
    }
}

pub fn parse(args: &[String]) -> Result<Control, String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["open", panel] => Ok(Control::Open((*panel).to_owned())),
        ["toggle", panel] => Ok(Control::Toggle((*panel).to_owned())),
        ["close"] => Ok(Control::Close),
        ["brightness", rest @ ..] => change(rest, false).map(Control::Brightness),
        ["log", spec] => Ok(Control::Log((*spec).to_owned())),
        all => match features::check_command(all) {
            Some(checked) => checked.map(|feature| Control::Feature(feature, all.iter().map(|arg| (*arg).to_owned()).collect())),
            None => Err("unknown command".to_owned()),
        },
    }
}

pub fn send(args: Vec<String>) -> Result<(), String> {
    parse(&args)?;
    let connection = Connection::session().map_err(|error| format!("no session bus: {error}"))?;
    let proxy = Proxy::new(&connection, SERVICE, PATH, INTERFACE).map_err(|error| error.to_string())?;
    let reply: String = proxy
        .call("Run", &(args,))
        .map_err(|error| format!("island is not running: {error}"))?;
    if reply.is_empty() { Ok(()) } else { Err(reply) }
}
