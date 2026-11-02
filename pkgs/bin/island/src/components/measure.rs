//! Reports an element's drawn size, for layouts that size themselves to their content.

use std::{cell::Cell, rc::Rc};

use gpui::{IntoElement, Pixels, Size, canvas, prelude::*};

pub type Measured = Rc<Cell<Option<Size<Pixels>>>>;

pub fn measured() -> Measured {
    Rc::new(Cell::new(None))
}

pub fn probe(cell: &Measured) -> impl IntoElement {
    let cell = cell.clone();
    canvas(
        move |bounds, window, _| {
            if cell.get() != Some(bounds.size) {
                cell.set(Some(bounds.size));
                window.refresh();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// What a feature measures its text with, before drawing.
pub struct Measure<'a>(&'a dyn Fn(&str, f32, f32) -> usize);

impl<'a> Measure<'a> {
    /// `lines` says how many lines a text takes at a size in a width.
    pub fn new(lines: &'a dyn Fn(&str, f32, f32) -> usize) -> Self {
        Self(lines)
    }

    pub fn wrapped_lines(&self, body: &str, size: f32, width: f32) -> usize {
        (self.0)(body, size, width)
    }
}
