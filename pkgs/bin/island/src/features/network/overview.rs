//! What the network puts on the dashboard's overview: switches for Wi-Fi or the wired link and for
//! a VPN, a card of how fast it moves now, and a warning while the internet does not answer.

use gpui::SharedString;

use super::{
    Network,
    panel::{bytes, latency_color, millis},
    runtime::{Request, SAMPLE_EVERY, phase},
    state::{Activity, State},
};
use crate::{
    components::card::{Size, Title},
    features::{Feature, PanelCx},
    island::{Mode, Panel},
    panel::{self, ActionTile, Block, ChartView, Handler, Meter, MeterShape, Warning},
    panels::{nice_peak, rate, series},
    theme::{LAVENDER, OVERLAY2, RED, SKY, TEAL},
};

const CHART_H: f32 = 56.0;

fn open() -> Handler {
    panel::on(|island, cx| island.set_mode(Mode::Open(Panel::Feature(Network::NAME)), cx))
}

pub fn switches(state: &State, p: &PanelCx<'_>) -> Vec<ActionTile> {
    let mut tiles = Vec::new();
    if let Some(wifi) = &state.wifi {
        let on = wifi.enabled;
        tiles.push(ActionTile {
            id: "switch-wifi".into(),
            glyph: p.glyph(if on {
                "network-wifi-excellent"
            } else {
                "network-wifi-off"
            }),
            label: "Wi-Fi".into(),
            state: Some(match wifi.active() {
                _ if !on => "Off".into(),
                Some(network) => network.ssid.clone().into(),
                None => "Not connected".into(),
            }),
            color: TEAL,
            armed: on,
            on_click: p.call(move |network: &Network| network.send(Request::Wifi(Some(!on)))),
        });
    } else if let Some(wired) = &state.wired {
        tiles.push(ActionTile {
            id: "switch-wired".into(),
            glyph: p.glyph(if wired.active {
                "network-wired"
            } else {
                "network-wired-off"
            }),
            label: "Ethernet".into(),
            state: Some(if wired.active {
                link_speed(wired.speed).into()
            } else {
                "Not connected".into()
            }),
            color: TEAL,
            armed: wired.active,
            on_click: open(),
        });
    }
    // With several, the switch turns off the one that is on, or else turns on the first.
    let vpn = state
        .vpns
        .iter()
        .find(|vpn| vpn.activity != Activity::Off)
        .or_else(|| state.vpns.first());
    if let Some(vpn) = vpn {
        let (name, on) = (vpn.name.clone(), vpn.activity != Activity::Off);
        tiles.push(ActionTile {
            id: "switch-vpn".into(),
            glyph: p.glyph("network-vpn"),
            label: "VPN".into(),
            state: Some(match vpn.activity {
                Activity::On => name.clone().into(),
                Activity::Connecting => "Connecting…".into(),
                Activity::Off => "Off".into(),
            }),
            color: LAVENDER,
            armed: on,
            on_click: p.call(move |network: &Network| {
                network.send(Request::Vpn(name.clone(), Some(!on)))
            }),
        });
    }
    tiles
}

/// Such as "2.5 Gb/s" from 2500 Mbit/s.
fn link_speed(mbits: u32) -> String {
    match mbits {
        0 => "Connected".to_owned(),
        speed if speed >= 1000 => format!("{} Gb/s", speed as f32 / 1000.0),
        speed => format!("{speed} Mb/s"),
    }
}

/// "32.7 kB/s" as the number and its unit.
fn split_rate(bytes_per_second: f64) -> (String, String) {
    let text = rate(bytes_per_second);
    match text.split_once(' ') {
        Some((value, unit)) => (value.to_owned(), unit.to_owned()),
        None => (text, String::new()),
    }
}

pub fn cards(state: &State, p: &PanelCx<'_>) -> Vec<Block> {
    let traffic = &state.traffic;
    let name = state
        .link
        .as_ref()
        .map(|link| link.name.clone())
        .or_else(|| state.wired.as_ref().map(|wired| wired.name.clone()))
        .unwrap_or_default();
    let (down, down_unit) = split_rate(traffic.rx_rate);
    let (up, up_unit) = split_rate(traffic.tx_rate);
    let latency = state.latency.internet;
    let number = |id: &str, label: &str, amount: String, unit: &str, color: u32| Meter {
        id: id.to_owned().into(),
        glyph: None,
        caption: SharedString::default(),
        label: label.to_owned().into(),
        level: None,
        color,
        shape: MeterShape::History {
            amount: amount.into(),
            unit: unit.to_owned().into(),
            history: None,
        },
    };
    let numbers = vec![
        number("network-down", "Download", down, &down_unit, TEAL),
        number("network-up", "Upload", up, &up_unit, SKY),
        number(
            "network-latency",
            "Latency",
            millis(latency).trim_end_matches(" ms").to_owned(),
            if latency.is_some() { "ms" } else { "" },
            latency_color(latency),
        ),
    ];
    let peak = nice_peak(
        traffic.rx_history.iter().chain(&traffic.tx_history),
        64.0 * 1024.0,
    );
    let chart = ChartView {
        id: "network-traffic-overview".into(),
        series: vec![
            series(&traffic.rx_history, peak, TEAL, "Download", |value| {
                rate(value as f64)
            }),
            series(&traffic.tx_history, peak, SKY, "Upload", |value| {
                rate(value as f64)
            }),
        ],
        height: CHART_H,
        phase: Some(phase(traffic.sampled, SAMPLE_EVERY)),
    };
    let today = state.usage.today_rx + state.usage.today_tx;
    let title = Title::with_detail("Network", name)
        .glyph(p.glyph("tab-network"), TEAL)
        .opens(open());
    vec![Block::Card(
        Some(title),
        Size::Medium,
        vec![
            Block::Meters(3, numbers),
            Block::Chart(chart),
            Block::caption(format!("{} today", bytes(today)), OVERLAY2),
        ],
    )]
}

pub fn warnings(state: &State, p: &PanelCx<'_>) -> Vec<Warning> {
    let linked = state.link.is_some();
    if !(linked && state.internet.missing()) {
        return Vec::new();
    }
    vec![Warning {
        id: "warning-internet".into(),
        glyph: p.glyph("network-offline"),
        label: "Internet".into(),
        value: "Not answering".into(),
        color: RED,
        on_click: Some(open()),
    }]
}
