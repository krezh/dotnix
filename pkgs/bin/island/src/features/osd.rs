//! A feature with an OSD names it in [`Feature::osd`](super::Feature::osd), keeps [`Shown`] as its
//! [`Local`], and draws it with [`popup`].

use std::time::{Duration, Instant};

use super::{IslandInfo, Local, PanelCx};
use crate::panel::{Block, Header, Meter, PanelView};

const DURATION: Duration = Duration::from_millis(1500);

#[derive(Clone, Debug, PartialEq)]
pub struct Osd {
    pub glyph: &'static str,
    pub color: u32,
    /// Such as "Output"; may be empty.
    pub caption: &'static str,
    pub label: String,
    /// Level and muted; `None` when the device itself changed, which shows only the device.
    pub level: Option<(f32, bool)>,
}

#[derive(Default)]
pub struct Shown {
    osd: Option<Osd>,
    until: Option<Instant>,
    hovered: bool,
    fullscreen: bool,
}

impl Shown {
    /// Call it from [`Feature::sync_local`](super::Feature::sync_local).
    pub fn sync(&mut self, island: &IslandInfo<'_>) {
        self.fullscreen = island.monitor.is_some_and(|monitor| monitor.fullscreen);
    }

    /// Call it from [`Feature::elapse`](super::Feature::elapse).
    pub fn expire(&mut self) {
        self.osd = None;
        self.until = None;
    }
}

impl Local for Shown {
    fn popup(&self) -> bool {
        self.osd.is_some()
    }

    fn due(&self) -> Option<Instant> {
        self.until
    }

    fn pointer(&mut self, over: bool) {
        self.hovered = over;
    }

    /// Not over an island the pointer is on, whose bar shows the level already, unless the device
    /// itself changed or a fullscreen window hides the bar.
    fn osd(&mut self, osd: Osd) {
        if self.hovered && self.osd.is_none() && !self.fullscreen && osd.level.is_some() {
            return;
        }
        self.osd = Some(osd);
        self.until = Some(Instant::now() + DURATION);
    }
}

pub fn popup<'a>(p: &PanelCx<'a>) -> Option<PanelView> {
    let osd = p.local::<Shown>()?.osd.clone()?;
    Some(PanelView {
        header: Header::None,
        blocks: vec![Block::Meter(Meter {
            glyph: p.glyph(osd.glyph),
            caption: osd.caption.into(),
            label: osd.label.into(),
            level: osd.level,
            color: osd.color,
        })],
    })
}
