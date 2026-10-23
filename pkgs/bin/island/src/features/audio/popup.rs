//! The pop-up shown when a volume changes or a default device switches.

use super::{
    glyphs,
    state::{AudioState, Target},
};
use crate::{
    features::{Osd, osd::DURATION},
    theme::{BLUE, MAUVE},
};

/// Nothing shows while connecting, or on the first state after connecting, which only fills in
/// the devices.
pub fn osd(before: &AudioState, after: &AudioState) -> Option<Osd> {
    if !before.connected || (before.sinks.is_empty() && before.sources.is_empty()) {
        return None;
    }
    [Target::Output, Target::Input]
        .into_iter()
        .find_map(|target| {
            let (old, new) = (before.channel(target), after.channel(target));
            let device = after.default_device(target)?;
            let switched =
                before.default_device(target).map(|device| &device.name) != Some(&device.name);
            let (glyph, color, caption) = match target {
                Target::Output => (glyphs::speaker(new.volume, new.muted), BLUE, "Output"),
                Target::Input => (glyphs::microphone(new.muted), MAUVE, "Input"),
            };
            (switched || old != new).then(|| Osd {
                glyph,
                color,
                caption,
                label: device.description.clone(),
                level: (!switched).then_some((new.volume, new.muted)),
                lasts: DURATION,
            })
        })
}
