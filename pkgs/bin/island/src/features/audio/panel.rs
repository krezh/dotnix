//! The Audio tab: the headset, output and input side by side, then the apps playing.

use std::collections::HashMap;

use gpui::SharedString;

use super::{
    Audio, glyphs,
    headset_card::headset_card,
    meters::{Peak, Probe},
    request::Request,
    state::{App, AudioState, Target},
};
use crate::{
    components::card::{self, Look, Title},
    features::{PanelCx, headset},
    panel::{
        ActionTile, Block, Choice, Dropdown, FrameView, Header, Item, Lead, Mark, PanelView,
        Slider, Trail,
    },
    theme::{ACCENT, BLUE, CRITICAL, GREEN, MAUVE, OVERLAY2, RED, app_color},
};

/// Above this the microphone is taken to hear speech, rather than room noise.
const SPEAKING: f32 = 0.35;
/// About -2 dBFS: a peak this high is close to clipping.
const CLIPPING: f32 = 0.97;

pub fn amount(volume: f32, muted: bool) -> String {
    if muted {
        "Muted".to_owned()
    } else {
        format!("{}%", (volume * 100.0).round() as i32)
    }
}

fn hint(peak: Peak, muted: bool) -> (SharedString, u32) {
    match peak {
        _ if muted => ("Muted".into(), OVERLAY2),
        peak if peak.hold >= CLIPPING => ("Clipping, lower the gain".into(), RED),
        peak if peak.level >= SPEAKING => ("Speaking".into(), GREEN),
        _ => ("Silent".into(), OVERLAY2),
    }
}

/// The default device's volume, with what it hears under it.
fn volume(
    state: &AudioState,
    target: Target,
    peaks: &HashMap<Probe, Peak>,
    p: &PanelCx<'_>,
) -> Option<Block> {
    let device = state.default_device(target)?;
    let channel = state.channel(target);
    let (id, glyph, color, probe) = match target {
        Target::Output => (
            "level-output",
            glyphs::speaker(channel.volume, channel.muted),
            BLUE,
            Probe::Output,
        ),
        Target::Input => (
            "level-input",
            glyphs::microphone(channel.muted),
            MAUVE,
            Probe::Input,
        ),
    };
    let peak = if channel.muted {
        Peak::default()
    } else {
        peaks.get(&probe).copied().unwrap_or_default()
    };
    let tint = if channel.muted { OVERLAY2 } else { color };
    let mut under = vec![
        Block::Slider(Slider {
            id: id.into(),
            value: if channel.muted { 0.0 } else { channel.volume },
            color,
            signal: None,
            on_change: Some(
                p.slider(move |audio: &Audio, value| audio.send(Request::SetVolume(target, value))),
            ),
        }),
        Block::Segments(peak.level, peak.hold),
    ];
    if target == Target::Input {
        let (text, color) = hint(peak, channel.muted);
        under.push(Block::caption(text, color));
    }
    let item = Item {
        id: format!("{id}-item").into(),
        lead: Some(Lead {
            mark: p.glyph(glyph).map(|path| Mark::Glyph(path, tint)),
            tint: Some(tint),
            glow: (target == Target::Input).then_some((color, peak.level)),
            faded: false,
            size: None,
            on_click: Some(p.call(move |audio: &Audio| audio.send(Request::ToggleMute(target)))),
            top: false,
        }),
        overline: None,
        title: device.description.clone().into(),
        title_color: None,
        wrap: false,
        detail: SharedString::default(),
        trail: vec![Trail::Text(
            amount(channel.volume, channel.muted).into(),
            tint,
        )],
    };
    Some(Block::Item(item, under))
}

fn devices(state: &AudioState, target: Target, p: &PanelCx<'_>) -> Vec<Block> {
    let devices = state.devices(target);
    if devices.len() < 2 {
        return Vec::new();
    }
    let current = state
        .default_device(target)
        .map(|device| device.name.clone());
    let headset = headset::latest().map(|battery| battery.name);
    let color = match target {
        Target::Output => BLUE,
        Target::Input => MAUVE,
    };
    let tiles = devices
        .iter()
        .enumerate()
        .map(|(index, device)| {
            let name = device.name.clone();
            ActionTile {
                id: format!("device-{target:?}-{index}").into(),
                glyph: p.glyph(glyphs::device(device, headset.as_deref())),
                label: device.description.clone().into(),
                color,
                armed: current.as_deref() == Some(device.name.as_str()),
                on_click: p.call(move |audio: &Audio| {
                    audio.send(Request::SetDefault(target, name.clone()))
                }),
            }
        })
        .collect();
    vec![Block::heading("Devices"), Block::Tiles(tiles)]
}

fn app_mark(app: &App) -> Mark {
    match &app.icon {
        Some(path) => Mark::Image(path.clone()),
        None => Mark::Letter(app.name.clone().into(), app_color(&app.name)),
    }
}

/// Apps recording from a microphone, each named once.
fn recording(state: &AudioState) -> Vec<Block> {
    let mut named: Vec<&str> = Vec::new();
    let mut blocks = Vec::new();
    for app in &state.recording {
        if named.contains(&app.name.as_str()) {
            continue;
        }
        named.push(&app.name);
        let item = Item {
            id: format!("recording-{}", app.index).into(),
            lead: Some(Lead {
                mark: Some(app_mark(app)),
                tint: None,
                glow: None,
                faded: false,
                size: None,
                on_click: None,
                top: false,
            }),
            overline: None,
            title: app.name.clone().into(),
            title_color: None,
            wrap: false,
            detail: SharedString::default(),
            trail: vec![Trail::Status("Recording".into(), CRITICAL)],
        };
        blocks.push(Block::Item(item, Vec::new()));
    }
    if !blocks.is_empty() {
        blocks.insert(0, Block::heading("In use by"));
    }
    blocks
}

/// An app's own volume, with what it plays inside the slider and where it plays in a dropdown.
fn app_item(
    audio: &Audio,
    state: &AudioState,
    app: &App,
    peaks: &HashMap<Probe, Peak>,
    p: &PanelCx<'_>,
) -> Block {
    let index = app.index;
    let choosing = audio.choosing.get() == Some(index);
    let id = format!("app-{index}");
    let headset = headset::latest().map(|battery| battery.name);
    let sinks: Vec<String> = state.sinks.iter().map(|sink| sink.name.clone()).collect();
    let device = Dropdown {
        id: format!("{id}-device").into(),
        choices: state
            .sinks
            .iter()
            .map(|sink| Choice {
                label: sink.description.clone().into(),
                glyph: p.glyph(glyphs::device(sink, headset.as_deref())),
            })
            .collect(),
        selected: state.sinks.iter().position(|sink| sink.index == app.device),
        open: choosing,
        on_toggle: p.call(move |audio: &Audio| {
            audio.choosing.set(if audio.choosing.get() == Some(index) {
                None
            } else {
                Some(index)
            });
        }),
        on_select: p.pick(move |audio: &Audio, choice: usize| {
            audio.choosing.set(None);
            if let Some(sink) = sinks.get(choice) {
                audio.send(Request::MoveApp(index, sink.clone()));
            }
        }),
    };
    // Twilight playing two streams shows two rows; their names tell them apart.
    let shared = state
        .playing
        .iter()
        .filter(|other| other.name == app.name)
        .count()
        > 1;
    let detail = if shared {
        app.stream.clone()
    } else {
        String::new()
    };
    let slider = Block::Slider(Slider {
        id: id.clone().into(),
        value: if app.muted { 0.0 } else { app.volume },
        color: ACCENT,
        signal: Some(peaks.get(&Probe::App(index)).map_or(0.0, |peak| peak.level)),
        on_change: Some(
            p.slider(move |audio: &Audio, value| audio.send(Request::SetAppVolume(index, value))),
        ),
    });
    let frame = FrameView {
        id: format!("{id}-frame").into(),
        look: Look::Inset,
        on_click: None,
        on_dismiss: None,
        countdown: None,
        selected: false,
        motion: crate::components::card::Motion::Still,
        shown_height: None,
    };
    let item = Item {
        id: id.into(),
        lead: Some(Lead {
            mark: Some(app_mark(app)),
            tint: None,
            glow: None,
            faded: app.muted,
            size: None,
            on_click: Some(p.call(move |audio: &Audio| audio.send(Request::ToggleAppMute(index)))),
            top: false,
        }),
        overline: None,
        title: app.name.clone().into(),
        title_color: None,
        wrap: false,
        detail: detail.into(),
        trail: vec![
            Trail::Dropdown(device),
            Trail::Text(
                amount(app.volume, app.muted).into(),
                if app.muted { OVERLAY2 } else { ACCENT },
            ),
        ],
    };
    Block::Frame(frame, vec![Block::Item(item, vec![slider])])
}

pub fn panel<'a>(audio: &Audio, state: &AudioState, p: &PanelCx<'a>) -> PanelView {
    let peaks = audio.mixer.peaks();
    headset::watch();
    let sides: Vec<Block> = [(Target::Output, "Output"), (Target::Input, "Input")]
        .into_iter()
        .filter_map(|(target, title)| {
            let mut blocks = vec![volume(state, target, &peaks, p)?];
            blocks.extend(devices(state, target, p));
            if target == Target::Input {
                blocks.extend(recording(state));
            }
            let format = state
                .default_device(target)
                .map(|device| device.format.clone())
                .unwrap_or_default();
            Some(Block::Card(
                Some(Title::with_detail(title, format)),
                card::Size::Medium,
                blocks,
            ))
        })
        .collect();
    if sides.is_empty() {
        let note = if state.connected {
            "No audio devices"
        } else {
            "No sound server"
        };
        return PanelView {
            header: Header::Tabs,
            blocks: vec![Block::Card(
                None,
                card::Size::Medium,
                vec![Block::note(note)],
            )],
        };
    }
    let mut blocks = Vec::new();
    if let Some(battery) = headset::latest() {
        blocks.push(headset_card(battery, p));
    }
    blocks.push(Block::Row(sides));
    if !state.playing.is_empty() {
        let apps = state
            .playing
            .iter()
            .map(|app| app_item(audio, state, app, &peaks, p))
            .collect();
        blocks.push(Block::Card(
            Some("Apps".into()),
            card::Size::Wide,
            vec![Block::Grid(2, apps)],
        ));
    }
    PanelView {
        header: Header::Tabs,
        blocks,
    }
}
