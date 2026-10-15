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
