//! What molt's screens share: their frames, header, and the palette's shade of molt-core's colors.

use std::rc::Rc;

use molt_core::{home::present::Tone, units::Hue};

use crate::{
    components::card::{Look, Motion},
    features::PanelCx,
    panel::{FrameView, Handler, HeaderLead, Views},
    theme::{
        BLUE, CRITICAL, GREEN, LAVENDER, MAROON, MAUVE, OVERLAY1, OVERLAY2, PEACH, PINK, RED,
        SAPPHIRE, SKY, TEAL, YELLOW,
    },
};

pub(super) fn frame(id: String, min_height: Option<f32>) -> FrameView {
    FrameView {
        id: id.into(),
        look: Look::Raised { urgent: false },
        on_click: None,
        on_dismiss: None,
        countdown: None,
        selected: false,
        motion: Motion::Appear,
        shown_height: None,
        min_height,
    }
}

pub(super) fn tone(tone: Tone) -> u32 {
    match tone {
        Tone::Neutral | Tone::Muted => OVERLAY2,
        Tone::Success => GREEN,
        Tone::Attention => YELLOW,
        Tone::Error => CRITICAL,
    }
}

/// A screen's header: its lead, and a way back to Home for a tool.
pub(super) fn views(lead: Option<HeaderLead>, back: Option<Handler>, _: &PanelCx<'_>) -> Views {
    Views {
        lead,
        tabs: Vec::new(),
        selected: 0,
        on_select: Rc::new(|_, _| {}),
        pick: None,
        segments: None,
        field: None,
        buttons: Vec::new(),
        icons: Vec::new(),
        passing: None,
        back,
        status: None,
        route: None,
    }
}

/// The palette's shade of a color molt-core names.
pub(super) const fn hue(hue: Hue) -> u32 {
    match hue {
        Hue::Green => GREEN,
        Hue::Red => RED,
        Hue::Blue => BLUE,
        Hue::Yellow => YELLOW,
        Hue::Sapphire => SAPPHIRE,
        Hue::Mauve => MAUVE,
        Hue::Peach => PEACH,
        Hue::Pink => PINK,
        Hue::Sky => SKY,
        Hue::Teal => TEAL,
        Hue::Lavender => LAVENDER,
        Hue::Maroon => MAROON,
        Hue::Overlay => OVERLAY1,
    }
}
