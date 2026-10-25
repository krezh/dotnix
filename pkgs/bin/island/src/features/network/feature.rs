//! How island sees the network: the bar widget, the Network tab and its password card, and
//! `island ctl wifi` and `island ctl vpn`.

use std::any::Any;

use gpui::{AnyElement, Keystroke};

use super::{
    Network, osd, panel,
    runtime::{Request, Settings},
    state::{Activity, State},
    view::View,
    widget,
};
use crate::{
    components::input::{self, Edit},
    control::{self, Command, Group, Report, Row, SWITCH, Shown, Takes},
    features::{
        Feature, Feed, GlyphSpec, IslandInfo, Outcome, PanelCx, WidgetCx,
        osd::{Osd, shown_popup as osd_popup},
        parse_settings,
    },
    panel::PanelView,
};

impl Feature for Network {
    const NAME: &'static str = "network";
    const PANEL: Option<&'static str> = Some("network");
    const TAB: Option<(&'static str, u8)> = Some(("Network", 5));
    const GLYPHS: &'static [GlyphSpec] = &[
        ("tab-network", &["network-workgroup-symbolic", "network-wired-symbolic"]),
        ("network-wifi-excellent", &["network-wireless-signal-excellent-symbolic"]),
        ("network-wifi-good", &["network-wireless-signal-good-symbolic"]),
        ("network-wifi-ok", &["network-wireless-signal-ok-symbolic"]),
        ("network-wifi-weak", &["network-wireless-signal-weak-symbolic"]),
        ("network-wifi-none", &["network-wireless-signal-none-symbolic"]),
        ("network-wifi-acquiring", &["network-wireless-acquiring-symbolic"]),
        (
            "network-wifi-off",
            &["network-wireless-disabled-symbolic", "network-wireless-offline-symbolic"],
        ),
        ("network-offline", &["network-wireless-offline-symbolic", "network-offline-symbolic"]),
        ("network-wired", &["network-wired-symbolic"]),
        ("network-wired-no-route", &["network-wired-no-route-symbolic", "network-error-symbolic"]),
        (
            "network-wifi-no-route",
            &["network-wireless-no-route-symbolic", "network-error-symbolic"],
        ),
        ("network-wired-off", &["network-wired-disconnected-symbolic", "network-wired-symbolic"]),
        ("network-vpn", &["network-vpn-symbolic"]),
        ("network-scan", &["view-refresh-symbolic"]),
        ("network-disconnect", &["network-offline-symbolic", "process-stop-symbolic"]),
        ("network-forget", &["edit-delete-symbolic", "user-trash-symbolic"]),
        ("network-reveal", &["view-reveal-symbolic", "view-visible-symbolic"]),
        ("network-conceal", &["view-conceal-symbolic", "view-hidden-symbolic"]),
        ("network-join", &["go-next-symbolic", "go-up-symbolic"]),
    ];
    const CONTROLS: &'static [Command] = &[
        Command {
            name: "wifi",
            about: "Turn Wi-Fi on or off",
            group: Group::System,
            takes: Takes::Forms {
                forms: SWITCH,
                bare: Some("Show the Wi-Fi network"),
            },
            examples: &["island ctl wifi off"],
        },
        Command {
            name: "vpn",
            about: "Turn a VPN on or off by its name",
            group: Group::System,
            takes: Takes::Word("name"),
            examples: &["island ctl vpn \"Home WireGuard\""],
        },
    ];
    const KEYBOARD: bool = true;

    type State = State;
    type Local = View;
    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        Network::start(feed)
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        parse_settings::<Settings>(settings).map(drop)
    }

    fn sync_local(state: &State, view: &mut View, island: &IslandInfo<'_>, _animate: bool) {
        view.sync(state);
        view.shown.sync(island);
    }

    fn osd(before: &State, after: &State) -> Option<Osd> {
        osd::osd(before, after)
    }

    fn popup<'a>(&self, _: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        osd_popup(&p.local::<View>()?.shown, p)
    }

    fn elapse(&self, view: &mut View, _hovered: bool) {
        view.shown.expire();
    }

    fn widget(&self, _: &str, state: &State, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        Some(widget::widget(state, w))
    }

    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        Some(Box::new(self.watch()))
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        Some(panel::panel(state, p))
    }

    fn command(&self, state: &State, args: &[&str]) -> Report {
        match args.first().copied() {
            Some("wifi") => {
                let Some(wifi) = &state.wifi else {
                    return Report::did(Row::said("No Wi-Fi device"));
                };
                if args.len() == 1 {
                    return Report::shows(vec![wifi_row(state)]);
                }
                let Ok(switch) = control::switch(&args[1..]) else {
                    return Report::default();
                };
                let on = switch.apply(wifi.enabled);
                self.send(Request::Wifi(Some(on)));
                Report::did(Row::new("Wi-Fi", Shown::Switch(on)))
            }
            Some("vpn") => {
                let name = args[1..].join(" ");
                let Some(vpn) = state.vpns.iter().find(|vpn| vpn.name == name) else {
                    let known: Vec<&str> = state.vpns.iter().map(|vpn| vpn.name.as_str()).collect();
                    return Report::did(Row::said(if known.is_empty() {
                        "No VPNs are set up in NetworkManager".to_owned()
                    } else {
                        format!("No VPN named {name}; there is {}", known.join(", "))
                    }));
                };
                let on = vpn.activity == Activity::Off;
                self.send(Request::Vpn(name.clone(), Some(on)));
                Report::did(Row::new(name, Shown::Switch(on)))
            }
            _ => Report::default(),
        }
    }

    fn status(&self, state: &State) -> Vec<Row> {
        let mut rows = Vec::new();
        if state.wifi.is_some() {
            rows.push(wifi_row(state));
        } else if let Some(wired) = &state.wired {
            rows.push(Row::new(
                "Ethernet",
                Shown::Text(if wired.active {
                    wired.name.clone()
                } else {
                    "not connected".to_owned()
                }),
            ));
        }
        if let Some(vpn) = state.vpns.iter().find(|vpn| vpn.activity == Activity::On) {
            rows.push(Row::new("VPN", Shown::Text(vpn.name.clone())));
        }
        rows
    }

    fn key(&self, _: &State, view: &mut View, keystroke: &Keystroke) -> Outcome {
        let Some(prompt) = &mut view.prompt else {
            return if keystroke.key == "escape" {
                Outcome::Close
            } else {
                Outcome::Open
            };
        };
        match input::edit(&mut prompt.text, keystroke) {
            Edit::Cancel => view.prompt = None,
            Edit::Send => view.submit(self),
            Edit::Changed | Edit::Ignored => {}
        }
        Outcome::Open
    }
}

fn wifi_row(state: &State) -> Row {
    let Some(wifi) = &state.wifi else {
        return Row::new("Wi-Fi", Shown::Unknown);
    };
    if !wifi.enabled {
        return Row::new("Wi-Fi", Shown::Switch(false));
    }
    match wifi.active() {
        Some(active) => Row::new("Wi-Fi", Shown::Text(active.ssid.clone()))
            .detail(format!("{}%", active.strength)),
        None => Row::new("Wi-Fi", Shown::Text("not connected".to_owned())),
    }
}
