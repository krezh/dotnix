use std::{
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Sender},
    },
    time::Duration,
};

use serde_json::Value;
use tracing::{debug, warn};

use super::{Battery as Power, Feature, Feed, Refreshed, Surface, Wake, WidgetCx};
use crate::{
    bar_widget::{BarWidget, Gauge},
    run,
    stage::Panel,
    theme::{GREEN, OVERLAY2, TEXT},
};

const INTERVAL: Duration = Duration::from_secs(60);
/// While the audio panel shows ChatMix, so turning the dial shows up soon.
const WATCHED_INTERVAL: Duration = Duration::from_secs(2);
const TIMEOUT: Duration = Duration::from_secs(15);

/// The latest reading, for the audio panel.
static LATEST: Mutex<Option<Battery>> = Mutex::new(None);
static WATCHERS: AtomicUsize = AtomicUsize::new(0);
static WAKE: Mutex<Option<Sender<Wake>>> = Mutex::new(None);

pub fn latest() -> Option<Battery> {
    LATEST.lock().ok()?.clone()
}

pub struct Watch;

impl Drop for Watch {
    fn drop(&mut self) {
        WATCHERS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Keeps readings frequent until the returned [`Watch`] is dropped.
pub fn watch() -> Watch {
    if WATCHERS.fetch_add(1, Ordering::SeqCst) == 0
        && let Some(wake) = WAKE.lock().ok().and_then(|wake| wake.clone())
    {
        let _ = wake.send(Wake::Retimed);
    }
    Watch
}

fn interval() -> Duration {
    if WATCHERS.load(Ordering::SeqCst) > 0 {
        WATCHED_INTERVAL
    } else {
        INTERVAL
    }
}

/// Game and chat volume, 0 to 1, from the dial's 0 (all game) to 128 (all chat).
pub fn chatmix(value: i64) -> (f32, f32) {
    const MIDDLE: f32 = 64.0;
    let value = value.clamp(0, 128) as f32;
    let game = ((128.0 - value) / MIDDLE).min(1.0);
    let chat = (value / MIDDLE).min(1.0);
    (game, chat)
}

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
        return Reading::Failed(output.failure("invalid output from headsetcontrol"));
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
                level: battery["level"]
                    .as_i64()
                    .and_then(|level| u8::try_from(level).ok()),
                charging: status == "BATTERY_CHARGING",
                chatmix: device["chatmix"].as_i64(),
            })
        })
        .map_or(Reading::Absent, Reading::Present)
}

fn reading() -> Reading {
    let reading = read();
    if let Ok(mut latest) = LATEST.lock() {
        *latest = match &reading {
            Reading::Present(battery) => Some(battery.clone()),
            _ => None,
        };
    }
    reading
}

fn report(reading: &Reading) {
    match reading {
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

fn tip(battery: &Battery) -> String {
    let level = battery
        .level
        .map_or("battery unknown".to_owned(), |level| format!("{level}%"));
    let state = if battery.charging { " · charging" } else { "" };
    format!("{} · {level}{state}", battery.name)
}

pub struct Headset {
    refresh: Sender<Wake>,
}

impl Headset {
    fn refresh(&self) {
        let _ = self.refresh.send(Wake::Asked);
    }
}

impl Feature for Headset {
    const NAME: &'static str = "headset";

    type State = Refreshed<Reading>;

    type Event = ();

    fn start(feed: Feed<Refreshed<Reading>>) -> Self {
        let (refresh, requests) = mpsc::channel();
        if let Ok(mut wake) = WAKE.lock() {
            *wake = Some(refresh.clone());
        }
        feed.supervise(move |feed| feed.poll(&requests, interval, reading, report));
        Self { refresh }
    }

    fn batteries(&self, state: &Refreshed<Reading>) -> Vec<Power> {
        let Reading::Present(battery) = &state.value else {
            return Vec::new();
        };
        battery
            .level
            .map(|level| Power {
                id: "headset".into(),
                glyph: "headset",
                name: battery.name.clone().into(),
                level,
                charging: battery.charging,
            })
            .into_iter()
            .collect()
    }
}

impl Surface for Headset {
    const DASHBOARD_ORDER: u8 = 40;

    const GLYPHS: &'static [super::GlyphSpec] = &[("headset", &["headset"])];

    type Local = ();

    fn present(_: &str, state: &Refreshed<Reading>) -> bool {
        matches!(state.value, Reading::Present(_))
    }

    fn widget(
        &self,
        _: &str,
        state: &Refreshed<Reading>,
        w: &mut WidgetCx<'_>,
    ) -> Option<BarWidget> {
        let Reading::Present(battery) = &state.value else {
            return None;
        };
        // A headset that does not say how full it is still shows, without a level.
        let gauge = match self.batteries(state).pop() {
            Some(power) => w.battery(&power, state.refreshing),
            None => w.gauge(Gauge {
                glyph: w.glyph("headset"),
                glyph_color: if battery.charging { GREEN } else { TEXT },
                busy: state.refreshing,
                level: None,
                level_color: OVERLAY2,
            }),
        };
        Some(
            gauge
                .tip(tip(battery))
                .on_click(w.toggle(Panel::Dashboard))
                .on_right(w.call(Headset::refresh)),
        )
    }
}
