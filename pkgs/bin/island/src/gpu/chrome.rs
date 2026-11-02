//! The tabs and the close button along a panel's top.

use super::{
    blocks::ROW_RADIUS,
    node::SEMIBOLD,
    paint::Painter,
    panel::mixed,
    renderer::{Gpu, Sprite, inside},
    widget::{color, tinted},
};
use crate::{
    bar::{
        DIVIDER_ALPHA, DIVIDER_GAP, DIVIDER_H, GLYPH_X, LABEL_GAP, LABEL_SIZE, PICKED_ALPHA,
        TAB_GAP, TAB_GLYPH, TabStrip, VIEWS_ALPHA, VIEWS_BORDER_ALPHA, tab_list,
    },
    glyphs::Glyphs,
    stage::Panel,
    theme::{ACCENT, HOVER_ALPHA, ON_ACCENT, OVERLAY2, SUBTEXT1, TINT},
    ui::{CONTROL_RADIUS, HEADER_H, STRIP_PAD},
};

const STRIP_ALPHA: u8 = 0x0d;
const STRIP_BORDER_ALPHA: u8 = 0x10;

#[derive(Clone, Copy)]
pub enum Action {
    Open(Panel),
    Close,
}

pub struct Hit {
    pub rect: [f32; 4],
    pub action: Action,
    pub tip: &'static str,
}

pub struct Chrome<'a> {
    pub strip: &'a TabStrip,
    pub glyphs: &'a Glyphs,
    pub painter: &'a mut Painter,
    pub gpu: &'a Gpu,
    pub scale: f32,
    pub pointer: Option<(f32, f32)>,
}

impl Chrome<'_> {
    fn hovered(&self, rect: [f32; 4]) -> bool {
        self.pointer.is_some_and(|pointer| inside(rect, pointer))
    }

    /// With the strip's left at `x` and top at `y`, and the close button ending at `right`.
    pub fn draw(&mut self, x: f32, y: f32, right: f32) -> (Vec<Sprite>, Vec<Hit>) {
        let mut out = Vec::with_capacity(48);
        let mut hits = Vec::new();
        let (places, width) = self.strip.places();
        let fan = self.strip.fan();
        let tabs = tab_list();
        let strip = [
            x - STRIP_PAD,
            y - STRIP_PAD,
            width + 2.0 * STRIP_PAD,
            HEADER_H + 2.0 * STRIP_PAD,
        ];
        let radius = ROW_RADIUS + STRIP_PAD;
        out.push(Sprite::shape(strip, tinted(TINT, STRIP_ALPHA), radius, 2.0));
        out.push(Sprite::ring(
            strip,
            tinted(TINT, STRIP_BORDER_ALPHA),
            radius,
            2.0,
        ));
        let views: Vec<(f32, f32)> = tabs
            .iter()
            .zip(&places)
            .filter(|(tab, _)| tab.view)
            .map(|(_, place)| *place)
            .collect();
        if let (Some(first), Some(last)) = (views.first(), views.last())
            && fan > 0.01
        {
            let group = [x + first.0, y, last.0 + last.1 - first.0, HEADER_H];
            out.push(Sprite::shape(group, tinted(ACCENT, VIEWS_ALPHA), ROW_RADIUS, 2.0).faded(fan));
            out.push(
                Sprite::ring(group, tinted(ACCENT, VIEWS_BORDER_ALPHA), ROW_RADIUS, 2.0).faded(fan),
            );
            out.push(
                Sprite::shape(
                    [
                        x + first.0 + first.1 + (TAB_GAP + fan * DIVIDER_GAP) / 2.0,
                        y + (HEADER_H - DIVIDER_H) / 2.0,
                        1.0,
                        DIVIDER_H,
                    ],
                    tinted(ACCENT, DIVIDER_ALPHA),
                    0.0,
                    2.0,
                )
                .faded(fan),
            );
        }
        let picked = mixed(tinted(ACCENT, PICKED_ALPHA), color(ACCENT), fan);
        let (pill_x, pill_w) = self.strip.pill();
        out.push(Sprite::shape(
            [x + pill_x, y, pill_w.max(0.0), HEADER_H],
            picked,
            ROW_RADIUS,
            2.0,
        ));
        let in_views = tabs
            .iter()
            .find(|tab| tab.panel == self.strip.picked())
            .is_some_and(|tab| tab.view);
        for (index, (tab, (at, w))) in tabs.iter().zip(places).enumerate() {
            let selected = self.strip.picked() == tab.panel;
            let shown = if tab.view && tab.panel != Panel::Dashboard {
                fan
            } else {
                1.0
            };
            if w < 1.0 || shown < 0.01 {
                continue;
            }
            let rect = [x + at, y, w, HEADER_H];
            let clip = [rect[0], rect[1], rect[0] + w, rect[1] + HEADER_H];
            let first = out.len();
            if !selected && self.hovered(rect) {
                out.push(Sprite::shape(
                    rect,
                    tinted(ACCENT, HOVER_ALPHA),
                    ROW_RADIUS,
                    2.0,
                ));
            }
            let tint = match (selected, tab.view) {
                (true, true) => ON_ACCENT,
                (true, false) => ACCENT,
                (false, _) => OVERLAY2,
            };
            let key = format!("tab-{}", tab.panel.name());
            if let Some(path) = self.glyphs.get(&key) {
                self.painter.glyph_icon(
                    self.gpu,
                    path,
                    [
                        rect[0] + GLYPH_X,
                        y + (HEADER_H - TAB_GLYPH) / 2.0,
                        TAB_GLYPH,
                        TAB_GLYPH,
                    ],
                    color(tint),
                    self.scale,
                    &mut out,
                );
            }
            let label = self.strip.label(index);
            if label > 0.01 {
                let line = self
                    .painter
                    .line(tab.title, LABEL_SIZE, SEMIBOLD, self.scale);
                let start = out.len();
                self.painter.text(
                    self.gpu,
                    &line,
                    (
                        rect[0] + GLYPH_X + TAB_GLYPH + LABEL_GAP,
                        y + (HEADER_H - LABEL_SIZE * 1.2) / 2.0,
                    ),
                    color(tint),
                    self.scale,
                    &mut out,
                );
                for sprite in &mut out[start..] {
                    *sprite = sprite.faded(label);
                }
            }
            for sprite in &mut out[first..] {
                *sprite = sprite.within(clip).faded(shown);
            }
            let target = if tab.panel == Panel::Dashboard && !in_views {
                self.strip.last_view()
            } else {
                tab.panel
            };
            hits.push(Hit {
                rect,
                action: Action::Open(target),
                tip: tab.title,
            });
        }
        let close = [right - HEADER_H, y, HEADER_H, HEADER_H];
        let hovered = self.hovered(close);
        if hovered {
            out.push(Sprite::shape(
                close,
                tinted(ACCENT, HOVER_ALPHA),
                CONTROL_RADIUS,
                2.0,
            ));
        }
        let line = self.painter.line("✕", 14.0, 500, self.scale);
        self.painter.text(
            self.gpu,
            &line,
            (
                close[0] + (HEADER_H - line.width) / 2.0,
                y + (HEADER_H - 14.0 * 1.2) / 2.0,
            ),
            color(if hovered {
                crate::theme::TEXT
            } else {
                SUBTEXT1
            }),
            self.scale,
            &mut out,
        );
        hits.push(Hit {
            rect: close,
            action: Action::Close,
            tip: "",
        });
        (out, hits)
    }
}
