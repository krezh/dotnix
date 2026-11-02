use std::{
    sync::mpsc::{self, Sender},
    time::Duration,
};

use serde_json::Value;
use tracing::{debug, warn};

use super::{Battery as Power, Feature, Feed, Refreshed, Surface, Wake, WidgetCx};
use crate::{
    bar_widget::BarWidget,
    control::{self, Command, Group, Report, Row, Takes},
    run,
    stage::Panel,
};

const INTERVAL: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(10);

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
        return Reading::Failed(
            output.failure(format!("zowie-batt exited with status {:?}", output.code)),
        );
    }
    let Ok(report) = serde_json::from_str::<Value>(&output.stdout) else {
        return Reading::Failed(output.failure("invalid output from zowie-batt"));
    };
    let Some(percent) = report["percent"]
        .as_u64()
        .and_then(|p| u8::try_from(p).ok())
    else {
        return Reading::Failed("missing or invalid percent in zowie-batt report".to_owned());
    };
    let voltage_mv = report["voltage_mv"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0);
    let connection = report["connection"]
        .as_str()
        .unwrap_or("wireless")
        .to_owned();
    let power = report["power"].as_str().map(ToOwned::to_owned);

    Reading::Present(Battery {
        percent,
        voltage_mv,
        connection,
        power,
    })
}

fn report(reading: &Reading) {
    match reading {
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
    refresh: Sender<Wake>,
}

impl Zowie {
    pub fn refresh(&self) {
        let _ = self.refresh.send(Wake::Asked);
    }
}

impl Feature for Zowie {
    const NAME: &'static str = "zowie";

    const CONTROLS: &'static [Command] = &[Command {
        name: "zowie",
        about: "Read the mouse battery now",
        group: Group::System,
        takes: Takes::Forms {
            forms: &[control::form(
                "refresh",
                control::Value::Nothing,
                "Read the mouse battery now",
            )],
            bare: None,
        },
        examples: &[],
    }];

    type State = Refreshed<Reading>;

    type Event = ();

    fn start(feed: Feed<Refreshed<Reading>>) -> Self {
        let (refresh, requests) = mpsc::channel();
        feed.supervise(move |feed| feed.poll(&requests, || INTERVAL, read, report));
        Self { refresh }
    }

    fn batteries(&self, state: &Refreshed<Reading>) -> Vec<Power> {
        let Reading::Present(battery) = &state.value else {
            return Vec::new();
        };
        vec![Power {
            id: "zowie".into(),
            glyph: "mouse",
            name: "ZOWIE mouse".into(),
            level: battery.percent,
            charging: battery.charging(),
        }]
    }

    fn command(&self, _: &Self::State, args: &[&str]) -> Report {
        if !matches!(args, [_, "refresh"]) {
            return Report::default();
        }
        self.refresh();
        Report::did(Row::said("Reading the mouse battery"))
    }
}

impl Surface for Zowie {
    const GLYPHS: &'static [super::GlyphSpec] = &[("mouse", &["mouse"])];

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
        let power = self.batteries(state).pop()?;
        Some(
            w.battery("bar-zowie", &power, state.refreshing)
                .tip(tip(battery))
                .on_click(w.toggle(Panel::Dashboard))
                .on_right(w.call(Zowie::refresh)),
        )
    }
}
