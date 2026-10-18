//! The headset's battery and, when it reports one, its ChatMix dial as game and chat volume.

use gpui::SharedString;

use super::panel::amount;
use crate::{
    components::card,
    features::{
        PanelCx,
        headset::{self, Battery},
    },
    panel::{Block, Item, Lead, Mark, Slider, Trail},
    theme::{BLUE, GREEN, MAUVE, SUBTEXT1},
};

fn battery_line(battery: &Battery) -> (String, u32) {
    let level = battery.level.map_or("Battery unknown".to_owned(), |level| format!("{level}%"));
    let color = match battery.level {
        _ if battery.charging => GREEN,
        Some(level) => headset::battery_color(level),
        None => SUBTEXT1,
    };
    let state = if battery.charging { " · Charging" } else { "" };
    (format!("{level}{state}"), color)
}

/// A level shown but not set here: the headset's own dial sets it.
fn reading(id: &'static str, title: &'static str, value: f32, color: u32) -> Block {
    let item = Item {
        id: format!("{id}-item").into(),
        lead: None,
        overline: None,
        title: title.into(),
        title_color: None,
        detail: SharedString::default(),
        trail: vec![Trail::Text(amount(value, false).into(), color)],
    };
    let slider = Slider {
        id: id.into(),
        value,
        color,
        signal: None,
        on_change: None,
    };
    Block::Item(item, vec![Block::Slider(slider)])
}

pub fn headset_card(battery: Battery, p: &PanelCx<'_>) -> Block {
    let (line, color) = battery_line(&battery);
    let name = Item {
        id: "headset".into(),
        lead: Some(Lead {
            mark: p.glyph("headset").map(|path| Mark::Glyph(path, MAUVE)),
            tint: Some(MAUVE),
            glow: None,
            faded: false,
            size: None,
            on_click: None,
        }),
        overline: None,
        title: battery.name.into(),
        title_color: None,
        detail: SharedString::default(),
        trail: Vec::new(),
    };
    let mut cells = vec![Block::Item(name, vec![Block::caption(line, color)])];
    if let Some(value) = battery.chatmix {
        let (game, chat) = headset::chatmix(value);
        cells.push(Block::Grid(
            2,
            vec![reading("chatmix-game", "Game", game, BLUE), reading("chatmix-chat", "Chat", chat, MAUVE)],
        ));
    }
    Block::Card(None, card::Size::Wide, vec![Block::Grid(2, cells)])
}
