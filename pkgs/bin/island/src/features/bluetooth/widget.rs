//! The bar widget: the Bluetooth glyph, blue while Bluetooth is on, with the lowest battery of what
//! is connected beside it when anything connected reports one.

use gpui::{AnyElement, prelude::*, px};

use super::{Bluetooth, glyphs, state::State};
use crate::{
    bar::{GLYPH, Gauge, ICON_GAP},
    components::{icon, tooltip::Tip},
    features::{Feature, WidgetCx},
    island::Panel,
    theme::{BLUE, OVERLAY2},
};

fn tip(state: &State) -> String {
    if !state.available {
        return "No Bluetooth adapter".to_owned();
    }
    if !state.powered {
        return "Bluetooth is off".to_owned();
    }
    let connected: Vec<String> = state
        .connected()
        .map(|device| match device.battery {
            Some(battery) => format!("{} {battery}%", device.name),
            None => device.name.clone(),
        })
        .collect();
    if connected.is_empty() {
        "Nothing connected".to_owned()
    } else {
        connected.join(" · ")
    }
}

pub fn widget(state: &State, w: &mut WidgetCx<'_>) -> AnyElement {
    let on = state.available && state.powered;
    let busy = state.devices.iter().any(|device| device.busy.is_some());
    let color = if on { BLUE } else { OVERLAY2 };
    let glyph = w.glyph(if on { "bt-on" } else { "bt-off" });
    let base = match state.lowest_battery() {
        Some(level) => w.gauge(
            "bar-bluetooth",
            Gauge {
                glyph,
                glyph_color: color,
                busy,
                level: Some(f32::from(level) / 100.0),
                level_color: glyphs::battery(Some(level)),
            },
        ),
        None => w
            .base("bar-bluetooth")
            .gap(px(ICON_GAP))
            .when_some(glyph, |widget, path| {
                widget.child(icon::glyph(path, GLYPH, color))
            }),
    };
    base.tip(w.tips(), "bar-bluetooth", tip(state))
    .on_click(w.toggle(Panel::Feature(<Bluetooth as Feature>::NAME)))
    .into_any_element()
}
