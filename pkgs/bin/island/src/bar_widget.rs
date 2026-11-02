//! A bar widget as data: what it shows and what answers the pointer. The frontend draws it.

use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{
    drawing::Drawing,
    panel::{SharedString, Ui},
    picture::Picture,
};

/// Where the pointer is across the island's surface.
#[derive(Clone, Copy)]
pub struct Pointer {
    pub x: f32,
}

pub type Press = Rc<dyn Fn(Pointer, &mut dyn Ui)>;

/// Called with how far it scrolled, in notches and parts of one.
pub type Scroll = Rc<dyn Fn(f32, &mut dyn Ui)>;

/// Takes the whole notches out of what has scrolled so far, leaving the rest for the next.
pub fn notches(scrolled: &Cell<f32>, delta: f32) -> i32 {
    let total = scrolled.get() + delta;
    let steps = total.trunc();
    scrolled.set(total - steps);
    steps as i32
}

pub enum Emphasis {
    Title,
    Small,
}

pub enum Frame {
    /// Clickable as a whole, showing the hover behind everything.
    Control,
    /// Several clickable parts that each show the hover themselves.
    Bare,
    /// Only lays out what is in it.
    Row,
    /// One clickable part of a [`Frame::Bare`], whose hover is a pill `tint` high.
    Spot { tint: f32 },
}

pub struct Gauge<'a> {
    pub glyph: Option<&'a Path>,
    pub glyph_color: u32,
    pub busy: bool,
    pub level: Option<f32>,
    pub level_color: u32,
}

pub enum Part {
    Glyph(PathBuf, u32),
    Picture(Picture, f32),
    Meter {
        level: Option<f32>,
        color: u32,
    },
    /// The moving border, while `true`.
    Busy(bool),
    /// Changes in place when the text does.
    Text {
        id: &'static str,
        text: SharedString,
        emphasis: Emphasis,
    },
    /// As wide as the widest count ("88"), so the widget keeps its width as it changes.
    Count(SharedString),
    /// Takes the room left, and scrolls when the text does not fit `slot`.
    Marquee {
        text: SharedString,
        slot: f32,
    },
    Plain(SharedString),
    /// Keeps its size when what sits beside it runs out of room.
    Fixed(Box<Part>),
    /// Marks and short text side by side, in `color`.
    Cluster {
        color: u32,
        parts: Vec<Part>,
    },
    /// A count on the part's corner, in the color.
    Badged {
        id: &'static str,
        part: Box<Part>,
        badge: Option<(SharedString, u32)>,
    },
    Pill {
        width: f32,
        height: f32,
        color: u32,
    },
    /// A pill laid over the bottom of the widget, `lift` above its edge.
    Underline {
        width: f32,
        height: f32,
        lift: f32,
        opacity: f32,
        color: u32,
    },
    Widget(BarWidget),
    Drawing(Drawing),
}

impl Part {
    pub fn glyph(path: &Path, color: u32) -> Self {
        Self::Glyph(path.to_path_buf(), color)
    }

    pub fn text(id: &'static str, text: impl Into<SharedString>, emphasis: Emphasis) -> Self {
        Self::Text {
            id,
            text: text.into(),
            emphasis,
        }
    }

    pub fn plain(text: impl Into<SharedString>) -> Self {
        Self::Plain(text.into())
    }

    pub fn count(text: impl Into<SharedString>) -> Self {
        Self::Count(text.into())
    }

    pub fn marquee(text: impl Into<SharedString>, slot: f32) -> Self {
        Self::Marquee {
            text: text.into(),
            slot,
        }
    }

    pub fn fixed(self) -> Self {
        Self::Fixed(Box::new(self))
    }
}

pub struct BarWidget {
    pub id: SharedString,
    pub frame: Frame,
    pub parts: Vec<Part>,
    pub tip: Option<SharedString>,
    pub gap: Option<f32>,
    /// Short, strong text.
    pub small: bool,
    pub color: Option<u32>,
    pub width: Option<f32>,
    /// Sits at its start rather than its middle.
    pub start: bool,
    pub padding: Option<f32>,
    pub clip: bool,
    pub opacity: Option<f32>,
    pub on_click: Option<Press>,
    pub on_right: Option<Press>,
    pub on_middle: Option<Press>,
    pub on_scroll: Option<Scroll>,
    /// Keeps a left press from reaching what lies under it, as the click opens a menu.
    pub holds_press: bool,
}

impl BarWidget {
    pub fn new(id: impl Into<SharedString>, frame: Frame) -> Self {
        Self {
            id: id.into(),
            frame,
            parts: Vec::new(),
            tip: None,
            gap: None,
            small: false,
            color: None,
            width: None,
            start: false,
            padding: None,
            clip: false,
            opacity: None,
            on_click: None,
            on_right: None,
            on_middle: None,
            on_scroll: None,
            holds_press: false,
        }
    }

    pub fn part(mut self, part: Part) -> Self {
        self.parts.push(part);
        self
    }

    pub fn parts(mut self, parts: impl IntoIterator<Item = Part>) -> Self {
        self.parts.extend(parts);
        self
    }

    /// Nothing without a path, as a glyph the theme lacks.
    pub fn glyph(self, path: Option<&Path>, color: u32) -> Self {
        self.parts(path.map(|path| Part::glyph(path, color)))
    }

    pub fn tip(mut self, text: impl Into<SharedString>) -> Self {
        self.tip = Some(text.into());
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn start(mut self) -> Self {
        self.start = true;
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn clip(mut self) -> Self {
        self.clip = true;
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity);
        self
    }

    pub fn on_click(mut self, press: Press) -> Self {
        self.on_click = Some(press);
        self
    }

    pub fn on_right(mut self, press: Press) -> Self {
        self.on_right = Some(press);
        self
    }

    pub fn on_middle(mut self, press: Press) -> Self {
        self.on_middle = Some(press);
        self
    }

    pub fn on_scroll(mut self, scroll: Scroll) -> Self {
        self.on_scroll = Some(scroll);
        self
    }

    pub fn holds_press(mut self) -> Self {
        self.holds_press = true;
        self
    }
}

#[cfg(test)]
#[path = "bar_widget_test.rs"]
mod tests;
