use std::cell::Cell;

use super::notches;

#[test]
fn parts_of_a_notch_add_up_to_a_step() {
    let scrolled = Cell::new(0.0);
    assert_eq!(notches(&scrolled, 0.6), 0);
    assert_eq!(notches(&scrolled, 0.6), 1);
    assert!((scrolled.get() - 0.2).abs() < 1e-6);
}

#[test]
fn scrolling_back_takes_whole_steps_the_other_way() {
    let scrolled = Cell::new(0.0);
    assert_eq!(notches(&scrolled, -2.5), -2);
    assert!((scrolled.get() + 0.5).abs() < 1e-6);
}
