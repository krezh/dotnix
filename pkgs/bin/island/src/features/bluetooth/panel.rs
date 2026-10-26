//! The Bluetooth tab: icon buttons, every connected device side by side, then what is paired and
//! what is nearby, with a card over the panel while a device asks about a pairing code.

use std::time::Duration;

use gpui::SharedString;

use super::{
    Bluetooth, glyphs,
    runtime::Request,
    state::{Busy, Device, State},
    view::View,
};
use crate::{
    components::card::{self, Look, Motion, Title},
    features::PanelCx,
    panel::{
        Action, Actions, Block, ButtonRow, FrameView, Header, IconButton, Item, Lead, Mark,
        PanelView, PromptView, Trail,
    },
    panels::short_duration,
    theme::{BLUE, GREEN, OVERLAY2, SUBTEXT1},
};

const LEAD: f32 = 56.0;

fn frame(id: String, look: Look, on_click: Option<crate::panel::Handler>) -> FrameView {
    FrameView {
        id: id.into(),
        look,
        on_click,
        on_dismiss: None,
        countdown: None,
        selected: false,
        motion: Motion::Appear,
        shown_height: None,
    }
}

fn what(device: &Device) -> String {
    let use_for = device
        .codec
        .clone()
        .unwrap_or_else(|| device.service.clone());
    if use_for.is_empty() {
        device.kind().to_owned()
    } else {
        format!("{} · {use_for}", device.kind())
    }
}

/// One connected device: its battery ring, what it is, how long it has been connected, and its
/// own way to disconnect, with trust and forget behind its menu.
fn connected(device: &Device, open: bool, now: i64, subsec: Duration, p: &PanelCx<'_>) -> Block {
    let path = device.path.clone();
    let mark = match device.battery {
        Some(level) => Mark::Ring {
            key: format!("bt-ring-{path}").into(),
            fraction: f32::from(level) / 100.0,
            color: glyphs::battery(Some(level)),
            value: format!("{level}%").into(),
        },
        None => Mark::Glyph(
            p.glyph(glyphs::device(&device.icon)).unwrap_or_default(),
            BLUE,
        ),
    };
    let status = match (device.busy, device.since) {
        (Some(busy), _) => (busy.label().to_owned(), BLUE),
        (None, Some(since)) => {
            let elapsed = (now - since).max(0) as u64;
            let step = if elapsed < 86_400 { 60 } else { 3600 };
            p.redraw_in(Duration::from_secs(step - (elapsed % step) - 1) + subsec);
            (format!("Connected · {}", short_duration(elapsed)), GREEN)
        }
        (None, None) => ("Connected".to_owned(), GREEN),
    };
    let item = Item {
        id: format!("bt-item-{path}").into(),
        lead: Some(Lead {
            mark: Some(mark),
            tint: device.battery.is_none().then_some(BLUE),
            glow: None,
            faded: false,
            size: Some(LEAD),
            on_click: None,
            top: false,
        }),
        overline: None,
        title: device.name.clone().into(),
        title_color: None,
        wrap: false,
        detail: SharedString::default(),
        trail: Vec::new(),
    };
    let (trust_path, forget_path, menu_path, off_path) =
        (path.clone(), path.clone(), path.clone(), path.clone());
    let trusted = device.trusted;
    let menu = Actions {
        id: format!("bt-menu-{path}").into(),
        glyph: p.glyph("bt-more"),
        label: format!("More for {}", device.name).into(),
        actions: vec![
            Action {
                id: format!("bt-trust-{path}").into(),
                label: "Trusted".into(),
                glyph: p.glyph("bt-trust"),
                switch: Some(trusted),
                danger: false,
                on_click: p.call(move |bluetooth: &Bluetooth| {
                    bluetooth.send(Request::Trust(trust_path.clone(), !trusted));
                }),
            },
            Action {
                id: format!("bt-forget-{path}").into(),
                label: "Forget".into(),
                glyph: p.glyph("bt-forget"),
                switch: None,
                danger: true,
                on_click: p.on_local::<Bluetooth>(move |bluetooth, view: &mut View| {
                    view.menu = None;
                    bluetooth.send(Request::Forget(forget_path.clone()));
                }),
            },
        ],
        open,
        on_toggle: p.on_local::<Bluetooth>(move |_, view: &mut View| {
            view.menu = if view.menu.as_deref() == Some(menu_path.as_str()) {
                None
            } else {
                Some(menu_path.clone())
            };
        }),
    };
    Block::Frame(
        frame(
            format!("bt-conn-{path}"),
            Look::Raised { urgent: false },
            None,
        ),
        vec![
            Block::Item(
                item,
                vec![
                    Block::caption(what(device), OVERLAY2),
                    Block::caption(status.0, status.1),
                ],
            ),
            Block::Buttons(ButtonRow {
                buttons: vec![IconButton {
                    id: format!("bt-disconnect-{path}").into(),
                    glyph: None,
                    rising: None,
                    label: "Disconnect".into(),
                    active: false,
                    badge: None,
                    on_click: p.call(move |bluetooth: &Bluetooth| {
                        bluetooth.send(Request::Disconnect(off_path.clone()));
                    }),
                }],
                compact: true,
                indent: 0.0,
                menu: Some(menu),
            }),
        ],
    )
}

/// A row in Paired or Nearby: what the device is, and what clicking it does.
fn row(device: &Device, detail: String, does: &str, request: Request, p: &PanelCx<'_>) -> Block {
    let busy = device.busy.map(Busy::label);
    let path = device.path.clone();
    let on_click = busy
        .is_none()
        .then(|| p.call(move |bluetooth: &Bluetooth| bluetooth.send(request.clone())));
    let item = Item {
        id: format!("bt-row-{path}").into(),
        lead: Some(Lead {
            mark: p
                .glyph(glyphs::device(&device.icon))
                .map(|glyph| Mark::Glyph(glyph, SUBTEXT1)),
            tint: None,
            glow: None,
            faded: false,
            size: None,
            on_click: None,
            top: false,
        }),
        overline: None,
        title: device.name.clone().into(),
        title_color: None,
        wrap: false,
        detail: SharedString::default(),
        trail: vec![Trail::Text(busy.unwrap_or(does).into(), BLUE)],
    };
    Block::Frame(
        frame(format!("bt-row-{path}"), Look::Inset, on_click),
        vec![Block::Item(item, vec![Block::caption(detail, OVERLAY2)])],
    )
}

fn paired(state: &State, p: &PanelCx<'_>) -> Block {
    let rows: Vec<Block> = state
        .paired_idle()
        .map(|device| {
            let detail = if device.trusted {
                format!("{} · trusted", device.kind())
            } else {
                device.kind().to_owned()
            };
            row(
                device,
                detail,
                "Connect",
                Request::Connect(device.path.clone()),
                p,
            )
        })
        .collect();
    let any_paired = state.devices.iter().any(|device| device.paired);
    let detail = if rows.is_empty() { "" } else { "not connected" };
    let blocks = if rows.is_empty() {
        vec![Block::note(if any_paired {
            "Everything paired is connected"
        } else {
            "Nothing paired yet"
        })]
    } else {
        rows
    };
    Block::Card(
        Some(Title::with_detail("Paired", detail)),
        card::Size::Medium,
        blocks,
    )
}

fn nearby(state: &State, p: &PanelCx<'_>) -> Block {
    let rows: Vec<Block> = state
        .nearby()
        .map(|device| {
            let detail = match device.rssi {
                Some(rssi) => format!("{} · {rssi} dBm", device.kind()),
                None => device.kind().to_owned(),
            };
            row(
                device,
                detail,
                "Pair",
                Request::Pair(device.path.clone()),
                p,
            )
        })
        .collect();
    let detail = if state.discovering {
        "Scanning…".to_owned()
    } else if rows.is_empty() {
        String::new()
    } else {
        format!("{} found", rows.len())
    };
    let blocks = if rows.is_empty() {
        vec![Block::note(if state.discovering {
            "Looking for devices…"
        } else {
            "Scan to find devices ready to pair"
        })]
    } else {
        rows
    };
    Block::Card(
        Some(Title::with_detail("Nearby", detail)),
        card::Size::Medium,
        blocks,
    )
}

fn buttons(state: &State, now: i64, subsec: Duration, p: &PanelCx<'_>) -> Vec<IconButton> {
    let mut buttons = vec![IconButton {
        id: "bt-power".into(),
        glyph: p.glyph(if state.powered { "bt-on" } else { "bt-off" }),
        rising: None,
        label: if state.powered {
            "Turn Bluetooth off"
        } else {
            "Turn Bluetooth on"
        }
        .into(),
        active: state.powered,
        badge: None,
        on_click: p.call(|bluetooth: &Bluetooth| bluetooth.send(Request::Power(None))),
    }];
    if !state.powered {
        return buttons;
    }
    let nearby = state.nearby().count();
    buttons.push(IconButton {
        id: "bt-scan".into(),
        glyph: p.glyph("bt-scan"),
        rising: None,
        label: if state.discovering {
            "Stop scanning"
        } else {
            "Scan for devices"
        }
        .into(),
        active: state.discovering,
        badge: (nearby > 0).then(|| nearby.to_string().into()),
        on_click: p.call(|bluetooth: &Bluetooth| bluetooth.send(Request::Scan)),
    });
    let left = state.visible_until.map(|until| (until - now).max(0));
    if state.discoverable && left.is_some_and(|left| left > 0) {
        p.redraw_in(subsec);
    }
    buttons.push(IconButton {
        id: "bt-visible".into(),
        glyph: p.glyph("bt-visible"),
        rising: None,
        label: if state.discoverable {
            format!("Visible as {}", state.name)
        } else {
            format!("Let other devices find {}", state.name)
        }
        .into(),
        active: state.discoverable,
        badge: state
            .discoverable
            .then(|| left.map(|left| format!("{}:{:02}", left / 60, left % 60).into()))
            .flatten(),
        on_click: p.call(|bluetooth: &Bluetooth| bluetooth.send(Request::Visible)),
    });
    buttons
}

fn prompt(state: &State, p: &PanelCx<'_>) -> Option<Block> {
    let asking = state.asking.as_ref()?;
    let id = asking.id;
    Some(Block::Prompt(PromptView {
        id: "bt-pair".into(),
        title: format!("Pair {}", asking.device).into(),
        detail: if asking.confirm {
            format!(
                "Check that {} shows the same code, then confirm.",
                asking.device
            )
        } else {
            format!("Type this code on {} and press Enter on it.", asking.device)
        }
        .into(),
        field: None,
        reveal: None,
        code: Some(asking.code.clone().into()),
        confirm: asking.confirm.then(|| {
            (
                SharedString::from("Confirm"),
                p.call(move |bluetooth: &Bluetooth| bluetooth.send(Request::Answer(id, true))),
            )
        }),
        error: None,
        on_cancel: p.call(move |bluetooth: &Bluetooth| bluetooth.send(Request::Answer(id, false))),
    }))
}

pub fn panel(state: &State, p: &PanelCx<'_>) -> PanelView {
    let utc = chrono::Utc::now();
    let now = utc.timestamp();
    let subsec = Duration::from_millis(1000 - u64::from(utc.timestamp_subsec_millis()).min(999));
    let mut blocks = Vec::new();
    if !state.available {
        blocks.push(Block::Card(
            None,
            card::Size::Wide,
            vec![Block::note("No Bluetooth adapter, or BlueZ is not running")],
        ));
        return PanelView {
            header: Header::Tabs,
            blocks,
        };
    }
    blocks.push(Block::Buttons(ButtonRow {
        buttons: buttons(state, now, subsec, p),
        compact: false,
        indent: 0.0,
        menu: None,
    }));
    if !state.powered {
        let paired = state.devices.iter().filter(|device| device.paired).count();
        blocks.push(Block::Card(
            None,
            card::Size::Wide,
            vec![Block::note(format!(
                "Bluetooth is off. {paired} paired device{} will reconnect when it is on.",
                if paired == 1 { "" } else { "s" }
            ))],
        ));
    } else {
        let menu = p.local::<View>().and_then(|view| view.menu.clone());
        let cards: Vec<Block> = state
            .connected()
            .map(|device| {
                connected(
                    device,
                    menu.as_deref() == Some(device.path.as_str()),
                    now,
                    subsec,
                    p,
                )
            })
            .collect();
        // Up to three a row, never scrolling sideways; four make two rows of two.
        let columns = if cards.len() == 4 {
            2
        } else {
            cards.len().min(3)
        };
        blocks.push(if cards.is_empty() {
            let next = if state.devices.iter().any(|device| device.paired) {
                "Nothing connected. Click a paired device below to connect it."
            } else {
                "Nothing connected. Scan, then pick a device nearby to pair it."
            };
            Block::Card(
                Some("Connected".into()),
                card::Size::Wide,
                vec![Block::note(next)],
            )
        } else {
            Block::Grid(columns, cards)
        });
        blocks.push(Block::Row(vec![paired(state, p), nearby(state, p)]));
    }
    blocks.extend(prompt(state, p));
    PanelView {
        header: Header::Tabs,
        blocks,
    }
}
