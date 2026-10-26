//! What `island` takes outside `island ctl`, written down once: its options, `island logs`,
//! `island completions` and the features' own commands, such as `island ai-hook`.

/// What an option's value is.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Number,
    Path,
    Text,
    /// One of these, and only these.
    OneOf(&'static [&'static str]),
    /// Free text, with these offered when completing.
    Suggested(fn() -> Vec<&'static str>),
}

#[derive(Debug)]
pub struct Opt {
    pub long: &'static str,
    pub short: Option<char>,
    /// The value's name in help, such as `px`, and what it is; `None` for a flag.
    pub value: Option<(&'static str, Kind)>,
    pub about: &'static str,
}

impl Opt {
    pub fn spelled(&self) -> String {
        format!("--{}", self.long)
    }
}

const fn opt(long: &'static str, value: (&'static str, Kind), about: &'static str) -> Opt {
    Opt {
        long,
        short: None,
        value: Some(value),
        about,
    }
}

pub const ISLAND: &[Opt] = &[
    opt(
        "config",
        ("path", Kind::Path),
        "Config file, by default $XDG_CONFIG_HOME/island/config.toml",
    ),
    opt(
        "radius",
        ("px", Kind::Number),
        "Corner radius, over the config's",
    ),
    opt(
        "power",
        ("p", Kind::Number),
        "Corner curve, 2 circular and 4 squircle, over the config's",
    ),
    opt(
        "opacity",
        ("a", Kind::Number),
        "Body opacity from 0 to 1, over the config's",
    ),
    opt(
        "icon-theme",
        ("name", Kind::Text),
        "Icon theme, over the config's",
    ),
];

pub const LEVELS: &[&str] = &["error", "warn", "info", "debug", "trace"];

fn modules() -> Vec<&'static str> {
    crate::features::NAMES.to_vec()
}

pub const LOGS: &[Opt] = &[
    Opt {
        long: "lines",
        short: Some('n'),
        value: Some(("count", Kind::Number)),
        about: "Recent entries to show before following, 50 unless given",
    },
    opt(
        "level",
        ("level", Kind::OneOf(LEVELS)),
        "Show this level and more severe ones, and have island log it while connected",
    ),
    opt(
        "module",
        ("name", Kind::Suggested(modules)),
        "Only this module or feature, such as audio, media or tray",
    ),
    Opt {
        long: "no-follow",
        short: None,
        value: None,
        about: "Print the recent entries and stop",
    },
];

pub const SHELLS: &[&str] = &["bash", "fish", "zsh"];

/// A feature's own command, run by another program and waiting for island's answer.
#[derive(Debug)]
pub struct Client {
    pub name: &'static str,
    pub about: &'static str,
    /// What it takes, named so in help, and the values it knows.
    pub value: &'static str,
    pub values: &'static [&'static str],
    /// A line under its help, such as what else it takes.
    pub note: Option<&'static str>,
}

#[derive(Debug)]
pub enum Takes {
    /// `island ctl`'s commands.
    Ctl,
    Options(&'static [Opt]),
    /// One value, named so in help, from these.
    One(&'static str, &'static [&'static str]),
}

#[derive(Debug)]
pub struct Sub {
    pub name: &'static str,
    pub about: &'static str,
    pub takes: Takes,
    /// A line under its help, such as where it reads from.
    pub note: Option<&'static str>,
    /// Run by another program rather than typed, such as a feature's client: help lists it apart
    /// and nothing completes it.
    pub for_programs: bool,
}

/// Every command `island` takes, its own first, then the features'.
pub fn commands() -> Vec<Sub> {
    let own = [
        Sub {
            name: "ctl",
            about: "Control the running island",
            takes: Takes::Ctl,
            note: None,
            for_programs: false,
        },
        Sub {
            name: "logs",
            about: "Follow the running island's logs",
            takes: Takes::Options(LOGS),
            note: Some("Reads the running island over its log socket, however it was started."),
            for_programs: false,
        },
        Sub {
            name: "completions",
            about: "Print shell completions for island",
            takes: Takes::One("shell", SHELLS),
            note: Some("Such as island completions fish | source"),
            for_programs: false,
        },
    ];
    own.into_iter()
        .chain(crate::features::clients().into_iter().map(|client| Sub {
            name: client.name,
            about: client.about,
            takes: Takes::One(client.value, client.values),
            note: client.note,
            for_programs: true,
        }))
        .collect()
}

pub fn find(name: &str) -> Option<Sub> {
    commands().into_iter().find(|sub| sub.name == name)
}
