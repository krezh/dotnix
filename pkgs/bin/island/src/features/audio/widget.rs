//! The bar's volume gauge: click for the panel, right-click to mute, scroll to change.

use gpui::{AnyElement, MouseButton, ScrollEvent, prelude::*};

use super::{
    Audio, glyphs,
    request::Request,
    state::{AudioState, Target},
};
use crate::{
    bar::Gauge,
    components::{segments, tooltip::Tip},
    features::WidgetCx,
    island,
    theme::{CRITICAL, OVERLAY2, TEXT},
};

/// One scroll notch or `up`/`down` step.
pub const STEP: f32 = 0.05;

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

impl Audio {
    fn scroll(&self, event: &ScrollEvent) {
        let mut scrolled = self.scrolled.get();
        let steps = island::notches(&mut scrolled, event);
        self.scrolled.set(scrolled);
        if steps != 0 {
            self.send(Request::Step(Target::Output, steps as f32 * STEP));
        }
    }
}

/// Shows whether there is a default output at all.
pub fn present(state: &AudioState) -> bool {
    !state.connected || state.default_device(Target::Output).is_some()
}

pub fn widget(state: &AudioState, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
    let channel = state.channel(Target::Output);
    let gauge = if state.connected {
        state.default_device(Target::Output)?;
        Gauge {
            glyph: w.glyph(glyphs::speaker(channel.volume, channel.muted)),
            glyph_color: if channel.muted { OVERLAY2 } else { TEXT },
            busy: false,
            level: (!channel.muted).then_some(channel.volume),
            level_color: segments::color(channel.volume),
        }
    } else {
        Gauge {
            glyph: w.glyph(glyphs::speaker(0.0, true)),
            glyph_color: CRITICAL,
            busy: false,
            level: None,
            level_color: OVERLAY2,
        }
    };
    Some(
        w.gauge("bar-volume", gauge)
            .when_some(tip(state), |widget, text| {
                widget.tip(w.tips(), "bar-volume", text)
            })
            .on_click(w.toggle(Audio::panel_handle()))
            .on_mouse_down(
                MouseButton::Right,
                w.call(|audio: &Audio| audio.send(Request::ToggleMute(Target::Output))),
            )
            .on_scroll(w.on_event(Audio::scroll))
            .into_any_element(),
    )
}
