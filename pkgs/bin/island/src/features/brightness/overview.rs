//! What brightness puts on the dashboard's overview: a Display card with a large slider for each
//! screen that answers.

use super::{runtime::Brightness, state::Levels};
use crate::{
    components::card::{Size, Title},
    features::PanelCx,
    panel::{Block, Slider, SliderLabel},
    theme::YELLOW,
};

pub fn cards(levels: &Levels, p: &PanelCx<'_>) -> Vec<Block> {
    let named = levels.screens.len() > 1;
    let sliders: Vec<Block> = levels
        .screens
        .iter()
        .filter_map(|screen| {
            let level = screen.level?;
            let name = screen.name.clone();
            let label = match (named, screen.model.is_empty()) {
                (false, _) => "Brightness".to_owned(),
                (true, true) => screen.name.clone(),
                (true, false) => screen.model.clone(),
            };
            Some(Block::Slider(Slider {
                id: format!("dashboard-brightness-{}", screen.name).into(),
                value: level,
                color: YELLOW,
                signal: None,
                on_change: Some(p.slider(move |brightness: &Brightness, value| {
                    brightness.set(&name, value)
                })),
                label: Some(SliderLabel {
                    glyph: p.glyph("brightness"),
                    label: label.into(),
                    amount: format!("{:.0}%", level * 100.0).into(),
                }),
            }))
        })
        .collect();
    if sliders.is_empty() {
        return Vec::new();
    }
    let title = Title::from("Display").glyph(p.glyph("brightness"), YELLOW);
    vec![Block::Card(Some(title), Size::Medium, sliders)]
}
