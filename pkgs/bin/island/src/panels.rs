use std::path::Path;

use crate::{
    components::{
        card,
        chart::Series,
        tile::{Meter as TileMeter, Part, Tile},
    },
    features::{self, Battery},
    glyphs::Glyphs,
    panel::{Block, Header, Meter, MeterShape, PanelView, SharedString, Split, Warning},
    theme::{GREEN, RED},
};

/// Nothing, for a panel whose content went away while it closes.
pub fn empty() -> PanelView {
    PanelView {
        scroll: false,
        header: Header::None,
        blocks: Vec::new(),
    }
}

/// A round chart ceiling (1, 2 or 5 × 10ⁿ) above every value, never below `floor`.
pub(crate) fn nice_peak<'a>(values: impl IntoIterator<Item = &'a f32>, floor: f32) -> f32 {
    let peak = values.into_iter().copied().fold(floor, f32::max);
    let magnitude = 10f32.powf(peak.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|step| step * magnitude)
        .find(|nice| *nice >= peak)
        .unwrap_or(peak)
}

pub(crate) fn series(
    values: &[f32],
    max: f32,
    color: u32,
    label: impl Into<SharedString>,
    format: fn(f32) -> String,
) -> Series {
    Series {
        values: values.to_vec(),
        max,
        color,
        label: label.into(),
        format,
    }
}

pub(crate) fn tile_meter(
    label: impl Into<SharedString>,
    fraction: f32,
    value: impl Into<SharedString>,
    color: u32,
) -> TileMeter {
    TileMeter {
        label: label.into(),
        fraction,
        value: value.into(),
        color,
        mark: None,
    }
}

pub(crate) fn titled(
    id: &str,
    span: usize,
    title: &str,
    value: Option<card::Stat>,
    parts: Vec<Part>,
) -> Tile {
    Tile {
        id: id.to_owned().into(),
        span,
        title: Some((title.to_owned().into(), value)),
        on_click: None,
        parts,
    }
}

/// Every battery as a ring, as many to a row as fit.
pub fn batteries_card(batteries: &[Battery], glyphs: &Glyphs) -> Block {
    let rings = batteries
        .iter()
        .map(|battery| Meter {
            glyph: glyphs.get(battery.glyph).map(Path::to_path_buf),
            caption: SharedString::default(),
            label: battery.name.clone(),
            level: Some((battery.fraction(), false)),
            color: battery.color(),
            shape: MeterShape::Ring,
        })
        .collect();
    let title = card::Title::from("Batteries").glyph(
        glyphs.get("dashboard-battery").map(Path::to_path_buf),
        GREEN,
    );
    Block::Card(
        Some(title),
        card::Size::Medium,
        vec![Block::Meters(1, rings)],
    )
}

pub fn battery_warnings(batteries: &[Battery], glyphs: &Glyphs) -> Vec<Warning> {
    batteries
        .iter()
        .filter(|battery| battery.low())
        .map(|battery| Warning {
            glyph: glyphs.get(battery.glyph).map(Path::to_path_buf),
            label: battery.name.clone(),
            value: battery.percent().into(),
            color: RED,
            on_click: None,
        })
        .collect()
}

/// The overview: what needs a look, every feature's switches, then their cards in two columns.
pub fn dashboard(dashboard: features::Dashboard) -> PanelView {
    let mut blocks = Vec::new();
    if !dashboard.warnings.is_empty() {
        blocks.push(Block::Warnings(dashboard.warnings));
    }
    if !dashboard.switches.is_empty() {
        blocks.push(Block::Switches(dashboard.switches));
    }
    blocks.push(Block::Columns(Split::Balanced, dashboard.cards));
    PanelView {
        scroll: false,
        header: Header::Tabs,
        blocks,
    }
}
