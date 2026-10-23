//! Which glyph stands for the network as it is.

use super::state::{Activity, State};

/// The Wi-Fi glyph for a signal strength from 0 to 100.
pub fn signal(strength: u8) -> &'static str {
    match strength {
        80.. => "network-wifi-excellent",
        60.. => "network-wifi-good",
        40.. => "network-wifi-ok",
        20.. => "network-wifi-weak",
        _ => "network-wifi-none",
    }
}

/// What the bar shows for the whole network: the Wi-Fi signal, the cable, or that there is
/// nothing.
pub fn link(state: &State) -> &'static str {
    if let Some(wifi) = &state.wifi {
        if state.joining.as_ref().is_some_and(|joining| joining.failed.is_none()) {
            return "network-wifi-acquiring";
        }
        if !wifi.enabled {
            return "network-wifi-off";
        }
        if let Some(active) = wifi.active() {
            return if state.internet.missing() {
                "network-wifi-no-route"
            } else {
                signal(active.strength)
            };
        }
    }
    match &state.wired {
        Some(wired) if wired.active && state.internet.missing() => "network-wired-no-route",
        Some(wired) if wired.active => "network-wired",
        Some(_) => "network-wired-off",
        None if state.wifi.is_some() => "network-offline",
        None => "network-wired-off",
    }
}

pub fn vpn_on(state: &State) -> bool {
    state.vpns.iter().any(|vpn| vpn.activity == Activity::On)
}

#[cfg(test)]
#[path = "glyphs_test.rs"]
mod tests;
