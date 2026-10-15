//! The widget toolkit every screen builds on. Screens themselves live with their domain
//! (`build::ui`, `report::ui`, `inputs::ui`, …).

mod animation;
mod blur;
mod energy;
mod fade_scroll;
mod format;
mod morph_bin;
mod segmented;
mod slide_bin;
pub(crate) mod theme;
mod widgets;

pub(crate) use animation::{
    Slide, Tween, ease_in_out, fade_in, glide, replace_screen, slide_screen,
};
pub(crate) use energy::{EnergyOverlay, draw_energy_capsule};
pub(crate) use morph_bin::MorphBin;
pub(crate) use fade_scroll::FadeScroll;
pub(crate) use format::{
    byte_unit, compact_size, duration_text, format_size, format_timestamp, seconds_since,
    short_age, signed_size,
};
pub(crate) use segmented::SegmentedControl;
pub(crate) use slide_bin::SlideBin;
pub(crate) use widgets::*;
