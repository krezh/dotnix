//! What `island ctl` takes, written down once: help, checking and completions all come from it.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Panels,
    Notifications,
    SoundAndScreen,
    System,
}

impl Group {
    pub const ALL: [Group; 4] = [
        Group::Panels,
        Group::Notifications,
        Group::SoundAndScreen,
        Group::System,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Group::Panels => "Panels",
            Group::Notifications => "Notifications",
            Group::SoundAndScreen => "Sound and screen",
            Group::System => "System",
        }
    }
}

/// What may follow a form's word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Nothing,
    Percent,
    /// A percentage that may be left out.
    MaybePercent,
}

/// One word a command takes, such as `up` in `volume up`.
#[derive(Debug)]
pub struct Form {
    pub word: &'static str,
    pub value: Value,
    pub about: &'static str,
}

#[derive(Debug)]
pub enum Takes {
    Nothing,
    /// One of these words; `bare` says what the command does without one, or `None` when it
    /// needs one.
    Forms {
        forms: &'static [Form],
        bare: Option<&'static str>,
    },
    /// As `Forms`, maybe after a word naming which of several it acts on, such as a monitor;
    /// named `at` in help. Without that word it acts on all of them.
    Aimed {
        at: &'static str,
        forms: &'static [Form],
        bare: Option<&'static str>,
    },
    /// A panel and, for one with views, maybe one of them.
    Panel,
    /// A word of the caller's own, named so in help.
    Word(&'static str),
}

#[derive(Debug)]
pub struct Command {
    pub name: &'static str,
    pub about: &'static str,
    pub group: Group,
    pub takes: Takes,
    pub examples: &'static [&'static str],
}

pub const fn form(word: &'static str, value: Value, about: &'static str) -> Form {
    Form { word, value, about }
}

pub const SWITCH: &[Form] = &[
    form("on", Value::Nothing, "Turn it on"),
    form("off", Value::Nothing, "Turn it off"),
    form(
        "toggle",
        Value::Nothing,
        "Turn it on if it is off, and off if it is on",
    ),
];

pub const LEVEL: &[Form] = &[
    form("up", Value::MaybePercent, "Raise it, by 5% unless given"),
    form("down", Value::MaybePercent, "Lower it, by 5% unless given"),
    form("set", Value::Percent, "Set it, such as 40 or 40%"),
];

pub const LEVEL_AND_MUTE: &[Form] = &[
    form("up", Value::MaybePercent, "Raise it, by 5% unless given"),
    form("down", Value::MaybePercent, "Lower it, by 5% unless given"),
    form("set", Value::Percent, "Set it, such as 40 or 40%"),
    form("mute", Value::Nothing, "Mute or unmute"),
];

/// The commands island itself answers, rather than one of its features.
pub const CORE: &[Command] = &[
    Command {
        name: "open",
        about: "Open a panel",
        group: Group::Panels,
        takes: Takes::Panel,
        examples: &["island ctl open audio", "island ctl open ai chat"],
    },
    Command {
        name: "toggle",
        about: "Open a panel, or close it if it is open",
        group: Group::Panels,
        takes: Takes::Panel,
        examples: &["island ctl toggle launcher"],
    },
    Command {
        name: "close",
        about: "Close whatever is open",
        group: Group::Panels,
        takes: Takes::Nothing,
        examples: &[],
    },
    Command {
        name: "log",
        about: "Change what is logged until island restarts",
        group: Group::System,
        takes: Takes::Word("spec"),
        examples: &[
            "island ctl log debug",
            "island ctl log audio=debug,tray=trace",
        ],
    },
];

/// Every command, island's own first, each with the feature that answers it, or `None` for
/// island's own.
pub fn all() -> Vec<(Option<&'static str>, &'static Command)> {
    CORE.iter()
        .map(|command| (None, command))
        .chain(
            crate::features::controls()
                .into_iter()
                .map(|(feature, command)| (Some(feature), command)),
        )
        .collect()
}

pub fn find(name: &str) -> Option<(Option<&'static str>, &'static Command)> {
    all().into_iter().find(|(_, command)| command.name == name)
}

/// Every panel `open` takes, with the views it can be opened on.
pub fn panels() -> Vec<(&'static str, &'static [&'static str])> {
    [("dashboard", &[][..]), ("system", &[][..])]
        .into_iter()
        .chain(
            crate::features::panels()
                .into_iter()
                .map(|panel| (panel, crate::features::views(panel))),
        )
        .collect()
}
