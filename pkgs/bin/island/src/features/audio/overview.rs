//! What audio puts on the dashboard's overview: the microphone's switch, a Sound card with the
//! output and input as large sliders and their devices under them, and a card of what is playing.

use gpui::SharedString;

use super::{
    Audio, glyphs,
    panel::amount,
    request::Request,
    state::{AudioState, Target},
};
use crate::{
    components::card::{Size, Title},
    features::{PanelCx, headset},
    island::Mode,
    panel::{
        self, Block, Choice, Dropdown, Item, Lead, Mark, Slider, SliderLabel, ActionTile, Trail,
    },
    theme::{CRITICAL, MAUVE, OVERLAY2, PINK, SUBTEXT1, app_color},
};

/// Apps the Playing card lists before saying how many more there are.
const PLAYING: usize = 4;
const PICK_GLYPH: f32 = 16.0;

pub fn switches(state: &AudioState, p: &PanelCx<'_>) -> Vec<ActionTile> {
    if state.default_device(Target::Input).is_none() {
        return Vec::new();
    }
    let muted = state.channel(Target::Input).muted;
    vec![ActionTile {
        id: "switch-microphone".into(),
        glyph: p.glyph(glyphs::microphone(muted)),
        label: "Microphone".into(),
        state: Some(if muted { "Muted" } else { "Live" }.into()),
        color: if muted { CRITICAL } else { PINK },
        armed: true,
        on_click: p.call(|audio: &Audio| audio.send(Request::ToggleMute(Target::Input))),
    }]
}

fn level(state: &AudioState, target: Target, p: &PanelCx<'_>) -> Option<Block> {
    state.default_device(target)?;
    let channel = state.channel(target);
    let (id, label, glyph, color) = match target {
        Target::Output => (
            "dashboard-output",
            "Output",
            glyphs::speaker(channel.volume, channel.muted),
            MAUVE,
        ),
        Target::Input => (
            "dashboard-input",
            "Input",
            glyphs::microphone(channel.muted),
            PINK,
        ),
    };
    Some(Block::Slider(Slider {
        id: id.into(),
        value: if channel.muted { 0.0 } else { channel.volume },
        color: if channel.muted { OVERLAY2 } else { color },
        signal: None,
        on_change: Some(
            p.slider(move |audio: &Audio, value| audio.send(Request::SetVolume(target, value))),
        ),
        label: Some(SliderLabel {
            glyph: p.glyph(glyph),
            label: label.into(),
            amount: amount(channel.volume, channel.muted).into(),
        }),
    }))
}

/// Which device plays or listens, as a dropdown after its glyph, for a side with a choice.
fn picker(audio: &Audio, state: &AudioState, target: Target, p: &PanelCx<'_>) -> Option<Block> {
    let devices = state.devices(target);
    if devices.len() < 2 {
        return None;
    }
    let headset = headset::latest().map(|battery| battery.name);
    let names: Vec<String> = devices.iter().map(|device| device.name.clone()).collect();
    let current = state.default_device(target).map(|device| device.name.clone());
    let side = match target {
        Target::Output => "output",
        Target::Input => "input",
    };
    let dropdown = Dropdown {
        id: format!("dashboard-{side}-device").into(),
        choices: devices
            .iter()
            .map(|device| Choice {
                label: device.description.clone().into(),
                glyph: p.glyph(glyphs::device(device, headset.as_deref())),
                detail: None,
            })
            .collect(),
        selected: devices
            .iter()
            .position(|device| Some(&device.name) == current.as_ref()),
        open: audio.picking.get() == Some(target),
        height: None,
        on_toggle: p.call(move |audio: &Audio| {
            let open = audio.picking.get() == Some(target);
            audio.picking.set((!open).then_some(target));
        }),
        on_select: p.pick(move |audio: &Audio, choice: usize| {
            audio.picking.set(None);
            if let Some(name) = names.get(choice) {
                audio.send(Request::SetDefault(target, name.clone()));
            }
        }),
    };
    let glyph = match target {
        Target::Output => glyphs::speaker(1.0, false),
        Target::Input => glyphs::microphone(false),
    };
    let item = Item {
        id: format!("dashboard-{side}-pick").into(),
        lead: Some(Lead {
            mark: p.glyph(glyph).map(|path| Mark::Glyph(path, SUBTEXT1)),
            tint: None,
            glow: None,
            faded: false,
            size: Some(PICK_GLYPH),
            on_click: None,
            top: false,
        }),
        overline: None,
        title: SharedString::default(),
        title_color: None,
        wrap: false,
        detail: SharedString::default(),
        tinted: Vec::new(),
        trail: vec![Trail::Dropdown(dropdown)],
    };
    Some(Block::Item(item, Vec::new()))
}

fn sound_card(audio: &Audio, state: &AudioState, p: &PanelCx<'_>) -> Option<Block> {
    let mut blocks: Vec<Block> = [Target::Output, Target::Input]
        .into_iter()
        .filter_map(|target| level(state, target, p))
        .collect();
    if blocks.is_empty() {
        return None;
    }
    let pickers: Vec<Block> = [Target::Output, Target::Input]
        .into_iter()
        .filter_map(|target| picker(audio, state, target, p))
        .collect();
    if !pickers.is_empty() {
        blocks.push(Block::Row(pickers));
    }
    let title = Title::from("Sound")
        .glyph(p.glyph("tab-audio"), MAUVE)
        .opens(panel::on(|island, cx| {
            island.set_mode(Mode::Open(Audio::panel_handle()), cx)
        }));
    Some(Block::Card(Some(title), Size::Medium, blocks))
}

fn playing_card(state: &AudioState, p: &PanelCx<'_>) -> Option<Block> {
    if state.playing.is_empty() {
        return None;
    }
    let mut blocks: Vec<Block> = state
        .playing
        .iter()
        .take(PLAYING)
        .map(|app| {
            let index = app.index;
            let item = Item {
                id: format!("dashboard-app-{index}").into(),
                lead: Some(Lead {
                    mark: Some(match &app.icon {
                        Some(path) => Mark::Image(path.clone()),
                        None => Mark::Letter(app.name.clone().into(), app_color(&app.name)),
                    }),
                    tint: None,
                    glow: None,
                    faded: app.muted,
                    size: None,
                    on_click: None,
                    top: false,
                }),
                overline: None,
                title: app.name.clone().into(),
                title_color: None,
                wrap: false,
                detail: state
                    .sink(app.device)
                    .map(|sink| SharedString::from(sink.description.clone()))
                    .unwrap_or_default(),
                tinted: Vec::new(),
                trail: vec![Trail::Text(
                    amount(app.volume, app.muted).into(),
                    if app.muted { OVERLAY2 } else { SUBTEXT1 },
                )],
            };
            let slider = Block::Slider(Slider {
                id: format!("dashboard-app-{index}-level").into(),
                value: if app.muted { 0.0 } else { app.volume },
                color: MAUVE,
                signal: None,
                on_change: Some(p.slider(move |audio: &Audio, value| {
                    audio.send(Request::SetAppVolume(index, value))
                })),
                label: None,
            });
            Block::Item(item, vec![slider])
        })
        .collect();
    let more = state.playing.len().saturating_sub(PLAYING);
    if more > 0 {
        blocks.push(Block::caption(format!("{more} more"), OVERLAY2));
    }
    let count = state.playing.len();
    let title = Title::with_detail(
        "Playing",
        if count == 1 {
            "1 app".to_owned()
        } else {
            format!("{count} apps")
        },
    )
    .glyph(p.glyph("tab-audio"), MAUVE)
    .opens(panel::on(|island, cx| {
        island.set_mode(Mode::Open(Audio::panel_handle()), cx)
    }));
    Some(Block::Card(Some(title), Size::Medium, blocks))
}

pub fn cards(audio: &Audio, state: &AudioState, p: &PanelCx<'_>) -> Vec<Block> {
    [sound_card(audio, state, p), playing_card(state, p)]
        .into_iter()
        .flatten()
        .collect()
}
