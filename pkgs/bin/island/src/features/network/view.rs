//! Each island's own part of the network feature: the password being typed for a network, and the
//! pop-up when the network goes or comes back.

use std::time::Instant;

use crate::features::{
    Local,
    osd::{Osd, Shown},
};

use super::{Network, runtime::Request, state::State};

pub struct Prompt {
    pub ssid: String,
    pub text: String,
    /// The password shows as typed rather than as dots.
    pub shown: bool,
}

/// Which row of a list is open: one at a time, and until one is picked, the one that matters most.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Open {
    #[default]
    Natural,
    Row(String),
    None,
}

impl Open {
    /// Whether `row` is open, `natural` being the one open until another is picked.
    pub fn is(&self, row: &str, natural: Option<&str>) -> bool {
        match self {
            Open::Natural => natural == Some(row),
            Open::Row(open) => open == row,
            Open::None => false,
        }
    }

    /// Opens `row`, or closes it when it already is.
    pub fn toggle(&mut self, row: &str, natural: Option<&str>) {
        *self = if self.is(row, natural) {
            Open::None
        } else {
            Open::Row(row.to_owned())
        };
    }
}

#[derive(Default)]
pub struct View {
    pub prompt: Option<Prompt>,
    pub shown: Shown,
    pub interface: Open,
    pub network: Open,
    /// The Activity card's view: apps, ports or routes.
    pub activity: usize,
    /// The ⋯ menu of the network in use is open.
    pub menu: bool,
}

impl View {
    pub fn ask(&mut self, ssid: &str) {
        self.prompt = Some(Prompt {
            ssid: ssid.to_owned(),
            text: String::new(),
            shown: false,
        });
    }

    /// Joins the network with what was typed; the card stays up until it is joined.
    pub fn submit(&self, network: &Network) {
        if let Some(prompt) = &self.prompt
            && !prompt.text.is_empty()
        {
            network.send(Request::Join(prompt.ssid.clone(), prompt.text.clone()));
        }
    }

    /// Puts the card away once its network is joined.
    pub fn sync(&mut self, state: &State) {
        let joined = self.prompt.as_ref().is_some_and(|prompt| {
            state.joining.is_none()
                && state
                    .wifi
                    .as_ref()
                    .and_then(|wifi| wifi.active())
                    .is_some_and(|active| active.ssid == prompt.ssid)
        });
        if joined {
            self.prompt = None;
        }
    }
}

impl Local for View {
    fn panel_closed(&mut self) {
        self.prompt = None;
        self.menu = false;
        self.interface = Open::Natural;
        self.network = Open::Natural;
    }

    fn keyboard(&self) -> bool {
        self.prompt.is_some()
    }

    fn popup(&self) -> bool {
        self.shown.popup()
    }

    fn due(&self) -> Option<Instant> {
        self.shown.due()
    }

    fn pointer(&mut self, over: bool) {
        self.shown.pointer(over);
    }

    fn osd(&mut self, osd: Osd) {
        self.shown.osd(osd);
    }
}
