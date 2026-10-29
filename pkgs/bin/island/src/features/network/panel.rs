//! The Network tab: icon buttons over a board of tiles, four columns of three units each, and the
//! card asking for a password while one is wanted.

use gpui::SharedString;

use super::{
    Network, glyphs,
    runtime::{PROBE_EVERY, Request, SAMPLE_EVERY, phase},
    state::{AccessPoint, Activity, Security, State, Vpn, channel},
    view::{Prompt, View},
};
use crate::{
    components::{
        card::stat,
        tile::{Entry, EntryLead, Part, Tile},
    },
    features::PanelCx,
    panel::{Block, ButtonRow, Header, IconButton, InputView, PanelView, PromptView, Reveal},
    panels::{nice_peak, rate, series, short_duration, tile_meter, titled},
    theme::{BLUE, GREEN, MAUVE, OVERLAY2, PEACH, PINK, RED, SUBTEXT1, TEAL, YELLOW},
};

/// Decimal sizes, as data used is usually counted: "1.4 GB", "312 MB".
pub(super) fn bytes(count: u64) -> String {
    let count = count as f64;
    match count {
        count if count >= 1e12 => format!("{:.1} TB", count / 1e12),
        count if count >= 1e9 => format!("{:.1} GB", count / 1e9),
        count if count >= 1e6 => format!("{:.0} MB", count / 1e6),
        count if count >= 1e3 => format!("{:.0} kB", count / 1e3),
        count => format!("{count:.0} B"),
    }
}

/// A tenth of a millisecond under 10 ms, so a gateway on the LAN does not read as 0.
pub(super) fn millis(value: Option<f32>) -> String {
    match value {
        None => "—".to_owned(),
        Some(ms) if ms < 10.0 => format!("{ms:.1} ms"),
        Some(ms) => format!("{ms:.0} ms"),
    }
}

pub(super) fn latency_color(ms: Option<f32>) -> u32 {
    match ms {
        Some(ms) if ms < 50.0 => GREEN,
        Some(ms) if ms < 150.0 => YELLOW,
        Some(_) => RED,
        None => OVERLAY2,
    }
}

fn fact(label: &str, value: impl Into<SharedString>) -> (SharedString, SharedString) {
    (label.to_owned().into(), value.into())
}

fn hero(name: impl Into<SharedString>, lines: Vec<String>) -> Part {
    Part::Hero {
        name: name.into(),
        lines: lines.into_iter().map(SharedString::from).collect(),
    }
}

fn joining<'a>(state: &'a State, ssid: &str) -> Option<&'a super::state::Joining> {
    state
        .joining
        .as_ref()
        .filter(|joining| joining.ssid == ssid)
}

fn wifi_tile(state: &State) -> Tile {
    let wifi = state.wifi.as_ref();
    let active = wifi.and_then(|wifi| wifi.active());
    let connecting = state
        .joining
        .as_ref()
        .filter(|joining| joining.failed.is_none());
    let (stat_text, parts) = match (wifi, active, connecting) {
        (Some(wifi), _, _) if !wifi.enabled => (
            stat("Off", OVERLAY2),
            vec![hero(
                "Wi-Fi is off",
                vec!["Turn it on with the button above".into()],
            )],
        ),
        (_, _, Some(joining)) => (
            stat("…", BLUE),
            vec![hero(joining.ssid.clone(), vec!["Connecting…".into()])],
        ),
        (Some(wifi), Some(active), None) => (
            stat(format!("{}%", active.strength), TEAL),
            vec![
                hero(
                    active.ssid.clone(),
                    vec![
                        format!(
                            "{} · {} · channel {}",
                            active.security.label(),
                            active.band(),
                            active.channel()
                        ),
                        format!(
                            "{} · {}",
                            wifi.interface,
                            if active.saved { "saved" } else { "not saved" }
                        ),
                    ],
                ),
                Part::Spacer,
                Part::Facts(vec![
                    fact("Rate", format!("{} Mb/s", wifi.bitrate)),
                    fact("Max", format!("{} Mb/s", active.max_bitrate)),
                    fact("Strength", format!("{}%", active.strength)),
                ]),
            ],
        ),
        _ => (
            stat("—", OVERLAY2),
            vec![hero("Not connected", vec!["Pick a network to join".into()])],
        ),
    };
    titled("network-wifi", 1, "Wi-Fi", Some(stat_text), parts)
}

fn signal_tile(state: &State) -> Tile {
    let signal = &state.signal;
    let history: Vec<f32> = signal
        .history
        .iter()
        .map(|dbm| (dbm + 100.0).clamp(0.0, 70.0))
        .collect();
    let retries = signal.retries.unwrap_or(0.0);
    titled(
        "network-signal",
        1,
        "Signal",
        Some(stat(
            signal
                .dbm
                .map_or_else(|| "—".to_owned(), |dbm| format!("{dbm} dBm")),
            TEAL,
        )),
        vec![
            Part::Chart {
                series: vec![series(&history, 70.0, TEAL, "Signal", |value| {
                    format!("{:.0} dBm", value - 100.0)
                })],
                grow: true,
                phase: Some(phase(state.traffic.sampled, SAMPLE_EVERY)),
            },
            Part::Meters {
                rows: vec![
                    tile_meter(
                        "Quality",
                        signal.quality.unwrap_or(0.0),
                        signal.quality.map_or_else(
                            || "—".to_owned(),
                            |quality| format!("{:.0}%", quality * 100.0),
                        ),
                        TEAL,
                    ),
                    tile_meter(
                        "Retries",
                        retries / 20.0,
                        signal
                            .retries
                            .map_or_else(|| "—".to_owned(), |retries| format!("{retries:.1}%")),
                        if retries > 5.0 { YELLOW } else { GREEN },
                    ),
                ],
                scroll: false,
            },
        ],
    )
}

fn latency_tile(state: &State) -> Tile {
    let latency = &state.latency;
    let peak = nice_peak(
        latency
            .gateway_history
            .iter()
            .chain(&latency.internet_history),
        20.0,
    );
    titled(
        "network-latency",
        1,
        "Latency",
        Some(stat(
            millis(latency.internet),
            latency_color(latency.internet),
        )),
        vec![
            Part::Chart {
                series: vec![
                    series(
                        &latency.internet_history,
                        peak,
                        PEACH,
                        latency.target.clone(),
                        |value| format!("{value:.0} ms"),
                    ),
                    series(&latency.gateway_history, peak, TEAL, "Gateway", |value| {
                        format!("{value:.0} ms")
                    }),
                ],
                grow: true,
                phase: Some(phase(state.latency.sampled, PROBE_EVERY)),
            },
            Part::Legend(vec![
                (
                    Some(TEAL),
                    format!("Gateway {}", millis(latency.gateway)).into(),
                ),
                (Some(PEACH), millis(latency.internet).into()),
                (None, format!("Loss {:.0}%", latency.loss * 100.0).into()),
            ]),
        ],
    )
}

fn network_entry(network: &AccessPoint, state: &State, p: &PanelCx<'_>) -> Entry {
    let attempt = joining(state, &network.ssid);
    let connecting = attempt.is_some_and(|joining| joining.failed.is_none());
    let detail = match (attempt, network.active) {
        (Some(joining), _) if joining.failed.is_none() => "Connecting…".to_owned(),
        (Some(joining), _) => joining.failed.clone().unwrap_or_default(),
        (None, true) => format!("Connected · {}", network.security.label()),
        (None, false) if network.saved => format!("Saved · {}", network.security.label()),
        (None, false) if network.security == Security::Enterprise => {
            "Enterprise · set it up in NetworkManager".to_owned()
        }
        (None, false) => format!("{} · {}", network.security.label(), network.band()),
    };
    let color = if network.active {
        TEAL
    } else if connecting {
        BLUE
    } else {
        SUBTEXT1
    };
    let ssid = network.ssid.clone();
    let on_click = if network.active || connecting {
        None
    } else if network.saved || network.security == Security::Open {
        Some(p.call(move |network: &Network| network.send(Request::Connect(ssid.clone()))))
    } else if network.security == Security::Enterprise {
        None
    } else {
        Some(p.on_local::<Network>(move |_, view: &mut View| view.ask(&ssid)))
    };
    Entry {
        id: network.ssid.clone().into(),
        lead: Some(EntryLead::Glyph(
            p.glyph(glyphs::signal(network.strength)),
            color,
        )),
        title: network.ssid.clone().into(),
        title_color: network.active.then_some(TEAL),
        detail: detail.into(),
        value: format!("{}%", network.strength).into(),
        value_color: if network.active { TEAL } else { OVERLAY2 },
        chosen: network.active.then_some(TEAL),
        on_click,
    }
}

fn networks_tile(state: &State, p: &PanelCx<'_>) -> Tile {
    let Some(wifi) = &state.wifi else {
        return titled("network-networks", 2, "Networks", None, Vec::new());
    };
    if !wifi.enabled {
        return titled(
            "network-networks",
            2,
            "Networks",
            None,
            vec![hero("Wi-Fi is off", Vec::new())],
        );
    }
    let count = wifi.networks.len();
    titled(
        "network-networks",
        2,
        "Networks",
        Some(if wifi.scanning {
            stat("Scanning…", BLUE)
        } else {
            stat(format!("{count} nearby"), BLUE)
        }),
        vec![Part::Entries {
            entries: wifi
                .networks
                .iter()
                .map(|network| network_entry(network, state, p))
                .collect(),
            scroll: true,
        }],
    )
}

fn traffic_tile(state: &State) -> Tile {
    let traffic = &state.traffic;
    let peak = nice_peak(
        traffic.rx_history.iter().chain(&traffic.tx_history),
        64.0 * 1024.0,
    );
    titled(
        "network-traffic",
        1,
        "Traffic",
        Some(stat(format!("↓ {}", rate(traffic.rx_rate)), TEAL)),
        vec![
            Part::Chart {
                series: vec![
                    series(&traffic.rx_history, peak, TEAL, "Download", |value| {
                        rate(value as f64)
                    }),
                    series(&traffic.tx_history, peak, PEACH, "Upload", |value| {
                        rate(value as f64)
                    }),
                ],
                grow: true,
                phase: Some(phase(state.traffic.sampled, SAMPLE_EVERY)),
            },
            Part::Legend(vec![
                (Some(TEAL), format!("↓ {}", rate(traffic.rx_rate)).into()),
                (Some(PEACH), format!("↑ {}", rate(traffic.tx_rate)).into()),
            ]),
        ],
    )
}

fn vpn_entry(vpn: &Vpn, p: &PanelCx<'_>) -> Entry {
    let color = match vpn.activity {
        Activity::On => MAUVE,
        Activity::Connecting => YELLOW,
        Activity::Off => OVERLAY2,
    };
    let name = vpn.name.clone();
    Entry {
        id: vpn.name.clone().into(),
        lead: Some(EntryLead::Glyph(p.glyph("network-vpn"), color)),
        title: vpn.name.clone().into(),
        title_color: (vpn.activity == Activity::On).then_some(MAUVE),
        detail: vpn
            .endpoint
            .clone()
            .unwrap_or_else(|| vpn.kind.clone())
            .into(),
        value: match vpn.activity {
            Activity::On => "On".into(),
            Activity::Connecting => "Connecting…".into(),
            Activity::Off => vpn.kind.clone().into(),
        },
        value_color: color,
        chosen: (vpn.activity == Activity::On).then_some(MAUVE),
        on_click: Some(p.call(move |network: &Network| {
            network.send(Request::Vpn(name.clone(), None));
        })),
    }
}

fn vpn_tile(state: &State, span: usize, p: &PanelCx<'_>) -> Tile {
    let active = state.vpns.iter().find(|vpn| vpn.activity == Activity::On);
    let stat_text = match active {
        Some(vpn) => stat(
            vpn.address.clone().unwrap_or_else(|| "On".to_owned()),
            MAUVE,
        ),
        None => stat("Off", OVERLAY2),
    };
    if state.vpns.is_empty() {
        return titled(
            "network-vpn",
            span,
            "VPN",
            Some(stat_text),
            vec![hero(
                "No VPNs",
                vec![
                    "Add one to NetworkManager".into(),
                    "to switch it here".into(),
                ],
            )],
        );
    }
    let mut parts = vec![Part::Entries {
        entries: state.vpns.iter().map(|vpn| vpn_entry(vpn, p)).collect(),
        scroll: span == 1,
    }];
    if span > 1 {
        let now = chrono::Utc::now().timestamp();
        let traffic = &state.traffic;
        let peak = nice_peak(&traffic.vpn_history, 64.0 * 1024.0);
        parts.push(Part::Chart {
            series: vec![series(
                &traffic.vpn_history,
                peak,
                MAUVE,
                "Through the VPN",
                |value| rate(value as f64),
            )],
            grow: true,
            phase: Some(phase(state.traffic.sampled, SAMPLE_EVERY)),
        });
        parts.push(Part::Facts(match active {
            Some(vpn) => vec![
                fact(
                    "Interface",
                    vpn.interface.clone().unwrap_or_else(|| "—".to_owned()),
                ),
                fact(
                    "Up for",
                    vpn.since.map_or_else(
                        || "—".to_owned(),
                        |since| short_duration((now - since).max(0) as u64),
                    ),
                ),
                fact("Kind", vpn.kind.clone()),
                fact("Received", bytes(traffic.vpn_rx)),
                fact("Sent", bytes(traffic.vpn_tx)),
                fact(
                    "Address",
                    vpn.address.clone().unwrap_or_else(|| "—".to_owned()),
                ),
            ],
            None => vec![
                fact("Profiles", state.vpns.len().to_string()),
                fact("Received", "—"),
                fact("Sent", "—"),
            ],
        }));
    }
    titled("network-vpn", span, "VPN", Some(stat_text), parts)
}

fn connection_tile(state: &State) -> Tile {
    let Some(link) = &state.link else {
        return titled(
            "network-connection",
            1,
            "Connection",
            Some(stat("Offline", OVERLAY2)),
            vec![hero("Offline", vec!["No connection is up".into()])],
        );
    };
    let now = chrono::Utc::now().timestamp();
    let via = link.gateway.as_ref().map_or_else(
        || "no gateway".to_owned(),
        |gateway| format!("via {gateway}"),
    );
    let dns = if link.dns.is_empty() {
        "No DNS server".to_owned()
    } else {
        format!("DNS {}", link.dns.join(", "))
    };
    titled(
        "network-connection",
        1,
        "Connection",
        Some(stat(link.interface.clone(), SUBTEXT1)),
        vec![
            hero(
                link.ipv4.clone().unwrap_or_else(|| "No address".to_owned()),
                vec![format!("/{} {via}", link.prefix), dns],
            ),
            Part::Spacer,
            Part::Facts(vec![
                fact("IPv6", link.ipv6.as_ref().map_or("none", |_| "yes")),
                fact("MTU", link.mtu.to_string()),
                fact(
                    "Lease",
                    link.lease.map_or_else(
                        || "static".to_owned(),
                        |lease| short_duration((lease - now).max(0) as u64),
                    ),
                ),
            ]),
        ],
    )
}

fn usage_tile(state: &State) -> Tile {
    let usage = &state.usage;
    let most = usage
        .days
        .iter()
        .map(|(_, bytes)| *bytes)
        .max()
        .unwrap_or(0)
        .max(1);
    titled(
        "network-usage",
        1,
        "Data used",
        Some(stat(bytes(usage.today_rx + usage.today_tx), TEAL)),
        vec![
            Part::Meters {
                rows: usage
                    .days
                    .iter()
                    .rev()
                    .map(|(day, used)| {
                        tile_meter(
                            day.clone(),
                            *used as f32 / most as f32,
                            bytes(*used),
                            if day == "Today" { TEAL } else { BLUE },
                        )
                    })
                    .collect(),
                scroll: true,
            },
            Part::Legend(vec![
                (Some(TEAL), format!("↓ {}", bytes(usage.today_rx)).into()),
                (Some(PEACH), format!("↑ {}", bytes(usage.today_tx)).into()),
                (None, "today".into()),
            ]),
        ],
    )
}

fn access_point_tile(state: &State) -> Tile {
    let Some((wifi, active)) = state
        .wifi
        .as_ref()
        .and_then(|wifi| wifi.active().map(|active| (wifi, active)))
    else {
        return titled(
            "network-ap",
            1,
            "Access point",
            Some(stat("—", OVERLAY2)),
            vec![hero("Not connected", Vec::new())],
        );
    };
    let width = if active.width > 0 {
        format!("{} MHz wide", active.width)
    } else {
        wifi.interface.clone()
    };
    titled(
        "network-ap",
        1,
        "Access point",
        Some(stat(format!("Channel {}", active.channel()), TEAL)),
        vec![
            hero(
                active.band(),
                vec![
                    format!("{} MHz · {width}", active.frequency),
                    active.bssid.clone(),
                ],
            ),
            Part::Spacer,
            Part::Facts(vec![
                fact("Security", active.security.label()),
                fact("Channel", active.channel().to_string()),
                fact("Max", format!("{} Mb/s", active.max_bitrate)),
            ]),
        ],
    )
}

fn channels_tile(state: &State) -> Tile {
    let Some(wifi) = &state.wifi else {
        return titled("network-channels", 1, "Channels", None, Vec::new());
    };
    let mine = wifi.active().map(AccessPoint::channel);
    let mut counts: Vec<(u32, u32, usize)> = Vec::new();
    for network in &wifi.networks {
        let key = (network.frequency / 1000, channel(network.frequency));
        match counts.iter_mut().find(|(band, ch, _)| (*band, *ch) == key) {
            Some(entry) => entry.2 += 1,
            None => counts.push((key.0, key.1, 1)),
        }
    }
    counts.sort();
    let most = counts.iter().map(|entry| entry.2).max().unwrap_or(1);
    let shared = mine
        .and_then(|mine| counts.iter().find(|entry| entry.1 == mine))
        .map_or(0, |entry| entry.2.saturating_sub(1));
    titled(
        "network-channels",
        1,
        "Channels",
        Some(match (mine, shared) {
            (None, _) => stat("—", OVERLAY2),
            (Some(_), 0) => stat("Yours is clear", GREEN),
            (Some(_), shared) => stat(format!("{shared} share yours"), YELLOW),
        }),
        vec![Part::Meters {
            rows: counts
                .iter()
                .map(|(band, ch, count)| {
                    let band = if *band >= 5 { "5 GHz" } else { "2.4 GHz" };
                    tile_meter(
                        format!("{ch} · {band}"),
                        *count as f32 / most as f32,
                        format!("{count} network{}", if *count == 1 { "" } else { "s" }),
                        if Some(*ch) == mine { TEAL } else { BLUE },
                    )
                })
                .collect(),
            scroll: true,
        }],
    )
}

fn interfaces_tile(state: &State) -> Tile {
    let up = state
        .interfaces
        .iter()
        .filter(|interface| interface.up)
        .count();
    titled(
        "network-interfaces",
        1,
        "Interfaces",
        Some(stat(format!("{up} up"), SUBTEXT1)),
        vec![Part::Entries {
            entries: state
                .interfaces
                .iter()
                .map(|interface| Entry {
                    id: interface.name.clone().into(),
                    lead: Some(EntryLead::Dot(if interface.up { GREEN } else { OVERLAY2 })),
                    title: interface.name.clone().into(),
                    title_color: None,
                    detail: interface.kind.clone().into(),
                    value: if interface.up {
                        rate(interface.rate).into()
                    } else {
                        "down".into()
                    },
                    value_color: OVERLAY2,
                    chosen: None,
                    on_click: None,
                })
                .collect(),
            scroll: true,
        }],
    )
}

fn ethernet_tile(state: &State) -> Tile {
    let Some(wired) = &state.wired else {
        return titled(
            "network-ethernet",
            1,
            "Ethernet",
            Some(stat("—", OVERLAY2)),
            vec![hero("No wired port", Vec::new())],
        );
    };
    let (stat_text, name, line) = if wired.active {
        (
            stat(format!("{} Mb/s", wired.speed), TEAL),
            wired.name.clone(),
            format!("{} · {} Mbit/s", wired.interface, wired.speed),
        )
    } else if wired.carrier {
        (
            stat("Idle", OVERLAY2),
            "Not connected".to_owned(),
            wired.interface.clone(),
        )
    } else {
        (
            stat("No cable", OVERLAY2),
            "No cable".to_owned(),
            format!("Plug a cable into {}", wired.interface),
        )
    };
    titled(
        "network-ethernet",
        1,
        "Ethernet",
        Some(stat_text),
        vec![
            hero(name, vec![line, format!("MAC {}", wired.mac)]),
            Part::Spacer,
            Part::Facts(vec![
                fact("Errors", wired.errors.to_string()),
                fact("Drops", wired.drops.to_string()),
                fact("Carrier", wired.carrier_changes.to_string()),
            ]),
        ],
    )
}

fn routes_tile(state: &State) -> Tile {
    titled(
        "network-routes",
        1,
        "Routes",
        Some(stat(state.routes.len().to_string(), SUBTEXT1)),
        vec![Part::Entries {
            entries: state
                .routes
                .iter()
                .enumerate()
                .map(|(index, route)| Entry {
                    id: format!("{index}").into(),
                    lead: None,
                    title: route.destination.clone().into(),
                    title_color: None,
                    detail: route
                        .via
                        .as_ref()
                        .map_or_else(|| "on link".to_owned(), |via| format!("via {via}"))
                        .into(),
                    value: route.interface.clone().into(),
                    value_color: OVERLAY2,
                    chosen: None,
                    on_click: None,
                })
                .collect(),
            scroll: true,
        }],
    )
}

fn connections_tile(state: &State) -> Tile {
    let sockets = &state.sockets;
    let total = (sockets.established + sockets.listening + sockets.waiting).max(1) as f32;
    let busiest = sockets.apps.first().map_or(1, |(_, count)| *count).max(1) as f32;
    titled(
        "network-connections",
        1,
        "Connections",
        Some(stat(sockets.established.to_string(), BLUE)),
        vec![
            Part::Meters {
                rows: vec![
                    tile_meter(
                        "Open",
                        sockets.established as f32 / total,
                        sockets.established.to_string(),
                        BLUE,
                    ),
                    tile_meter(
                        "Listening",
                        sockets.listening as f32 / total,
                        sockets.listening.to_string(),
                        MAUVE,
                    ),
                    tile_meter(
                        "Closing",
                        sockets.waiting as f32 / total,
                        sockets.waiting.to_string(),
                        OVERLAY2,
                    ),
                ],
                scroll: false,
            },
            Part::Meters {
                rows: sockets
                    .apps
                    .iter()
                    .take(3)
                    .map(|(app, count)| {
                        tile_meter(
                            app.clone(),
                            *count as f32 / busiest,
                            count.to_string(),
                            PINK,
                        )
                    })
                    .collect(),
                scroll: false,
            },
        ],
    )
}

fn dns_tile(state: &State) -> Tile {
    let dns = &state.dns;
    let link = state.link.as_ref();
    let server = |index: usize| {
        link.and_then(|link| link.dns.get(index).cloned())
            .unwrap_or_else(|| "—".to_owned())
    };
    let peak = nice_peak(&dns.history, 20.0);
    titled(
        "network-dns",
        1,
        "DNS",
        Some(stat(millis(dns.lookup), latency_color(dns.lookup))),
        vec![
            Part::Chart {
                series: vec![series(&dns.history, peak, BLUE, "Lookup", |value| {
                    format!("{value:.0} ms")
                })],
                grow: true,
                phase: Some(phase(state.dns.sampled, PROBE_EVERY)),
            },
            Part::Facts(vec![
                fact("Server", server(0)),
                fact(
                    "Domain",
                    link.and_then(|link| link.domain.clone())
                        .unwrap_or_else(|| "—".to_owned()),
                ),
            ]),
        ],
    )
}

fn ports_tile(state: &State) -> Tile {
    titled(
        "network-ports",
        1,
        "Listening ports",
        Some(stat(state.ports.len().to_string(), MAUVE)),
        vec![Part::Entries {
            entries: state
                .ports
                .iter()
                .map(|port| Entry {
                    id: format!("{}-{}", port.protocol, port.port).into(),
                    lead: None,
                    title: port.app.clone().into(),
                    title_color: None,
                    detail: port.protocol.into(),
                    value: port.port.to_string().into(),
                    value_color: MAUVE,
                    chosen: None,
                    on_click: None,
                })
                .collect(),
            scroll: true,
        }],
    )
}

fn buttons(state: &State, p: &PanelCx<'_>) -> Vec<IconButton> {
    let mut buttons = Vec::new();
    if let Some(wifi) = &state.wifi {
        buttons.push(IconButton {
            id: "network-wifi-switch".into(),
            glyph: p.glyph(if wifi.enabled {
                "network-wifi-excellent"
            } else {
                "network-wifi-off"
            }),
            color: None,
            label: if wifi.enabled {
                "Wi-Fi is on"
            } else {
                "Wi-Fi is off"
            }
            .into(),
            active: wifi.enabled,
            badge: None,
            on_click: p.call(|network: &Network| network.send(Request::Wifi(None))),
        });
        if wifi.enabled {
            buttons.push(IconButton {
                id: "network-scan".into(),
                glyph: p.glyph("network-scan"),
                color: None,
                label: "Scan for networks".into(),
                active: wifi.scanning,
                badge: Some(wifi.networks.len().to_string().into()),
                on_click: p.call(|network: &Network| network.send(Request::Scan)),
            });
        }
        if let Some(active) = wifi.active() {
            buttons.push(IconButton {
                id: "network-disconnect".into(),
                glyph: p.glyph("network-disconnect"),
                color: None,
                label: format!("Disconnect from {}", active.ssid).into(),
                active: false,
                badge: None,
                on_click: p.call(|network: &Network| network.send(Request::Disconnect)),
            });
            if active.saved {
                let ssid = active.ssid.clone();
                buttons.push(IconButton {
                    id: "network-forget".into(),
                    glyph: p.glyph("network-forget"),
                    color: None,
                    label: format!("Forget {}", active.ssid).into(),
                    active: false,
                    badge: None,
                    on_click: p.call(move |network: &Network| {
                        network.send(Request::Forget(ssid.clone()));
                    }),
                });
            }
        }
    }
    if let Some(vpn) = state
        .vpns
        .iter()
        .find(|vpn| vpn.activity != Activity::Off)
        .or_else(|| state.vpns.first())
    {
        let on = vpn.activity == Activity::On;
        let name = vpn.name.clone();
        buttons.push(IconButton {
            id: "network-vpn-switch".into(),
            glyph: p.glyph("network-vpn"),
            color: None,
            label: if on {
                format!("Turn off {}", vpn.name)
            } else {
                format!("Turn on {}", vpn.name)
            }
            .into(),
            active: on,
            badge: None,
            on_click: p
                .call(move |network: &Network| network.send(Request::Vpn(name.clone(), None))),
        });
    }
    buttons
}

fn prompt(state: &State, prompt: &Prompt, p: &PanelCx<'_>) -> PromptView {
    let security = state
        .wifi
        .as_ref()
        .and_then(|wifi| {
            wifi.networks
                .iter()
                .find(|network| network.ssid == prompt.ssid)
        })
        .map_or(Security::Wpa2, |network| network.security);
    let attempt = joining(state, &prompt.ssid);
    let busy = attempt.is_some_and(|joining| joining.failed.is_none());
    let ready = !prompt.text.is_empty() && !busy;
    PromptView {
        id: "network-prompt".into(),
        title: format!("Join {}", prompt.ssid).into(),
        detail: format!("{} · enter its password", security.label()).into(),
        code: None,
        confirm: None,
        field: Some(InputView {
            id: "network-password".into(),
            text: prompt.text.clone().into(),
            placeholder: "Password".into(),
            max_lines: 1,
            send_glyph: p.glyph("network-join"),
            on_send: ready
                .then(|| p.on_local::<Network>(|network, view: &mut View| view.submit(network))),
            busy,
            secret: !prompt.shown,
        }),
        reveal: Some(Reveal {
            glyph: p.glyph(if prompt.shown {
                "network-conceal"
            } else {
                "network-reveal"
            }),
            shown: prompt.shown,
            on_click: p.on_local::<Network>(|_, view: &mut View| {
                if let Some(prompt) = &mut view.prompt {
                    prompt.shown = !prompt.shown;
                }
            }),
        }),
        error: attempt
            .and_then(|joining| joining.failed.clone())
            .map(SharedString::from),
        on_cancel: p.on_local::<Network>(|_, view: &mut View| view.prompt = None),
    }
}

pub fn panel(state: &State, p: &PanelCx<'_>) -> PanelView {
    let columns = if state.wifi.is_some() {
        vec![
            vec![wifi_tile(state), signal_tile(state), latency_tile(state)],
            vec![networks_tile(state, p), traffic_tile(state)],
            vec![
                vpn_tile(state, 1, p),
                connection_tile(state),
                usage_tile(state),
            ],
            vec![
                access_point_tile(state),
                channels_tile(state),
                interfaces_tile(state),
            ],
        ]
    } else {
        vec![
            vec![
                ethernet_tile(state),
                traffic_tile(state),
                latency_tile(state),
            ],
            vec![
                connection_tile(state),
                usage_tile(state),
                interfaces_tile(state),
            ],
            vec![vpn_tile(state, 2, p), routes_tile(state)],
            vec![connections_tile(state), dns_tile(state), ports_tile(state)],
        ]
    };
    let mut blocks = Vec::new();
    if !state.reachable {
        blocks.push(Block::caption(
            "NetworkManager is not answering, so this shows only what the kernel knows.",
            PEACH,
        ));
    }
    blocks.push(Block::Buttons(ButtonRow {
        buttons: buttons(state, p),
        compact: false,
        indent: 0.0,
        menu: None,
        end: false,
    }));
    blocks.push(Block::Board(columns));
    if let Some(asked) = p.local::<View>().and_then(|view| view.prompt.as_ref()) {
        blocks.push(Block::Prompt(prompt(state, asked, p)));
    }
    PanelView {
        scroll: false,
        header: Header::Tabs,
        blocks,
    }
}
