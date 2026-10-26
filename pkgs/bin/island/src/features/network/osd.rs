//! The pop-up when the network goes or comes back, as the volume's shows a change: the link lost or
//! back, and the internet gone or back behind a link that stays up.

use std::time::Duration;

use super::{
    glyphs,
    state::{Internet, State},
};
use crate::{
    features::osd::{DURATION, Osd},
    theme::{GREEN, OVERLAY2, RED, YELLOW},
};

/// Long enough to catch in a game, as nothing the user did brought it on.
const BAD_NEWS: Duration = Duration::from_secs(4);

fn shown(
    glyph: &'static str,
    color: u32,
    label: impl Into<String>,
    lasts: Duration,
) -> Option<Osd> {
    Some(Osd {
        glyph,
        color,
        caption: "Network",
        label: label.into(),
        level: None,
        lasts,
    })
}

/// What changed between `before` and `after`, if it is worth a pop-up. Nothing while island is
/// still learning the network as it starts.
pub fn osd(before: &State, after: &State) -> Option<Osd> {
    if !before.reachable || !after.reachable {
        return None;
    }
    let wifi_off = after.wifi.as_ref().is_some_and(|wifi| !wifi.enabled);
    match (&before.link, &after.link) {
        (Some(_), None) if wifi_off && after.wired.as_ref().is_none_or(|wired| !wired.active) => {
            shown("network-wifi-off", OVERLAY2, "Wi-Fi is off", DURATION)
        }
        (Some(_), None) => shown(glyphs::link(after), RED, "Connection lost", BAD_NEWS),
        (None, Some(link)) => {
            let name = if link.name.is_empty() {
                link.interface.clone()
            } else {
                link.name.clone()
            };
            shown(
                glyphs::link(after),
                GREEN,
                format!("Connected · {name}"),
                DURATION,
            )
        }
        (Some(_), Some(_)) if before.internet != after.internet => match after.internet {
            Internet::None | Internet::Limited => {
                shown(glyphs::link(after), YELLOW, "No internet", BAD_NEWS)
            }
            Internet::Portal => shown(glyphs::link(after), YELLOW, "Sign-in page needed", BAD_NEWS),
            Internet::Full if before.internet.missing() => {
                shown(glyphs::link(after), GREEN, "Internet is back", DURATION)
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
#[path = "osd_test.rs"]
mod tests;
