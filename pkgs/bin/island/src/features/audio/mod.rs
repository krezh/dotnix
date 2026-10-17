//! While the sound server is gone the widget stays, drawn as an error.

mod pulse;

use std::cell::Cell;

use gpui::{AnyElement, MouseButton, ScrollWheelEvent, SharedString, prelude::*};

use super::{
    Feature, Feed, GlyphSpec, IslandInfo, PanelCx, WidgetCx,
    osd::{self, Osd, Shown},
};
use crate::{
    bar::Gauge,
    components::{card, tooltip::Tip},
    control::{self, Change},
    island::{self, Panel},
    panel::{Block, Entry, Header, Level, PanelView},
    theme::{BLUE, CRITICAL, GREEN, MAUVE, OVERLAY2, PEACH, RED, TEXT, YELLOW},
};
use pulse::{AudioState, Mixer, Request, Target};

/// One scroll notch or `up`/`down` step.
const STEP: f32 = 0.05;

fn speaker(volume: f32, muted: bool) -> &'static str {
    match volume {
        _ if muted || volume <= 0.0 => "volume-muted",
        volume if volume < 0.34 => "volume-low",
        volume if volume < 0.67 => "volume-medium",
        _ => "volume-high",
    }
}

fn microphone(muted: bool) -> &'static str {
    if muted { "mic-muted" } else { "mic" }
}

fn loudness_color(volume: f32) -> u32 {
    match volume {
        volume if volume < 0.34 => GREEN,
        volume if volume < 0.67 => YELLOW,
        volume if volume <= 0.9 => PEACH,
        _ => RED,
    }
}

fn tip(state: &AudioState) -> Option<String> {
    if !state.connected {
        return Some("No sound server".to_owned());
    }
    let device = &state.default_device(Target::Output)?.description;
    let channel = state.channel(Target::Output);
    let level = if channel.muted {
        "muted".to_owned()
    } else {
        format!("{}%", (channel.volume * 100.0).round() as i32)
    };
    Some(format!("{device} · {level}"))
}

pub struct Audio {
    mixer: Mixer,
    /// Scroll left over below a whole notch.
    scrolled: Cell<f32>,
}

impl Audio {
    fn send(&self, request: Request) {
        self.mixer.send(request);
    }

    fn scroll(&self, event: &ScrollWheelEvent) {
        let mut scrolled = self.scrolled.get();
        let steps = island::notches(&mut scrolled, event);
        self.scrolled.set(scrolled);
        if steps != 0 {
            self.send(Request::Step(Target::Output, steps as f32 * STEP));
        }
    }

    fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }
}

fn target_of(word: &str) -> Target {
    if word == "mic" { Target::Input } else { Target::Output }
}

fn level(state: &AudioState, target: Target, p: &PanelCx<'_>) -> Option<Level> {
    let device = state.default_device(target)?;
    let channel = state.channel(target);
    let (id, glyph, color) = match target {
        Target::Output => ("level-output", speaker(channel.volume, channel.muted), BLUE),
        Target::Input => ("level-input", microphone(channel.muted), MAUVE),
    };
    Some(Level {
        id: id.into(),
        label: device.description.clone().into(),
        detail: SharedString::default(),
        glyph: p.glyph(glyph),
        value: channel.volume,
        muted: channel.muted,
        color,
        on_change: Some(p.slider(move |audio: &Audio, value| audio.send(Request::SetVolume(target, value)))),
        on_mute: Some(p.call(move |audio: &Audio| audio.send(Request::ToggleMute(target)))),
    })
}

fn devices(state: &AudioState, target: Target, p: &PanelCx<'_>) -> Option<Block<'static>> {
    let devices = state.devices(target);
    if devices.len() < 2 {
        return None;
    }
    let current = state.default_device(target).map(|device| device.name.clone());
    let entries = devices
        .iter()
        .enumerate()
        .map(|(index, device)| {
            let name = device.name.clone();
            Entry {
                id: format!("device-{target:?}-{index}").into(),
                label: device.description.clone().into(),
                enabled: true,
                separator: false,
                check: Some(current.as_deref() == Some(device.name.as_str())),
                submenu: false,
                on_click: Some(p.call(move |audio: &Audio| audio.send(Request::SetDefault(target, name.clone())))),
            }
        })
        .collect();
    Some(Block::Choices("Devices".into(), entries))
}

impl Feature for Audio {
    const NAME: &'static str = "audio";
    const GLYPHS: &'static [GlyphSpec] = &[
        ("volume-high", &["audio-volume-high-symbolic"]),
        ("volume-medium", &["audio-volume-medium-symbolic"]),
        ("volume-low", &["audio-volume-low-symbolic"]),
        ("volume-muted", &["audio-volume-muted-symbolic"]),
        ("mic", &["microphone-sensitivity-high-symbolic", "audio-input-microphone-symbolic"]),
        ("mic-muted", &["microphone-sensitivity-muted-symbolic"]),
    ];
    const PANEL: Option<&'static str> = Some("audio");
    const TAB: Option<(&'static str, u8)> = Some(("Audio", 2));
    const COMMANDS: &'static [&'static str] = &["volume", "mic"];
    const USAGE: &'static str = "  volume up|down|mute|set PERCENT       default output
  mic up|down|mute|set PERCENT          default input";

    type State = AudioState;
    type Local = Shown;
    type Event = ();

    fn start(feed: Feed<AudioState>) -> Self {
        Self {
            mixer: pulse::start(feed),
            scrolled: Cell::new(0.0),
        }
    }

    fn sync_local(_: &AudioState, shown: &mut Shown, island: &IslandInfo<'_>, _animate: bool) {
        shown.sync(island);
    }

    fn popup<'a>(&self, _: &AudioState, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        osd::popup(p)
    }

    fn elapse(&self, shown: &mut Shown, _hovered: bool) {
        shown.expire();
    }

    /// Connected with no devices yet, so the widget waits for the first query instead of
    /// flashing the error state.
    fn initial() -> AudioState {
        AudioState {
            connected: true,
            ..AudioState::default()
        }
    }

    fn present(_: &str, state: &AudioState) -> bool {
        !state.connected || state.default_device(Target::Output).is_some()
    }

    fn widget(&self, _: &str, state: &AudioState, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        let channel = state.channel(Target::Output);
        let gauge = if state.connected {
            state.default_device(Target::Output)?;
            Gauge {
                glyph: w.glyph(speaker(channel.volume, channel.muted)),
                glyph_color: if channel.muted { OVERLAY2 } else { TEXT },
                busy: false,
                level: (!channel.muted).then_some(channel.volume),
                level_color: loudness_color(channel.volume),
            }
        } else {
            Gauge {
                glyph: w.glyph(speaker(0.0, true)),
                glyph_color: CRITICAL,
                busy: false,
                level: None,
                level_color: OVERLAY2,
            }
        };
        Some(
            w.gauge("bar-volume", gauge)
                .when_some(tip(state), |widget, text| widget.tip(w.tips(), "bar-volume", text))
                .on_click(w.toggle(Self::panel_handle()))
                .on_mouse_down(
                    MouseButton::Right,
                    w.call(|audio: &Audio| audio.send(Request::ToggleMute(Target::Output))),
                )
                .on_scroll_wheel(w.on_event(Audio::scroll))
                .into_any_element(),
        )
    }

    fn panel<'a>(&self, state: &AudioState, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        let groups: Vec<Block<'static>> = [(Target::Output, "Output"), (Target::Input, "Input")]
            .into_iter()
            .filter_map(|(target, title)| {
                let mut blocks = vec![Block::Levels(vec![level(state, target, p)?])];
                blocks.extend(devices(state, target, p));
                Some(Block::Card(Some(title.into()), card::Size::Medium, blocks))
            })
            .collect();
        let blocks = if groups.is_empty() {
            let note = if state.connected { "No audio devices" } else { "No sound server" };
            vec![Block::Card(None, card::Size::Medium, vec![Block::Note(note.into())])]
        } else {
            vec![Block::Row(groups)]
        };
        Some(PanelView {
            header: Header::Tabs,
            blocks,
        })
    }

    /// Nothing shows while connecting, or on the first state after connecting, which only fills in
    /// the devices.
    fn osd(before: &AudioState, after: &AudioState) -> Option<Osd> {
        if !before.connected || (before.sinks.is_empty() && before.sources.is_empty()) {
            return None;
        }
        [Target::Output, Target::Input].into_iter().find_map(|target| {
            let (old, new) = (before.channel(target), after.channel(target));
            let device = after.default_device(target)?;
            let switched = before.default_device(target).map(|device| &device.name) != Some(&device.name);
            let (glyph, color, caption) = match target {
                Target::Output => (speaker(new.volume, new.muted), BLUE, "Output"),
                Target::Input => (microphone(new.muted), MAUVE, "Input"),
            };
            (switched || old != new).then(|| Osd {
                glyph,
                color,
                caption,
                label: device.description.clone(),
                level: (!switched).then_some((new.volume, new.muted)),
            })
        })
    }

    fn check(args: &[&str]) -> Result<(), String> {
        control::change(&args[1..], true).map(drop)
    }

    fn command(&self, _: &AudioState, args: &[&str]) {
        let Ok(change) = control::change(&args[1..], true) else {
            return;
        };
        let target = target_of(args[0]);
        self.send(match change {
            Change::Step(delta) => Request::Step(target, delta),
            Change::Set(value) => Request::SetVolume(target, value),
            Change::ToggleMute => Request::ToggleMute(target),
        });
    }
}
