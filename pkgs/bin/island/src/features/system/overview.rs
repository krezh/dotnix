//! What the system puts on the dashboard's overview: a card of what it is busy with, and warnings
//! when it runs hot or short of memory.

use super::{System, panel::percent, state::State};
use crate::{
    components::{card, chart::Phase},
    features::{PanelCx, Surface},
    panel::{self, Block, Handler, Meter, MeterShape, SharedString, Warning},
    panels::series,
    sys::{Sensor, SysStats},
    theme::{GREEN, LAVENDER, PEACH, RED, SAPPHIRE},
    units::{bytes, span},
};

/// Hot enough to say so on the overview.
const HOT: f32 = 85.0;
/// Memory this full is worth a look.
const MEMORY_FULL: f32 = 0.9;

fn open() -> Handler {
    panel::on(|ui| ui.open(System::panel_handle()))
}

fn hottest(stats: &SysStats) -> Option<&Sensor> {
    stats
        .sensors
        .iter()
        .max_by(|a, b| a.celsius.total_cmp(&b.celsius))
}

pub fn warnings(state: &State, p: &PanelCx<'_>) -> Vec<Warning> {
    let stats = &*state.stats;
    let glyph = p.glyph("dashboard-warning");
    let mut warnings = Vec::new();
    if let Some(sensor) = hottest(stats).filter(|sensor| sensor.celsius >= HOT) {
        warnings.push(Warning {
            glyph: glyph.clone(),
            label: sensor.label.clone().into(),
            value: format!("{:.0}°C", sensor.celsius).into(),
            color: RED,
            on_click: Some(open()),
        });
    }
    let memory = stats.mem_used_kib as f32 / stats.mem_total_kib.max(1) as f32;
    if memory >= MEMORY_FULL {
        warnings.push(Warning {
            glyph,
            label: "Memory".into(),
            value: format!("{:.0}%", memory * 100.0).into(),
            color: PEACH,
            on_click: Some(open()),
        });
    }
    warnings
}

/// A number with its recent history, such as CPU use.
fn history_meter(
    label: impl Into<SharedString>,
    amount: String,
    unit: &str,
    color: u32,
    history: Option<(&[f32], &'static str)>,
    phase: Phase,
) -> Meter {
    Meter {
        glyph: None,
        caption: SharedString::default(),
        label: label.into(),
        level: None,
        color,
        shape: MeterShape::History {
            amount: amount.into(),
            unit: unit.to_owned().into(),
            history: history.map(|(values, name)| series(values, 1.0, color, name, percent)),
            phase: Some(phase),
        },
    }
}

pub fn card(state: &State, p: &PanelCx<'_>) -> Block {
    let (stats, phase) = (&*state.stats, state.phase());
    let kib = |value: u64| value * 1024;
    let mut meters = vec![history_meter(
        "CPU",
        format!("{:.0}", stats.cpu * 100.0),
        "%",
        SAPPHIRE,
        Some((&stats.cpu_history, "CPU")),
        phase,
    )];
    if let Some(gpu) = stats.gpu {
        meters.push(history_meter(
            "GPU",
            format!("{:.0}", gpu.busy * 100.0),
            "%",
            GREEN,
            Some((&stats.gpu_history, "GPU")),
            phase,
        ));
    }
    meters.push(history_meter(
        "Memory",
        bytes(kib(stats.mem_used_kib)),
        &format!("/ {}", bytes(kib(stats.mem_total_kib))),
        LAVENDER,
        Some((&stats.mem_history, "Memory")),
        phase,
    ));
    if let Some(sensor) = hottest(stats) {
        let color = if sensor.celsius >= HOT { RED } else { PEACH };
        meters.push(history_meter(
            sensor.label.clone(),
            format!("{:.0}", sensor.celsius),
            "°C",
            color,
            None,
            phase,
        ));
    }
    let title = card::Title::with_detail("System", format!("up {}", span(stats.uptime_secs)))
        .glyph(p.glyph("tab-system"), SAPPHIRE)
        .opens(open());
    Block::Card(
        Some(title),
        card::Size::Medium,
        vec![Block::Meters(2, meters)],
    )
}
