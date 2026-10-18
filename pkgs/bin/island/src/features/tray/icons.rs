use std::time::Instant;

use gpui::SpringConfig;

use super::sni::TrayItem;
use crate::{bar, features::Local, presence::Presence};

pub(super) const ICON: f32 = 18.0;
const MOTION: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);

fn step(spacing: f32) -> f32 {
    ICON + 2.0 * bar::half(spacing)
}

pub struct Icons(pub(super) Presence<TrayItem>);

impl Default for Icons {
    fn default() -> Self {
        Self(Presence::new(MOTION))
    }
}

impl Local for Icons {
    fn advance(&mut self, dt: f32, now: Instant) {
        self.0.advance(dt, now);
    }

    /// Icons that need attention breathe, so they keep frames coming.
    fn is_settled(&self) -> bool {
        self.0.is_settled() && !self.0.iter().any(|slot| slot.item.attention)
    }

    fn pending_width(&self, spacing: f32) -> f32 {
        self.0.pending_width(step(spacing))
    }
}

pub(super) fn sync(items: &[TrayItem], icons: &mut Icons, animate: bool) {
    let keyed: Vec<(String, TrayItem)> = items
        .iter()
        .map(|item| (item.key.clone(), item.clone()))
        .collect();
    icons.0.sync(&keyed, animate);
}
