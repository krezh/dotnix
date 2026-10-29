//! What Bluetooth puts on the dashboard's overview: its switch, and the batteries of what is
//! connected.

use super::{Bluetooth, glyphs, runtime::Request, state::State};
use crate::{
    features::{Battery, PanelCx},
    panel::ActionTile,
    theme::BLUE,
};

pub fn switches(state: &State, p: &PanelCx<'_>) -> Vec<ActionTile> {
    if !state.available {
        return Vec::new();
    }
    let on = state.powered;
    let connected = state.connected().count();
    vec![ActionTile {
        id: "switch-bluetooth".into(),
        glyph: p.glyph(if on { "bt-on" } else { "bt-off" }),
        label: "Bluetooth".into(),
        state: Some(match connected {
            _ if !on => "Off".into(),
            0 => "On".into(),
            1 => "1 connected".into(),
            count => format!("{count} connected").into(),
        }),
        color: BLUE,
        armed: on,
        on_click: p.call(move |bluetooth: &Bluetooth| bluetooth.send(Request::Power(Some(!on)))),
    }]
}

pub fn batteries(state: &State) -> Vec<Battery> {
    state
        .connected()
        .filter_map(|device| {
            Some(Battery {
                id: device.address.clone().into(),
                glyph: glyphs::device(&device.icon),
                name: device.name.clone().into(),
                level: device.battery?,
                charging: false,
            })
        })
        .collect()
}
