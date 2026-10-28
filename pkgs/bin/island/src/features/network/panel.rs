//! The Network tab: cards in two rows of fixed height, laid out from what the machine has. Each
//! control sits on what it changes and each number beside what it answers: whether the internet
//! works and which way it goes, Wi-Fi, VPNs, interfaces, traffic, and what is using the network.

use gpui::SharedString;

use super::{
    Network, glyphs,
    layout::{Card, layout},
    runtime::Request,
    state::{AccessPoint, Activity, Interface, Internet, Role, Security, State, Vpn},
    view::{Open, View},
};
use crate::{
    components::{
        card::{Look, Motion, Size, Title, TitleEnd},
        tabs::TabLabel,
    },
    features::PanelCx,
    panel::{
        Action, Actions, Bars, Block, ChartView, Disclosure, Figure, FrameView, Header, Hop, InputView,
        Item, Lead, Mark, PanelView, Path, SegmentedView, Text, Tone, Trail,
    },
    panels::{nice_peak, rate, series, short_duration},
    theme::{BLUE, GREEN, MAUVE, OVERLAY2, PEACH, PINK, RED, SUBTEXT1, TEAL, YELLOW},
};

/// The rows' heights: the first holds a card of lists that open, the second a shorter one.
const TOP_H: f32 = 408.0;
const BOTTOM_H: f32 = 276.0;
const LEAD: f32 = 30.0;
/// Where a list's rules and what a row opens to start: past the frame's padding, the lead and the
/// gap after it.
const INDENT: f32 = 8.0 + LEAD + crate::components::item::LEAD_GAP;
/// What fits in the Wi-Fi card under the network in use.
const NETWORKS: usize = 6;
/// What fits in the Activity card.
const ENTRIES: usize = 4;
/// The height a top-row card has for what is under its title: the row, less the card's padding,
/// its title and the gap after it.
const CARD_BODY_H: f32 = TOP_H - 2.0 * 16.0 - 28.0 - 12.0;
/// A two-line row in a list.
const ROW_H: f32 = 48.0;
/// What an opened tunnel row holds besides its chart: the room round it and the totals under it.
const DISCLOSED_AROUND_CHART: f32 = 2.0 + 8.0 + 46.0 + 10.0;
const CHART_LEAST_H: f32 = 60.0;
const CHART_MOST_H: f32 = 220.0;
/// The least the latency chart's scale reaches, so a LAN's fraction of a millisecond still shows
/// as small rather than filling it.
const LATENCY_FLOOR: f32 = 5.0;

/// Decimal sizes, as data used is usually counted: "1.4 GB", "312 MB".
fn bytes(count: u64) -> String {
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
fn millis(value: Option<f32>) -> String {
    match value {
        None => "—".to_owned(),
        Some(ms) if ms < 10.0 => format!("{ms:.1}"),
        Some(ms) => format!("{ms:.0}"),
    }
}

fn latency_color(ms: Option<f32>) -> u32 {
    match ms {
        Some(ms) if ms < 50.0 => GREEN,
        Some(ms) if ms < 150.0 => YELLOW,
        Some(_) => RED,
        None => OVERLAY2,
    }
}

/// A rate as its number and its unit, for a figure.
fn rate_parts(bytes_per_second: f64) -> (String, String) {
    let text = rate(bytes_per_second);
    match text.split_once(' ') {
        Some((value, unit)) => (value.to_owned(), unit.to_owned()),
        None => (text, String::new()),
    }
}

fn fact(label: impl Into<SharedString>, value: impl Into<SharedString>) -> (SharedString, SharedString, Option<u32>) {
    (label.into(), value.into(), None)
}

/// Solid in its color while on; a quiet grey square while off.
fn lead(p: &PanelCx<'_>, glyph: &str, color: u32, on: bool) -> Lead {
    Lead {
        mark: p.glyph(glyph).map(|path| Mark::Glyph(path, color)),
        tint: Some(color),
        glow: None,
        faded: !on,
        size: Some(LEAD),
        on_click: None,
        top: false,
    }
}

/// A two-line row: what it is over a fainter line saying how it is.
fn item(id: String, lead: Lead, title: impl Into<SharedString>, line: Block, trail: Vec<Trail>) -> Block {
    Block::Item(
        Item {
            id: id.into(),
            lead: Some(lead),
            overline: None,
            title: title.into(),
            title_color: None,
            wrap: false,
            detail: SharedString::default(),
            trail,
        },
        vec![line],
    )
}

/// A row's second line.
fn line(text: impl Into<SharedString>) -> Block {
    Block::Text(Text::new(Tone::Detail(OVERLAY2), text))
}

/// A row of a list, clickable when it does something, with what it opens to under it.
fn row(id: String, item: Block, opens: Option<(bool, Vec<Block>)>, on_click: Option<crate::panel::Handler>) -> Block {
    let mut blocks = vec![item];
    if let Some((open, inside)) = opens {
        blocks.push(Block::Disclose(
            Disclosure {
                id: format!("{id}-more").into(),
                open,
                indent: INDENT - 8.0,
            },
            inside,
        ));
    }
    Block::Frame(
        FrameView {
            id: id.into(),
            look: Look::Flat,
            on_click,
            on_dismiss: None,
            countdown: None,
            selected: false,
            motion: Motion::Still,
            shown_height: None,
        },
        blocks,
    )
}

fn card(title: Title, span: usize, blocks: Vec<Block>) -> Block {
    Block::Card(Some(title), Size::Tiles(span), blocks)
}

// Internet

fn status(state: &State) -> (String, u32) {
    if !state.reachable {
        return ("NetworkManager is not answering".to_owned(), PEACH);
    }
    let Some(link) = &state.link else {
        return ("Offline".to_owned(), RED);
    };
    match state.internet {
        Internet::None => return ("No internet".to_owned(), YELLOW),
        Internet::Portal => return ("Sign-in needed".to_owned(), YELLOW),
        Internet::Limited => return ("Limited".to_owned(), YELLOW),
        Internet::Full | Internet::Unknown => {}
    }
    let on_wifi = matches!(link.kind, super::state::LinkKind::Wifi);
    let text = match (state.tunnel(), on_wifi) {
        (Some(vpn), true) => format!("Online through {}, on Wi-Fi", vpn.name),
        (Some(vpn), false) => format!("Online through {}", vpn.name),
        (None, true) => "Online on Wi-Fi, direct".to_owned(),
        (None, false) => "Online, direct".to_owned(),
    };
    (text, GREEN)
}

fn internet_card(state: &State, span: usize, p: &PanelCx<'_>) -> Block {
    let latency = &state.latency;
    let tunnel = state.tunnel();
    let link = state.link.as_ref();
    let (status, color) = status(state);
    let title = Title::with_detail("Internet", "").ending(vec![TitleEnd::Pill {
        id: "network-status".into(),
        text: status.into(),
        color: Some(color),
        busy: false,
    }]);
    let figures = Block::Figures(
        vec![
            Figure {
                id: "network-latency".into(),
                value: millis(latency.internet).into(),
                unit: "ms".into(),
                caption: match tunnel {
                    Some(_) => format!("to {}, through the tunnel", latency.target),
                    None => format!("to {}", latency.target),
                }
                .into(),
                dot: None,
                color: None,
                large: true,
                inline: false,
            },
            Figure {
                id: "network-loss".into(),
                value: format!("{:.0}", latency.loss * 100.0).into(),
                unit: "%".into(),
                caption: "Loss".into(),
                dot: None,
                color: (latency.loss > 0.0).then_some(YELLOW),
                large: false,
                inline: false,
            },
            Figure {
                id: "network-jitter".into(),
                value: millis(latency.jitter).into(),
                unit: "ms".into(),
                caption: "Jitter".into(),
                dot: None,
                color: None,
                large: false,
                inline: false,
            },
            Figure {
                id: "network-lookup".into(),
                value: millis(state.dns.lookup).into(),
                unit: "ms".into(),
                caption: "DNS lookup".into(),
                dot: None,
                color: None,
                large: false,
                inline: false,
            },
        ],
        None,
    );
    // Traffic leaves by the link, through the VPN's server while one carries it, to the internet.
    let tint = if tunnel.is_some() { MAUVE } else { TEAL };
    let ms = |value: Option<f32>| match value {
        Some(_) => format!("{} ms", millis(value)),
        None => "—".to_owned(),
    };
    let dns = tunnel
        .and_then(|vpn| vpn.dns.clone())
        .or_else(|| link.and_then(|link| link.dns.first().cloned()))
        .unwrap_or_else(|| "—".to_owned());
    let hops = vec![
        Hop {
            id: "me".into(),
            glyph: p.glyph("network-computer"),
            name: state.host.clone().into(),
            address: link
                .and_then(|link| link.ipv4.clone())
                .unwrap_or_else(|| "No address".to_owned())
                .into(),
            value: link.map_or_else(String::new, |link| link.interface.clone()).into(),
            value_color: SUBTEXT1,
            color: TEAL,
            shown: true,
        },
        Hop {
            id: "router".into(),
            glyph: p.glyph("network-router"),
            name: "Router".into(),
            address: link
                .and_then(|link| link.gateway.clone())
                .unwrap_or_else(|| "—".to_owned())
                .into(),
            value: ms(latency.gateway).into(),
            value_color: latency_color(latency.gateway),
            color: TEAL,
            shown: true,
        },
        Hop {
            id: "vpn".into(),
            glyph: p.glyph("network-vpn"),
            name: tunnel.map_or_else(String::new, |vpn| vpn.name.clone()).into(),
            address: tunnel
                .and_then(Vpn::server)
                .unwrap_or_default()
                .to_owned()
                .into(),
            value: ms(latency.tunnel).into(),
            value_color: latency_color(latency.tunnel),
            color: MAUVE,
            shown: tunnel.is_some(),
        },
        Hop {
            id: "dns".into(),
            glyph: p.glyph("network-dns"),
            name: "DNS".into(),
            address: dns.into(),
            value: ms(state.dns.lookup).into(),
            value_color: latency_color(state.dns.lookup),
            color: tint,
            shown: true,
        },
        Hop {
            id: "internet".into(),
            glyph: p.glyph("network-internet"),
            name: "Internet".into(),
            address: latency.target.clone().into(),
            value: ms(latency.internet).into(),
            value_color: latency_color(latency.internet),
            color: tint,
            shown: true,
        },
    ];
    let peak = nice_peak(
        latency
            .internet_history
            .iter()
            .chain(&latency.gateway_history)
            .chain(&state.dns.history),
        LATENCY_FLOOR,
    );
    let format = |value: f32| format!("{value:.0} ms");
    card(
        title,
        span,
        vec![
            figures,
            Block::Path(Path {
                id: "network-path".into(),
                hops,
                down: link.is_none(),
            }),
            Block::Chart(ChartView {
                id: "network-latency-chart".into(),
                series: vec![
                    series(&latency.internet_history, peak, PEACH, latency.target.clone(), format),
                    series(&state.dns.history, peak, BLUE, "DNS", format),
                    series(&latency.gateway_history, peak, TEAL, "Router", format),
                ],
                height: None,
                phase: Some(state.chart_phase()),
            }),
            Block::Legend(vec![
                (Some(PEACH), "Internet".into()),
                (Some(BLUE), "DNS".into()),
                (Some(TEAL), "Router".into()),
            ]),
        ],
    )
}

// VPN

/// "just connected", "4m", "2h 14m".
fn uptime(seconds: u64) -> String {
    match seconds {
        0..60 => "just connected".to_owned(),
        60..3600 => format!("up {}m", seconds / 60),
        _ => format!("up {}", short_duration(seconds)),
    }
}

fn vpn_line(vpn: &Vpn, now: i64) -> String {
    match vpn.activity {
        Activity::Connecting => "Connecting…".to_owned(),
        Activity::On if !vpn.full && !vpn.networks.is_empty() => {
            format!("Only {}", vpn.networks.join(", "))
        }
        Activity::On => match vpn.since.map(|since| (now - since).max(0) as u64) {
            Some(seconds) if seconds < 60 => "Just connected".to_owned(),
            Some(seconds) => format!("{} · {}", vpn.kind, uptime(seconds)),
            None => vpn.kind.clone(),
        },
        Activity::Off => match vpn.server() {
            Some(server) => format!("{} · {server}", vpn.kind),
            None => vpn.kind.clone(),
        },
    }
}

fn vpn_card(state: &State, short: bool, p: &PanelCx<'_>) -> Block {
    let now = chrono::Utc::now().timestamp();
    let on = state.vpns.iter().filter(|vpn| vpn.activity != Activity::Off).count();
    let tunnel = state.tunnel();
    let kill_switch = state
        .interfaces
        .iter()
        .any(|interface| interface.role == Role::KillSwitch && interface.up);
    let pill = match (tunnel, state.link.is_some()) {
        (_, false) => None,
        (Some(_), true) if kill_switch => Some(("Kill switch", MAUVE)),
        (Some(_), true) => None,
        (None, true) => Some(("Not protected", YELLOW)),
    };
    let title = Title::with_detail("VPN", format!("{on} of {} on", state.vpns.len())).ending(
        pill.into_iter()
            .map(|(text, color)| TitleEnd::Pill {
                id: "network-vpn-state".into(),
                text: text.into(),
                color: Some(color),
                busy: false,
            })
            .collect(),
    );
    let traffic = &state.traffic;
    // The tunnel's chart takes the room the card has left under its rows and the totals, so a
    // card with few profiles is not left half empty.
    let chart_h = (CARD_BODY_H
        - state.vpns.len() as f32 * ROW_H
        - DISCLOSED_AROUND_CHART)
        .clamp(CHART_LEAST_H, CHART_MOST_H);
    let rows = state
        .vpns_in_order()
        .into_iter()
        .map(|vpn| {
            let name = vpn.name.clone();
            let switched = vpn.activity != Activity::Off;
            let carrying = tunnel.is_some_and(|tunnel| tunnel.name == vpn.name);
            let inside = vec![
                Block::Chart(ChartView {
                    id: "network-vpn-chart".into(),
                    series: vec![series(
                        &traffic.vpn_history,
                        nice_peak(&traffic.vpn_history, 64.0 * 1024.0),
                        MAUVE,
                        "Through the VPN",
                        |value| rate(value as f64),
                    )],
                    height: Some(chart_h),
                    phase: Some(state.chart_phase()),
                }),
                Block::FactsIn(
                    2,
                    vec![
                        fact("Received", bytes(traffic.vpn_rx)),
                        fact("Sent", bytes(traffic.vpn_tx)),
                    ],
                ),
            ];
            row(
                format!("network-vpn-{}", vpn.name),
                item(
                    format!("network-vpn-{}-item", vpn.name),
                    lead(p, "network-vpn", MAUVE, vpn.activity == Activity::On),
                    vpn.name.clone(),
                    line(vpn_line(vpn, now)),
                    vec![Trail::Switch {
                        on: switched,
                        color: MAUVE,
                        enabled: true,
                        on_click: p.call(move |network: &Network| {
                            network.send(Request::Vpn(name.clone(), Some(!switched)));
                        }),
                    }],
                ),
                (!short).then_some((carrying, inside)),
                None,
            )
        })
        .collect();
    let mut blocks = vec![Block::List(INDENT, rows)];
    // The pill already says it is not protected; under the list, one quiet line says which way
    // traffic goes instead.
    if !short && tunnel.is_none() && state.link.is_some() {
        let networks: Vec<String> = state
            .vpns
            .iter()
            .filter(|vpn| vpn.activity == Activity::On && !vpn.full)
            .flat_map(|vpn| vpn.networks.clone())
            .collect();
        blocks.push(Block::note(if networks.is_empty() {
            format!(
                "Traffic leaves straight from {}",
                state
                    .link
                    .as_ref()
                    .and_then(|link| link.ipv4.clone())
                    .unwrap_or_else(|| "this machine".to_owned())
            )
        } else {
            format!("Only {} goes through a VPN", networks.join(", "))
        }));
    }
    card(title, 1, blocks)
}

// Interfaces

fn interface_glyph(interface: &Interface) -> (&'static str, u32) {
    match (&interface.role, interface.kind.as_str()) {
        (Role::Tunnel(_), _) => ("network-vpn", MAUVE),
        (Role::KillSwitch, _) => ("network-lock", PINK),
        (Role::Link, "Ethernet") => ("network-wired", TEAL),
        _ => ("network-bridge", BLUE),
    }
}

/// "2.5 Gb/s", "1 Gb/s", "100 Mb/s".
fn link_speed(mbits: u32) -> String {
    if mbits >= 1000 {
        let gbits = f64::from(mbits) / 1000.0;
        if gbits.fract() == 0.0 {
            format!("{gbits:.0} Gb/s")
        } else {
            format!("{gbits:.1} Gb/s")
        }
    } else {
        format!("{mbits} Mb/s")
    }
}

fn interface_line(interface: &Interface) -> String {
    match &interface.role {
        Role::Tunnel(vpn) => vpn.clone(),
        Role::KillSwitch => "Kill switch".to_owned(),
        // Its glyph already says it is wired.
        Role::Link if interface.kind == "Ethernet" && interface.speed > 0 => {
            link_speed(interface.speed)
        }
        Role::Link if interface.kind == "Ethernet" => "Wired".to_owned(),
        Role::Link => interface.kind.clone(),
    }
}

fn interfaces_card(state: &State, p: &PanelCx<'_>, view: Option<&View>) -> Block {
    let now = chrono::Utc::now().timestamp();
    let up = state.interfaces.iter().filter(|interface| interface.up).count();
    let open = view.map_or(&Open::Natural, |view| &view.interface);
    let natural = state.link.as_ref().map(|link| link.interface.as_str());
    let rows = state
        .interfaces
        .iter()
        .map(|interface| {
            let (glyph, color) = interface_glyph(interface);
            let mut trail = Vec::new();
            // As wide as it reads, so the name beside it keeps its room; it ends against the
            // switch, so it only grows leftwards.
            if interface.up && interface.rate > 0.0 {
                trail.push(Trail::Faint(rate(interface.rate).into()));
            }
            let tunnel = matches!(interface.role, Role::Tunnel(_));
            if interface.switchable || tunnel {
                let path = interface.path.clone();
                let on = interface.up;
                trail.push(Trail::Switch {
                    on,
                    color,
                    // A VPN's own interface follows the VPN's switch.
                    enabled: interface.switchable,
                    on_click: p.call(move |network: &Network| {
                        network.send(Request::Device(path.clone(), !on));
                    }),
                });
            }
            let errors = match interface.errors {
                0 => "no errors".to_owned(),
                1 => "1 error".to_owned(),
                count => format!("{count} errors"),
            };
            // Compact, so a card of several links still has room for the one opened.
            let about = format!(
                "MTU {} · {} dropped · {errors}",
                interface.mtu, interface.drops
            );
            let trouble = interface.drops > 0 || interface.errors > 0;
            let inside = vec![
                Block::FactsIn(
                    1,
                    vec![fact(
                        match interface.lease {
                            Some(lease) => format!(
                                "Address · lease {}",
                                short_duration((lease - now).max(0) as u64)
                            ),
                            None => "Address".to_owned(),
                        },
                        interface.address.clone().unwrap_or_else(|| "—".to_owned()),
                    )],
                ),
                Block::Text(Text::new(
                    Tone::Detail(if trouble { YELLOW } else { OVERLAY2 }),
                    about,
                )),
            ];
            let name = interface.name.clone();
            let natural = natural.map(str::to_owned);
            row(
                format!("network-if-{}", interface.name),
                item(
                    format!("network-if-{}-item", interface.name),
                    lead(p, glyph, color, interface.up),
                    interface.name.clone(),
                    line(interface_line(interface)),
                    trail,
                ),
                Some((open.is(&interface.name, natural.as_deref()), inside)),
                Some(p.on_local::<Network>(move |_, view: &mut View| {
                    view.interface.toggle(&name, natural.as_deref());
                })),
            )
        })
        .collect();
    card(
        Title::with_detail("Interfaces", format!("{up} of {} up", state.interfaces.len()))
            .ending(Vec::new()),
        1,
        vec![Block::List(INDENT, rows)],
    )
}

// Wi-Fi

fn network_line(network: &AccessPoint, state: &State, in_use: bool) -> String {
    if let Some(joining) = state.joining.as_ref().filter(|joining| joining.ssid == network.ssid) {
        return joining.failed.clone().unwrap_or_else(|| "Connecting…".to_owned());
    }
    match (network.active, network.saved, network.security) {
        (true, _, _) if in_use => "In use".to_owned(),
        (true, _, _) => "Standby".to_owned(),
        (false, _, Security::Enterprise) => "Set it up in NetworkManager".to_owned(),
        (false, true, _) => format!("Saved · {}", network.band()),
        (false, false, _) => network.band().to_owned(),
    }
}

fn wifi_card(state: &State, p: &PanelCx<'_>, view: Option<&View>) -> Block {
    let Some(wifi) = &state.wifi else {
        return card(Title::with_detail("Wi-Fi", ""), 1, Vec::new());
    };
    let enabled = wifi.enabled;
    let mut end = Vec::new();
    if enabled {
        end.push(TitleEnd::Pill {
            id: "network-nearby".into(),
            text: format!("{} nearby", wifi.networks.len()).into(),
            color: None,
            busy: wifi.scanning,
        });
    }
    end.push(TitleEnd::Switch {
        id: "network-wifi-switch".into(),
        on: enabled,
        color: BLUE,
        enabled: true,
        on_click: p.call(move |network: &Network| network.send(Request::Wifi(Some(!enabled)))),
    });
    let title = Title::with_detail("Wi-Fi", "").ending(end);
    if !enabled {
        return card(
            title,
            1,
            vec![Block::Hero(
                "Wi-Fi is off".into(),
                vec!["Saved networks rejoin when it is on".into()],
            )],
        );
    }
    let in_use = matches!(
        state.link.as_ref().map(|link| link.kind),
        Some(super::state::LinkKind::Wifi)
    );
    let open = view.map_or(&Open::Natural, |view| &view.network);
    let asked = view.and_then(|view| view.prompt.as_ref());
    let menu_open = view.is_some_and(|view| view.menu);
    let connected = wifi.active().map(|network| network.ssid.clone());
    let rows = wifi
        .networks
        .iter()
        .take(NETWORKS)
        .map(|network| {
            let ssid = network.ssid.clone();
            let id = format!("network-wifi-{ssid}");
            let line = line(network_line(network, state, in_use));
            let lead = lead(p, glyphs::signal(network.strength), BLUE, network.active);
            if network.active {
                let signal = &state.signal;
                let mut trail = Vec::new();
                if let Some(dbm) = signal.dbm {
                    trail.push(Trail::Steady(format!("{dbm} dBm").into(), vec!["-100 dBm".into()]));
                }
                let mut actions = vec![Action {
                    id: "network-disconnect".into(),
                    label: "Disconnect".into(),
                    glyph: p.glyph("network-disconnect"),
                    switch: None,
                    danger: false,
                    on_click: p.on_local::<Network>(|network, view: &mut View| {
                        view.menu = false;
                        network.send(Request::Disconnect);
                    }),
                }];
                if network.saved {
                    let forget = ssid.clone();
                    actions.push(Action {
                        id: "network-forget".into(),
                        label: "Forget this network".into(),
                        glyph: p.glyph("network-forget"),
                        switch: None,
                        danger: true,
                        on_click: p.on_local::<Network>(move |network, view: &mut View| {
                            view.menu = false;
                            network.send(Request::Forget(forget.clone()));
                        }),
                    });
                }
                trail.push(Trail::Menu(Actions {
                    id: "network-wifi-menu".into(),
                    glyph: p.glyph("network-more"),
                    label: format!("More for {ssid}").into(),
                    actions,
                    open: menu_open,
                    on_toggle: p.on_local::<Network>(|_, view: &mut View| view.menu = !view.menu),
                }));
                let history: Vec<f32> = signal
                    .history
                    .iter()
                    .map(|dbm| (dbm + 100.0).clamp(0.0, 70.0))
                    .collect();
                let mine = network.channel();
                let shared = wifi
                    .networks
                    .iter()
                    .filter(|other| other.ssid != network.ssid && other.channel() == mine)
                    .count();
                let inside = vec![
                    Block::Chart(ChartView {
                        id: "network-signal-chart".into(),
                        series: vec![series(&history, 70.0, BLUE, "Signal", |value| {
                            format!("{:.0} dBm", value - 100.0)
                        })],
                        height: Some(40.0),
                        phase: Some(state.chart_phase()),
                    }),
                    Block::FactsIn(
                        2,
                        vec![
                            fact(
                                format!("{} · {}", network.security.label(), network.band()),
                                format!("{} Mb/s", wifi.bitrate),
                            ),
                            (
                                format!("Channel {mine}").into(),
                                if shared == 0 {
                                    "Clear".to_owned()
                                } else {
                                    format!("Shared by {shared}")
                                }
                                .into(),
                                Some(if shared == 0 { GREEN } else { YELLOW }),
                            ),
                        ],
                    ),
                ];
                let toggle = ssid.clone();
                let natural = connected.clone();
                return row(
                    id.clone(),
                    item(format!("{id}-item"), lead, network.ssid.clone(), line, trail),
                    Some((open.is(&ssid, connected.as_deref()), inside)),
                    Some(p.on_local::<Network>(move |_, view: &mut View| {
                        view.network.toggle(&toggle, natural.as_deref());
                    })),
                );
            }
            let mut trail = Vec::new();
            let secured = network.security != Security::Open;
            if secured && let Some(glyph) = p.glyph("network-lock") {
                trail.push(Trail::Glyph(glyph, OVERLAY2));
            }
            let busy = state.joining.as_ref().is_some_and(|joining| joining.ssid == ssid && joining.failed.is_none());
            let on_click = if busy || network.security == Security::Enterprise {
                None
            } else if network.saved || !secured {
                Some(p.call(move |network: &Network| network.send(Request::Connect(ssid.clone()))))
            } else {
                Some(p.on_local::<Network>(move |_, view: &mut View| view.ask(&ssid)))
            };
            // A network that needs a password opens to its field.
            let asking = asked.filter(|prompt| prompt.ssid == network.ssid);
            let opens = (secured && !network.saved).then(|| {
                let field = asking.map_or_else(
                    || InputView {
                        id: format!("{id}-password").into(),
                        text: SharedString::default(),
                        placeholder: "Password".into(),
                        max_lines: 1,
                        send_glyph: p.glyph("network-join"),
                        on_send: None,
                        busy: false,
                        secret: true,
                    },
                    |prompt| InputView {
                        id: format!("{id}-password").into(),
                        text: prompt.text.clone().into(),
                        placeholder: "Password".into(),
                        max_lines: 1,
                        send_glyph: p.glyph("network-join"),
                        on_send: (!prompt.text.is_empty() && !busy).then(|| {
                            p.on_local::<Network>(|network, view: &mut View| view.submit(network))
                        }),
                        busy,
                        secret: !prompt.shown,
                    },
                );
                (asking.is_some(), vec![Block::Input(field)])
            });
            row(
                id.clone(),
                item(format!("{id}-item"), lead, network.ssid.clone(), line, trail),
                opens,
                on_click,
            )
        })
        .collect();
    card(title, 1, vec![Block::List(INDENT, rows)])
}

// Traffic

fn traffic_card(state: &State, span: usize) -> Block {
    let traffic = &state.traffic;
    let usage = &state.usage;
    let link = state.link.as_ref().map_or_else(String::new, |link| link.interface.clone());
    // Which way it goes, which holds still; a share of two noisy rates only jitters, and with a
    // full tunnel everything goes through it anyway.
    let detail = match state.tunnel() {
        _ if link.is_empty() => link.clone(),
        Some(vpn) => format!("{link} · through {}", vpn.name),
        None => format!("{link} · direct"),
    };
    let (down, down_unit) = rate_parts(traffic.rx_rate);
    let (up, up_unit) = rate_parts(traffic.tx_rate);
    let week: u64 = usage.days.iter().map(|(_, used)| *used).sum();
    let days = usage.days.len();
    let bars = Block::Bars(Bars {
        id: "network-week".into(),
        values: usage
            .days
            .iter()
            .map(|(day, used)| {
                let label = if day == "Today" {
                    chrono::Local::now().format("%a").to_string()
                } else {
                    day.clone()
                };
                (label.chars().take(1).collect::<String>().into(), *used as f32)
            })
            .collect(),
        picked: days.checked_sub(1),
        color: BLUE,
        picked_color: TEAL,
    });
    let peak = nice_peak(traffic.rx_history.iter().chain(&traffic.tx_history), 64.0 * 1024.0);
    card(
        Title::with_detail("Traffic", detail).ending(Vec::new()),
        span,
        vec![
            Block::Figures(
                vec![
                    Figure {
                        id: "network-down".into(),
                        value: down.into(),
                        unit: down_unit.into(),
                        caption: "Download".into(),
                        dot: Some(TEAL),
                        color: None,
                        large: true,
                        inline: false,
                    },
                    Figure {
                        id: "network-up".into(),
                        value: up.into(),
                        unit: up_unit.into(),
                        caption: "Upload".into(),
                        dot: Some(PEACH),
                        color: None,
                        large: true,
                        inline: false,
                    },
                    Figure {
                        id: "network-today".into(),
                        value: bytes(usage.today_rx + usage.today_tx).into(),
                        unit: SharedString::default(),
                        caption: "today".into(),
                        dot: None,
                        color: None,
                        large: false,
                        inline: true,
                    },
                    Figure {
                        id: "network-week-total".into(),
                        value: bytes(week).into(),
                        unit: SharedString::default(),
                        caption: "this week".into(),
                        dot: None,
                        color: None,
                        large: false,
                        inline: true,
                    },
                ],
                Some(Box::new(bars)),
            ),
            Block::Chart(ChartView {
                id: "network-traffic-chart".into(),
                series: vec![
                    series(&traffic.rx_history, peak, TEAL, "Download", |value| rate(value as f64)),
                    series(&traffic.tx_history, peak, PEACH, "Upload", |value| rate(value as f64)),
                ],
                height: None,
                phase: Some(state.chart_phase()),
            }),
        ],
    )
}

// Activity

/// A two-line entry: what it is, over `under`, and its value faint at the end.
fn entry(id: String, title: impl Into<SharedString>, under: Block, value: impl Into<SharedString>) -> Block {
    Block::Frame(
        FrameView {
            id: id.clone().into(),
            look: Look::Flat,
            on_click: None,
            on_dismiss: None,
            countdown: None,
            selected: false,
            motion: Motion::Still,
            shown_height: None,
        },
        vec![Block::Item(
            Item {
                id: format!("{id}-item").into(),
                lead: None,
                overline: None,
                title: title.into(),
                title_color: None,
                wrap: false,
                detail: SharedString::default(),
                trail: vec![Trail::Faint(value.into())],
            },
            vec![under],
        )],
    )
}

fn activity_card(state: &State, span: usize, p: &PanelCx<'_>, view: Option<&View>) -> Block {
    let selected = view.map_or(0, |view| view.activity).min(2);
    // A narrow card gives a row to its views, so it shows one entry fewer.
    let entries = if span == 1 { ENTRIES - 1 } else { ENTRIES };
    // Each app's bar against the busiest one's.
    let busiest = state.sockets.apps.first().map_or(1, |(_, count)| *count).max(1) as f32;
    let rows: Vec<Block> = match selected {
        0 => state
            .sockets
            .apps
            .iter()
            .take(entries)
            .map(|(app, count)| {
                let id = format!("network-app-{app}");
                entry(
                    id.clone(),
                    app.clone(),
                    Block::Level(format!("{id}-share").into(), *count as f32 / busiest, PINK),
                    format!("{count} open"),
                )
            })
            .collect(),
        1 => state
            .ports
            .iter()
            .take(entries)
            .map(|port| {
                entry(
                    format!("network-port-{}-{}", port.protocol, port.port),
                    port.app.clone(),
                    line(port.protocol.to_uppercase()),
                    port.port.to_string(),
                )
            })
            .collect(),
        _ => state
            .routes
            .iter()
            .take(entries)
            .enumerate()
            .map(|(index, route)| {
                let to = if route.destination == "default" {
                    "Everything".to_owned()
                } else {
                    route.destination.clone()
                };
                let how = match &route.via {
                    Some(via) => format!("Via {via}"),
                    None => "On this link".to_owned(),
                };
                entry(format!("network-route-{index}"), to, line(how), route.interface.clone())
            })
            .collect(),
    };
    let rows = if rows.is_empty() {
        vec![Block::note(match selected {
            0 => "Nothing is connected",
            1 => "Nothing is listening",
            _ => "No routes",
        })]
    } else {
        vec![Block::List(8.0, rows)]
    };
    let labels: Vec<TabLabel> = ["Apps", "Ports", "Routes"]
        .into_iter()
        .map(|label| TabLabel {
            label: label.into(),
            glyph: None,
        })
        .collect();
    let on_select = p.on_local_with::<Network, usize>(|_, view: &mut View, index| {
        view.activity = index;
    });
    // A one-column card has no room for its views beside its heading, so they take a row of
    // their own across it.
    if span == 1 {
        let mut blocks = vec![Block::Segmented(SegmentedView {
            id: "network-activity".into(),
            labels,
            selected,
            on_select,
        })];
        blocks.extend(rows);
        return card(Title::with_detail("Activity", "").ending(Vec::new()), span, blocks);
    }
    card(
        Title::with_detail("Activity", "").ending(vec![TitleEnd::Tabs {
            id: "network-activity".into(),
            labels,
            selected,
            on_select,
        }]),
        span,
        rows,
    )
}

pub fn panel(state: &State, p: &PanelCx<'_>) -> PanelView {
    let view = p.local::<View>();
    let rows = layout(state.wifi.is_some(), !state.vpns.is_empty());
    let band = |row: &Vec<(Card, usize)>, height: f32| {
        Block::Band(
            height,
            row.iter()
                .map(|(card, span)| match card {
                    Card::Internet => internet_card(state, *span, p),
                    Card::Wifi => wifi_card(state, p, view),
                    Card::Vpn => vpn_card(state, false, p),
                    Card::VpnShort => vpn_card(state, true, p),
                    Card::Interfaces => interfaces_card(state, p, view),
                    Card::Traffic => traffic_card(state, *span),
                    Card::Activity => activity_card(state, *span, p, view),
                })
                .collect(),
        )
    };
    let [top, bottom] = &rows;
    PanelView {
        header: Header::Tabs,
        blocks: vec![band(top, TOP_H), band(bottom, BOTTOM_H)],
    }
}
