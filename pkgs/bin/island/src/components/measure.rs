//! Reports an element's drawn size, for layouts that size themselves to their content.

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
