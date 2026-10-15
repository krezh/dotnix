//! ZOWIE mouse battery from `zowie-batt`: a gauge in the bar while a mouse or receiver is present.

use std::{
    sync::mpsc::{self, RecvTimeoutError, Sender},
    time::Duration,
};

use gpui::{AnyElement, MouseButton, prelude::*};
use serde_json::Value;
use tracing::{debug, warn};

use super::{Feature, Feed, WidgetCx};
use crate::{
    bar::Gauge,
    components::tooltip::Tip,
    island::Panel,
    run,
    supervisor::Ended,
    theme::{CRITICAL, GREEN, PEACH, RED, TEXT, YELLOW},
};

const INTERVAL: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(10);
const LOW: u8 = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Battery {
    pub percent: u8,
    pub voltage_mv: u32,
    pub connection: String,
    pub power: Option<String>,
}

impl Battery {
    pub fn charging(&self) -> bool {
        matches!(self.power.as_deref(), Some("usb" | "external"))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Reading {
    #[default]
    Absent,
    Present(Battery),
    Failed(String),
}

fn read() -> Reading {
    let output = match run::output("zowie-batt", &["--json"], TIMEOUT) {
        Ok(output) => output,
        Err(error) => {
            if error.contains("No such file or directory") {
                return Reading::Absent;
            }
            return Reading::Failed(error);
        }
    };
    if output.code == Some(2) {
        return Reading::Absent;
    }
    if !output.success {
        return Reading::Failed(if output.stderr.is_empty() {
            format!("zowie-batt exited with status {:?}", output.code)
        } else {
            output.stderr
        });
    }
    let Ok(report) = serde_json::from_str::<Value>(&output.stdout) else {
        return Reading::Failed(if output.stderr.is_empty() {
            "invalid output from zowie-batt".to_owned()
        } else {
            output.stderr
        });
    };
    let Some(percent) = report["percent"].as_u64().and_then(|p| u8::try_from(p).ok()) else {
        return Reading::Failed("missing or invalid percent in zowie-batt report".to_owned());
    };
    let voltage_mv = report["voltage_mv"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0);
    let connection = report["connection"].as_str().unwrap_or("wireless").to_owned();
    let power = report["power"].as_str().map(ToOwned::to_owned);

    Reading::Present(Battery {
        percent,
        voltage_mv,
        connection,
        power,
    })
}

fn battery_color(percent: u8) -> u32 {
    match percent {
        p if p <= LOW => RED,
        ..=40 => PEACH,
        ..=60 => YELLOW,
        _ => GREEN,
    }
}

fn tip(battery: &Battery) -> String {
    let level = format!("~{}%", battery.percent);
    let voltage = format!("{:.2} V", battery.voltage_mv as f32 / 1000.0);
    let state = match battery.power.as_deref() {
        Some("usb") => " · wired (USB power)",
        Some("external") => " · wired (external power)",
        Some("battery") => " · wired",
        _ => " · wireless",
    };
    format!("ZOWIE mouse · {level} · {voltage}{state}")
}

pub struct Zowie {
    refresh: Sender<()>,
}

impl Zowie {
    /// Reads the mouse battery now instead of at the next interval.
    pub fn refresh(&self) {
        let _ = self.refresh.send(());
    }
}

impl Feature for Zowie {
    const NAME: &'static str = "zowie";
    const GLYPHS: &'static [super::GlyphSpec] = &[("mouse", &["input-mouse-symbolic", "input-mouse", "mouse-symbolic", "mouse"])];
    const COMMANDS: &'static [&'static str] = &["zowie"];
    const USAGE: &'static str = "  zowie refresh                         read the mouse battery now";

    type State = Reading;
    type Local = ();
    type Event = ();

    fn start(feed: Feed<Reading>) -> Self {
        let (refresh, requests) = mpsc::channel();
        feed.supervise(move |feed| {
            let mut last = None;
            loop {
                let reading = read();
                if last.as_ref() != Some(&reading) {
                    match &reading {
                        Reading::Present(battery) => debug!(
                            percent = battery.percent,
                            voltage_mv = battery.voltage_mv,
                            connection = battery.connection,
                            power = ?battery.power,
                            "zowie battery reading"
                        ),
                        Reading::Absent => debug!("no zowie mouse or receiver"),
                        Reading::Failed(error) => warn!("cannot read zowie battery: {error}"),
                    }
                    last = Some(reading.clone());
                    if !feed.publish(reading) {
                        return Ended::Unwanted;
                    }
                }
                if let Err(RecvTimeoutError::Disconnected) = requests.recv_timeout(INTERVAL) {
                    return Ended::Unwanted;
                }
            }
        });
        Self { refresh }
    }

    fn present(_: &str, state: &Reading) -> bool {
        matches!(state, Reading::Present(_))
    }

    fn widget(&self, _: &str, state: &Reading, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        let Reading::Present(battery) = state else {
            return None;
        };
        let color = match battery.percent {
            _ if battery.charging() => GREEN,
            level if level <= LOW => CRITICAL,
            _ => TEXT,
        };
        Some(
            w.gauge(
                "bar-zowie",
                Gauge {
                    glyph: w.glyph("mouse"),
                    glyph_color: color,
                    level: Some(battery.percent as f32 / 100.0),
                    level_color: if battery.charging() {
                        GREEN
                    } else {
                        battery_color(battery.percent)
                    },
                },
            )
            .tip(w.tips(), "bar-zowie", tip(battery))
            .on_click(w.toggle(Panel::System))
            .on_mouse_down(MouseButton::Right, w.call(Zowie::refresh))
            .into_any_element(),
        )
    }

    fn check(args: &[&str]) -> Result<(), String> {
        match args {
            [_, "refresh"] => Ok(()),
            _ => Err("usage: island ctl zowie refresh".to_owned()),
        }
    }

    fn command(&self, _: &Self::State, args: &[&str]) {
        if matches!(args, [_, "refresh"]) {
            self.refresh();
        }
    }
}
