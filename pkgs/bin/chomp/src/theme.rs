use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    #[default]
    Catppuccin,
    Dracula,
    Gruvbox,
    Nord,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub background: &'static str,
    pub surface: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub danger: &'static str,
    pub warning: &'static str,
    pub info: &'static str,
}

impl ThemeName {
    pub const fn palette(self) -> ThemePalette {
        match self {
            Self::Catppuccin => ThemePalette {
                background: "#1E1E2E",
                surface: "#313244",
                text: "#CDD6F4",
                muted: "#BAC2DE",
                accent: "#89B4FA",
                danger: "#F38BA8",
                warning: "#F9E2AF",
                info: "#74C7EC",
            },
            Self::Dracula => ThemePalette {
                background: "#282A36",
                surface: "#44475A",
                text: "#F8F8F2",
                muted: "#BFBFBF",
                accent: "#BD93F9",
                danger: "#FF5555",
                warning: "#F1FA8C",
                info: "#8BE9FD",
            },
            Self::Gruvbox => ThemePalette {
                background: "#282828",
                surface: "#3C3836",
                text: "#EBDBB2",
                muted: "#BDAE93",
                accent: "#D3869B",
                danger: "#FB4934",
                warning: "#FABD2F",
                info: "#83A598",
            },
            Self::Nord => ThemePalette {
                background: "#2E3440",
                surface: "#3B4252",
                text: "#ECEFF4",
                muted: "#D8DEE9",
                accent: "#B48EAD",
                danger: "#BF616A",
                warning: "#EBCB8B",
                info: "#88C0D0",
            },
        }
    }
}

pub fn resolve_color<'a>(override_color: &'a str, fallback: &'a str) -> &'a str {
    if override_color.is_empty() {
        fallback
    } else {
        override_color
    }
}
