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

#[derive(Default)]
pub struct View {
    pub prompt: Option<Prompt>,
    pub shown: Shown,
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
