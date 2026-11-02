//! Lays a [`BarWidget`] out into sprites and the areas that answer the pointer, once, at the origin;
//! the island places the result, so a moving island reuses it.

use std::{collections::HashMap, time::Instant};

use super::{
    node::{BOLD, MEDIUM, SEMIBOLD},
    paint::{Framing, Painter},
    panel::{MorphText, Shown, edge_fade, noted},
    renderer::{Gpu, Sprite},
};
use crate::{
    bar::{GLYPH, ICON_GAP, METER_H, METER_W, WIDGET_H},
    bar_widget::{BarWidget, Emphasis, Frame, Part, Press, Scroll},
    components::{
        button::BADGE_OVERHANG,
        marquee::{GAP as MARQUEE_GAP, SPEED as MARQUEE_SPEED},
    },
    panel::SharedString,
    theme::{BASE, FONT, TEXT, TINT, TRACK_ALPHA, with_alpha},
    ui::{SMALL_SIZE, TEXT_SIZE, TITLE_SIZE},
};

const BADGE_H: f32 = 17.0;
const BADGE_TEXT: f32 = 10.0;

pub fn tinted(tint: u32, alpha: u8) -> [f32; 4] {
    color_alpha(with_alpha(tint, alpha))
}

/// From a color and alpha packed as `0xrrggbb`, premultiplied.
pub fn color(rgb: u32) -> [f32; 4] {
    let [_, r, g, b] = rgb.to_be_bytes().map(|channel| f32::from(channel) / 255.0);
    [r, g, b, 1.0]
}

/// From a color packed as `0xrrggbbaa`, premultiplied.
pub fn color_alpha(rgba: u32) -> [f32; 4] {
    let [r, g, b, a] = rgba.to_be_bytes().map(|channel| f32::from(channel) / 255.0);
    [r * a, g * a, b * a, a]
}

/// How an area shows the pointer on it.
#[derive(Clone, Copy)]
pub enum Tint {
    None,
    /// The widget's control, in from its sides by the amount.
    Control(f32),
    /// A pill this high across its middle.
    Pill(f32),
}

pub struct Hit {
    pub rect: [f32; 4],
    pub tint: Tint,
    pub tip: Option<SharedString>,
    pub on_click: Option<Press>,
    pub on_right: Option<Press>,
    pub on_middle: Option<Press>,
    pub on_scroll: Option<Scroll>,
    /// A press on it leaves a menu open, as what opens one does.
    pub holds_press: bool,
    /// It shows it can be pressed.
    pub pressable: bool,
}

#[derive(Default)]
pub struct Drawn {
    pub width: f32,
    pub sprites: Vec<Sprite>,
    /// Innermost last.
    pub hits: Vec<Hit>,
    /// Moves on its own, so needs drawing again after this long.
    pub moving: bool,
    pub morphs: Vec<MorphText>,
    /// Whether it is busy, and what the beam runs round.
    pub busy: Option<(bool, [f32; 4])>,
    /// What each text shows now, for the next time it is drawn.
    pub texts: HashMap<u64, Shown>,
    drawings: usize,
}

#[derive(Clone, Copy)]
struct Style {
    color: u32,
    small: bool,
}

pub struct Layout<'a> {
    pub painter: &'a mut Painter,
    pub gpu: &'a Gpu,
    pub scale: f32,
    pub hover_inset: f32,
    pub half: f32,
    pub grouped: bool,
    pub started: Instant,
    /// What each text showed the last time.
    pub before: &'a HashMap<u64, Shown>,
    /// Which widget on which output this is, for what it keeps in the atlas.
    pub owner: String,
}

fn weight(small: bool) -> u16 {
    if small { SEMIBOLD } else { MEDIUM }
}

fn size(small: bool) -> f32 {
    if small { SMALL_SIZE } else { TEXT_SIZE }
}

impl Layout<'_> {
    fn line(&mut self, text: &str, size: f32, weight: u16) -> std::rc::Rc<super::paint::Line> {
        self.painter.line(text, size, weight, self.scale)
    }

    fn count_width(&mut self) -> f32 {
        let wide = self.line("88", SMALL_SIZE, SEMIBOLD).width;
        wide.max(self.line("!", SMALL_SIZE, SEMIBOLD).width)
    }

    fn text_style(emphasis: &Emphasis) -> (f32, u16) {
        match emphasis {
            Emphasis::Title => (TITLE_SIZE, BOLD),
            Emphasis::Small => (SMALL_SIZE, MEDIUM),
        }
    }

    fn width_of(&mut self, part: &Part, style: Style) -> f32 {
        match part {
            Part::Glyph(..) => GLYPH,
            Part::Picture(_, side) => *side,
            Part::Meter { .. } => METER_W,
            Part::Busy(_) | Part::Underline { .. } => 0.0,
            Part::Text { text, emphasis, .. } => {
                let (size, weight) = Self::text_style(emphasis);
                self.line(text, size, weight).width.ceil()
            }
            Part::Count(text) => {
                let least = self.count_width();
                least.max(self.line(text, SMALL_SIZE, SEMIBOLD).width.ceil())
            }
            Part::Marquee { slot, .. } => *slot,
            Part::Plain(text) => self
                .line(text, size(style.small), weight(style.small))
                .width
                .ceil(),
            Part::Fixed(part) => self.width_of(part, style),
            Part::Cluster { parts, .. } => {
                let sum: f32 = parts.iter().map(|part| self.width_of(part, style)).sum();
                sum + ICON_GAP * parts.len().saturating_sub(1) as f32
            }
            Part::Badged { part, badge, .. } => {
                self.width_of(part, style) + if badge.is_some() { BADGE_OVERHANG } else { 0.0 }
            }
            Part::Pill { width, .. } => *width,
            Part::Widget(widget) => self.widget_width(widget, style),
            Part::Drawing(drawing) => drawing.side,
        }
    }

    fn flows(part: &Part) -> bool {
        !matches!(part, Part::Busy(_) | Part::Underline { .. })
    }

    fn content_width(&mut self, parts: &[Part], gap: f32, style: Style) -> f32 {
        let mut sum = 0.0;
        let mut flowing = 0;
        for part in parts {
            if Self::flows(part) {
                sum += self.width_of(part, style);
                flowing += 1;
            }
        }
        sum + gap * (flowing as f32 - 1.0).max(0.0)
    }

    fn padding(&self, widget: &BarWidget) -> f32 {
        match widget.frame {
            Frame::Row | Frame::Spot { .. } => widget.padding.unwrap_or(0.0),
            Frame::Control | Frame::Bare => widget.padding.unwrap_or(self.half),
        }
    }

    fn style_of(widget: &BarWidget, style: Style) -> Style {
        Style {
            color: widget.color.unwrap_or(style.color),
            small: widget.small || style.small,
        }
    }

    fn widget_width(&mut self, widget: &BarWidget, style: Style) -> f32 {
        let style = Self::style_of(widget, style);
        match widget.width {
            Some(width) => width,
            None => {
                let content = self.content_width(&widget.parts, widget.gap.unwrap_or(0.0), style);
                content + 2.0 * self.padding(widget)
            }
        }
    }

    pub fn draw(&mut self, widget: &BarWidget) -> Drawn {
        let style = Style {
            color: TEXT,
            small: false,
        };
        let mut drawn = Drawn {
            width: self.widget_width(widget, style),
            ..Drawn::default()
        };
        self.place(widget, 0.0, style, &mut drawn);
        drawn
    }

    fn place(&mut self, widget: &BarWidget, x: f32, style: Style, drawn: &mut Drawn) {
        let style = Self::style_of(widget, style);
        let width = self.widget_width(widget, style);
        let gap = widget.gap.unwrap_or(0.0);
        let padding = self.padding(widget);
        let content = self.content_width(&widget.parts, gap, style);
        let mut at = if widget.start {
            x + padding
        } else {
            x + (width - content) / 2.0
        };
        let tint = match widget.frame {
            Frame::Control if !self.grouped => Tint::Control(self.hover_inset),
            Frame::Spot { tint } => Tint::Pill(tint),
            _ => Tint::None,
        };
        drawn.hits.push(Hit {
            rect: [x, 0.0, width, WIDGET_H],
            tint,
            tip: widget.tip.clone(),
            on_click: widget.on_click.clone(),
            on_right: widget.on_right.clone(),
            on_middle: widget.on_middle.clone(),
            on_scroll: widget.on_scroll.clone(),
            holds_press: widget.holds_press,
            pressable: matches!(widget.frame, Frame::Control),
        });
        let first = drawn.sprites.len();
        for part in &widget.parts {
            if Self::flows(part) {
                let part_width = self.width_of(part, style);
                self.part(part, at, style, drawn);
                at += part_width + gap;
            } else {
                self.overlay(part, x, width, drawn);
            }
        }
        for sprite in &mut drawn.sprites[first..] {
            if let Some(opacity) = widget.opacity {
                *sprite = sprite.faded(opacity);
            }
            if widget.clip {
                *sprite = sprite.within([x, 0.0, x + width, WIDGET_H]);
            }
        }
    }

    fn overlay(&mut self, part: &Part, x: f32, width: f32, drawn: &mut Drawn) {
        match part {
            Part::Busy(busy) => {
                drawn.busy = Some((
                    *busy,
                    [
                        x + self.hover_inset,
                        0.0,
                        width - 2.0 * self.hover_inset,
                        WIDGET_H,
                    ],
                ));
            }
            Part::Underline {
                width: line,
                height,
                lift,
                opacity,
                color: tint,
            } => {
                drawn.moving = true;
                drawn.sprites.push(
                    Sprite::shape(
                        [
                            x + (width - line) / 2.0,
                            WIDGET_H - lift - height,
                            *line,
                            *height,
                        ],
                        color(*tint),
                        *height,
                        2.0,
                    )
                    .faded(*opacity),
                );
            }
            _ => {}
        }
    }

    fn middle(height: f32) -> f32 {
        (WIDGET_H - height) / 2.0
    }

    fn plain(&mut self, text: &str, size: f32, weight: u16, x: f32, tint: u32, drawn: &mut Drawn) {
        let line = self.line(text, size, weight);
        let top = Self::middle(size * 1.2);
        self.painter.text(
            self.gpu,
            &line,
            (x, top),
            color(tint),
            self.scale,
            &mut drawn.sprites,
        );
    }

    /// As it comes, or turning into it from what it showed before.
    fn morphing(
        &mut self,
        text: &SharedString,
        (size, weight): (f32, u16),
        x: f32,
        tint: u32,
        drawn: &mut Drawn,
    ) {
        let key = drawn.texts.len() as u64;
        let shown = noted(self.before.get(&key), text);
        let from = shown.from.clone();
        drawn.texts.insert(key, shown);
        let Some((old, since)) = from else {
            self.plain(text, size, weight, x, tint, drawn);
            return;
        };
        let line = self.line(text, size, weight);
        let top = Self::middle(size * 1.2);
        let (start, slots, end) = self.painter.morph_parts(
            self.gpu,
            FONT,
            (&old, text),
            (size, weight),
            color(tint),
            top,
            self.scale,
        );
        let mut rest = Vec::new();
        self.painter.text(
            self.gpu,
            &line,
            (x, top),
            color(tint),
            self.scale,
            &mut rest,
        );
        drawn.morphs.push(MorphText {
            rect: [x, 0.0, line.width, WIDGET_H],
            align: 0.0,
            since,
            start,
            slots,
            end,
            rest,
            clip: None,
            fade: 1.0,
        });
    }

    fn part(&mut self, part: &Part, x: f32, style: Style, drawn: &mut Drawn) {
        match part {
            Part::Glyph(path, tint) => self.painter.glyph_icon(
                self.gpu,
                path,
                [x, Self::middle(GLYPH), GLYPH, GLYPH],
                color(*tint),
                self.scale,
                &mut drawn.sprites,
            ),
            Part::Picture(picture, side) => self.painter.picture(
                self.gpu,
                picture,
                Framing::Contain,
                [x, Self::middle(*side), *side, *side],
                self.scale,
                &mut drawn.sprites,
            ),
            Part::Meter { level, color: tint } => {
                let top = Self::middle(METER_H);
                drawn.sprites.push(Sprite::shape(
                    [x, top, METER_W, METER_H],
                    tinted(TINT, TRACK_ALPHA),
                    METER_W,
                    2.0,
                ));
                let level = level.unwrap_or(0.0).clamp(0.0, 1.0);
                if level > 0.0 {
                    let height = (METER_H * level).max(METER_W);
                    drawn.sprites.push(Sprite::shape(
                        [x, top + METER_H - height, METER_W, height],
                        color(*tint),
                        METER_W,
                        2.0,
                    ));
                }
            }
            Part::Text { text, emphasis, .. } => {
                let (size, weight) = Self::text_style(emphasis);
                self.morphing(text, (size, weight), x, style.color, drawn);
            }
            Part::Count(text) => {
                self.morphing(text, (SMALL_SIZE, SEMIBOLD), x, style.color, drawn);
            }
            Part::Plain(text) => {
                self.plain(
                    text,
                    size(style.small),
                    weight(style.small),
                    x,
                    style.color,
                    drawn,
                );
            }
            Part::Marquee { text, slot } => self.marquee(text, *slot, x, style.color, drawn),
            Part::Fixed(part) => self.part(part, x, style, drawn),
            Part::Cluster { color: tint, parts } => {
                let style = Style {
                    color: *tint,
                    ..style
                };
                let mut at = x;
                for part in parts {
                    let width = self.width_of(part, style);
                    self.part(part, at, style, drawn);
                    at += width + ICON_GAP;
                }
            }
            Part::Badged { part, badge, .. } => {
                let width = self.width_of(part, style);
                self.part(part, x, style, drawn);
                if let Some((text, tint)) = badge {
                    let right = x + width + BADGE_OVERHANG;
                    let top = Self::middle(GLYPH) - BADGE_OVERHANG;
                    drawn.sprites.push(Sprite::shape(
                        [right - BADGE_H, top, BADGE_H, BADGE_H],
                        color(BASE),
                        6.0,
                        2.0,
                    ));
                    drawn.sprites.push(Sprite::shape(
                        [
                            right - BADGE_H + 2.0,
                            top + 2.0,
                            BADGE_H - 4.0,
                            BADGE_H - 4.0,
                        ],
                        color(*tint),
                        4.0,
                        2.0,
                    ));
                    let line = self.line(text, BADGE_TEXT, BOLD);
                    let at = (
                        right - (BADGE_H + line.width) / 2.0,
                        top + (BADGE_H - BADGE_TEXT * 1.2) / 2.0,
                    );
                    self.painter.text(
                        self.gpu,
                        &line,
                        at,
                        color(BASE),
                        self.scale,
                        &mut drawn.sprites,
                    );
                }
            }
            Part::Pill {
                width,
                height,
                color: tint,
            } => drawn.sprites.push(Sprite::shape(
                [x, Self::middle(*height), *width, *height],
                color(*tint),
                *height,
                2.0,
            )),
            Part::Widget(widget) => self.place(widget, x, style, drawn),
            Part::Drawing(drawing) => {
                let key = format!("{}-{}", self.owner, drawn.drawings);
                drawn.drawings += 1;
                self.painter.drawing(
                    self.gpu,
                    &key,
                    drawing,
                    (x, Self::middle(drawing.side)),
                    self.scale,
                    &mut drawn.sprites,
                );
            }
            Part::Busy(_) | Part::Underline { .. } => {}
        }
    }

    fn marquee(&mut self, text: &str, slot: f32, x: f32, tint: u32, drawn: &mut Drawn) {
        let line = self.line(text, SMALL_SIZE, SEMIBOLD);
        let top = Self::middle(SMALL_SIZE * 1.2);
        let clip = [x, 0.0, x + slot, WIDGET_H];
        let first = drawn.sprites.len();
        if line.width <= slot {
            self.painter.text(
                self.gpu,
                &line,
                (x, top),
                color(tint),
                self.scale,
                &mut drawn.sprites,
            );
        } else {
            drawn.moving = true;
            let lap = line.width + MARQUEE_GAP;
            let moved = (self.started.elapsed().as_secs_f32() * MARQUEE_SPEED) % lap;
            for at in [x - moved, x - moved + lap] {
                self.painter.text(
                    self.gpu,
                    &line,
                    (at, top),
                    color(tint),
                    self.scale,
                    &mut drawn.sprites,
                );
            }
        }
        let scrolling = line.width > slot;
        for sprite in &mut drawn.sprites[first..] {
            if scrolling {
                let at = sprite.rect[0] - x + sprite.rect[2] / 2.0;
                *sprite = sprite.faded(edge_fade(at, slot));
            }
            *sprite = sprite.within(clip);
        }
    }
}
