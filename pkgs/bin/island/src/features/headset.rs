use std::{
    sync::mpsc::{self, RecvTimeoutError, Sender},
    time::Duration,
};

use gpui::{AnyElement, MouseButton, prelude::*};
use serde_json::Value;
use tracing::{debug, warn};

use super::{Feature, Feed, Refreshed, WidgetCx};
use crate::{
    bar::Gauge,
    components::tooltip::Tip,
    island::Panel,
    run,
    supervisor::Ended,
    theme::{CRITICAL, GREEN, OVERLAY2, PEACH, RED, TEXT, YELLOW},
};

const INTERVAL: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(15);
const LOW: u8 = 15;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Battery {
    pub name: String,
    pub level: Option<u8>,
    pub charging: bool,
    pub chatmix: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Reading {
    #[default]
    Absent,
    Present(Battery),
    Failed(String),
}

fn read() -> Reading {
    let output = match run::output("headsetcontrol", &["-o", "json"], TIMEOUT) {
        Ok(output) => output,
        Err(error) => return Reading::Failed(error),
    };
    let Ok(report) = serde_json::from_str::<Value>(&output.stdout) else {
        return Reading::Failed(if output.stderr.is_empty() {
            "invalid output from headsetcontrol".to_owned()
        } else {
            output.stderr
        });
    };
    let devices = report["devices"].as_array().cloned().unwrap_or_default();
    devices
        .iter()
        .find_map(|device| {
            let battery = device.get("battery")?;
            let status = battery["status"].as_str().unwrap_or("BATTERY_UNAVAILABLE");
            if status == "BATTERY_UNAVAILABLE" {
                return None;
            }
            Some(Battery {
                name: device["device"]
                    .as_str()
                    .or_else(|| device["product"].as_str())
                    .unwrap_or("Headset")
                    .to_owned(),
                level: battery["level"].as_i64().and_then(|level| u8::try_from(level).ok()),
                charging: status == "BATTERY_CHARGING",
                chatmix: device["chatmix"].as_i64(),
            })
        })
        .map_or(Reading::Absent, Reading::Present)
}

fn battery_color(level: u8) -> u32 {
    match level {
        level if level <= LOW => RED,
        ..=40 => PEACH,
        ..=60 => YELLOW,
        _ => GREEN,
    }
}

fn tip(battery: &Battery) -> String {
    let level = battery.level.map_or("battery unknown".to_owned(), |level| format!("{level}%"));
    let state = if battery.charging { " · charging" } else { "" };
    format!("{} · {level}{state}", battery.name)
}

pub struct Headset {
    refresh: Sender<()>,
}

impl Headset {
    fn refresh(&self) {
        let _ = self.refresh.send(());
    }
}

impl Feature for Headset {
    const NAME: &'static str = "headset";
    const GLYPHS: &'static [super::GlyphSpec] = &[("headset", &["audio-headset-symbolic", "audio-headphones-symbolic"])];

    type State = Refreshed<Reading>;
    type Local = ();
    type Event = ();

    fn start(feed: Feed<Refreshed<Reading>>) -> Self {
        let (refresh, requests) = mpsc::channel();
        feed.supervise(move |feed| {
            let mut last: Option<Reading> = None;
            let mut asked = false;
            loop {
                if asked
                    && let Some(value) = last.clone()
                    && !feed.publish(Refreshed { value, refreshing: true })
                {
                    return Ended::Unwanted;
                }
                let reading = read();
                if asked || last.as_ref() != Some(&reading) {
                    if last.as_ref() != Some(&reading) {
                        match &reading {
                            Reading::Present(battery) => debug!(
                                name = battery.name,
                                level = ?battery.level,
                                charging = battery.charging,
                                "headset reading"
                            ),
                            Reading::Absent => debug!("no headset"),
                            Reading::Failed(error) => warn!("cannot read the headset: {error}"),
                        }
                    }
                    last = Some(reading.clone());
                    if !feed.publish(Refreshed { value: reading, refreshing: false }) {
                        return Ended::Unwanted;
                    }
                }
                asked = match requests.recv_timeout(INTERVAL) {
                    Ok(()) => true,
                    Err(RecvTimeoutError::Timeout) => false,
                    Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
                };
            }
        });
        Self { refresh }
    }

    fn present(_: &str, state: &Refreshed<Reading>) -> bool {
        matches!(state.value, Reading::Present(_))
    }

    fn widget(&self, _: &str, state: &Refreshed<Reading>, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        let Reading::Present(battery) = &state.value else {
            return None;
        };
        let color = match battery.level {
            _ if battery.charging => GREEN,
            Some(level) if level <= LOW => CRITICAL,
            _ => TEXT,
        };
        Some(
            w.gauge(
                "bar-headset",
                Gauge {
                    glyph: w.glyph("headset"),
                    glyph_color: color,
                    busy: state.refreshing,
                    level: battery.level.map(|level| level as f32 / 100.0),
                    level_color: battery.level.map_or(OVERLAY2, battery_color),
                },
            )
            .tip(w.tips(), "bar-headset", tip(battery))
            .on_click(w.toggle(Panel::System))
            .on_mouse_down(MouseButton::Right, w.call(Headset::refresh))
            .into_any_element(),
        )
    }
}
