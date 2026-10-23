//! The bar widget: the Wi-Fi signal with its strength beside it, or the cable, and the VPN shield
//! while one is on.

use gpui::{AnyElement, prelude::*};

use super::{
    Network, glyphs,
    state::{Internet, State},
};
use crate::{
    bar::{GLYPH, Gauge, ICON_GAP},
    components::{icon, tooltip::Tip},
    features::{Feature, WidgetCx},
    island::Panel,
    theme::{GREEN, MAUVE, OVERLAY2, PEACH, TEXT, YELLOW},
};

pub fn strength_color(strength: u8) -> u32 {
    match strength {
        60.. => GREEN,
        40.. => YELLOW,
        _ => PEACH,
    }
}

fn tip(state: &State) -> String {
    let joining = state
        .joining
        .as_ref()
        .filter(|joining| joining.failed.is_none());
    let link = match (&state.wifi, joining) {
        (_, Some(joining)) => format!("Joining {}", joining.ssid),
        (Some(wifi), None) if !wifi.enabled => "Wi-Fi is off".to_owned(),
        (Some(wifi), None) => match wifi.active() {
            Some(active) => format!("{} · {}%", active.ssid, active.strength),
            None => "Not connected".to_owned(),
        },
        (None, None) => match &state.wired {
            Some(wired) if wired.active => wired.name.clone(),
            _ => "No connection".to_owned(),
        },
    };
    let mut lines = vec![link];
    match state.internet {
        Internet::None | Internet::Limited if state.link.is_some() => {
            lines.push("no internet".to_owned());
        }
        Internet::Portal if state.link.is_some() => lines.push("sign-in page needed".to_owned()),
        _ => {}
    }
    if let Some(ipv4) = state.link.as_ref().and_then(|link| link.ipv4.clone()) {
        lines.push(ipv4);
    }
    if let Some(vpn) = state
        .vpns
        .iter()
        .find(|vpn| vpn.activity == super::state::Activity::On)
    {
        lines.push(format!("VPN {}", vpn.name));
    }
    lines.join(" · ")
}

pub fn widget(state: &State, w: &mut WidgetCx<'_>) -> AnyElement {
    let glyph = w.glyph(glyphs::link(state));
    let active = state
        .wifi
        .as_ref()
        .filter(|wifi| wifi.enabled)
        .and_then(|wifi| wifi.active());
    let joining = state
        .joining
        .as_ref()
        .is_some_and(|joining| joining.failed.is_none());
    let connected = active.is_some() || state.wired.as_ref().is_some_and(|wired| wired.active);
    let color = if connected && state.internet.missing() {
        YELLOW
    } else if connected || joining {
        TEXT
    } else {
        OVERLAY2
    };
    let base = match active {
        Some(active) => w.gauge(
            "bar-network",
            Gauge {
                glyph,
                glyph_color: color,
                busy: joining,
                level: Some(f32::from(active.strength) / 100.0),
                level_color: strength_color(active.strength),
            },
        ),
        None => w
            .base("bar-network")
            .gap(gpui::px(ICON_GAP))
            .when_some(glyph, |widget, path| {
                widget.child(icon::glyph(path, GLYPH, color))
            }),
    };
    let shield = glyphs::vpn_on(state)
        .then(|| w.glyph("network-vpn"))
        .flatten()
        .map(|path| icon::glyph(path, GLYPH, MAUVE));
    base.children(shield)
        .tip(w.tips(), "bar-network", tip(state))
        .on_click(w.toggle(Panel::Feature(<Network as Feature>::NAME)))
        .into_any_element()
}
