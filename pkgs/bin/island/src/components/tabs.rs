//! A row of tabs with a pill sliding to the selected one, each named by its label or its glyph;
//! or, contained in a strip of their own, labelled tabs with a count beside each label.

use std::{path::PathBuf, rc::Rc};

use crate::panel::SharedString;

use crate::{panel::Ui, spring::SpringConfig, ui::STRIP_PAD};

pub const PILL_SPRING: SpringConfig = SpringConfig::new(420.0, 41.0, 1.0);

pub type Select = Rc<dyn Fn(usize, &mut dyn Ui)>;

/// One tab: its label, shown unless it has a glyph to show instead.
pub struct TabLabel {
    pub label: SharedString,
    pub glyph: Option<PathBuf>,
    /// Smaller, after the label in a strip, such as how many the view holds.
    pub count: Option<SharedString>,
}

const STRIP_TAB_H: f32 = 28.0;
/// A strip of tabs, edge to edge, for what sits beside it to match.
pub const STRIP_H: f32 = STRIP_TAB_H + 2.0 * STRIP_PAD;
