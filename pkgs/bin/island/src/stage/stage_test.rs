use std::{rc::Rc, time::Instant};

use super::*;
use crate::{
    features::{Features, Local},
    stage::{COLLAPSED_H, COLLAPSED_LAYER, Grab, Mode, Morph, Panel, Shape},
};

#[derive(Default)]
struct Part {
    popup: bool,
    closed: bool,
}

impl Local for Part {
    fn popup(&self) -> bool {
        self.popup
    }

    fn panel_closed(&mut self) {
        self.closed = true;
    }
}

fn stage() -> Stage {
    Stage::new(Rc::default(), Rc::new(Features::default()), None)
}

#[test]
fn only_what_an_open_panel_reads_moves_it() {
    let mut stage = stage();
    assert!(!stage.shows_local("ai"));
    stage.mode = Mode::Open(Panel::System);
    assert!(!stage.shows_local("ai"));
    stage.mode = Mode::Open(Panel::Dashboard);
    assert!(stage.shows_local("ai"));
    stage.mode = Mode::Open(Panel::Feature("audio"));
    assert!(stage.shows_local("audio"));
    assert!(!stage.shows_local("ai"));
    stage.lent.borrow_mut().insert("ai".to_owned());
    assert!(stage.shows_local("ai"));
}

#[test]
fn collapsing_shows_waiting_popups_instead() {
    let mut stage = stage();
    assert_eq!(stage.resolved(Mode::Collapsed), Mode::Collapsed);
    stage.locals.push((
        "part",
        Box::new(Part {
            popup: true,
            closed: false,
        }),
    ));
    assert_eq!(stage.resolved(Mode::Collapsed), Mode::Open(Panel::Popups));
    assert_eq!(
        stage.resolved(Mode::Open(Panel::System)),
        Mode::Open(Panel::System)
    );
}

#[test]
fn a_fullscreen_window_hides_the_bar_only_while_collapsed() {
    let mut stage = stage();
    stage.fullscreen = true;
    assert!(stage.hidden());
    assert_eq!(stage.active_layer(), None);
    assert_eq!(
        stage.desired_shape(Some(500.0)),
        Some(Shape { w: 0.0, h: 0.0 })
    );
    stage.mode = Mode::Open(Panel::System);
    assert!(!stage.hidden());
    assert_eq!(stage.active_layer(), Some(Panel::System.layer()));
}

#[test]
fn the_bar_grows_under_the_pointer() {
    let mut stage = stage();
    let rest = stage.desired_shape(Some(500.0)).unwrap();
    assert_eq!(
        rest,
        Shape {
            w: 500.0,
            h: COLLAPSED_H
        }
    );
    stage.set_pointer(true);
    assert!(stage.desired_shape(Some(500.0)).unwrap().w > rest.w);
    assert_eq!(stage.desired_shape(None), None);
}

#[test]
fn an_open_panel_takes_the_shape_it_measured() {
    let mut stage = stage();
    stage.mode = Mode::Open(Panel::System);
    assert_eq!(stage.desired_shape(Some(500.0)), None);
    let shape = Shape { w: 800.0, h: 600.0 };
    stage.morph.frozen[Panel::System.layer()] = Some((1, shape));
    assert_eq!(stage.desired_shape(Some(500.0)), Some(shape));
}

#[test]
fn clicks_are_grabbed_only_for_an_open_menu() {
    let stage = stage();
    assert_eq!(stage.grab(false), Grab::None);
    assert_eq!(stage.grab(true), Grab::Clicks);
}

#[test]
fn the_first_shape_is_taken_at_once_and_later_ones_animate() {
    let mut morph = Morph::new();
    assert!(!morph.fit(None));
    assert!(morph.fit(Some(Shape { w: 500.0, h: 38.0 })));
    assert_eq!(morph.w.value(), 500.0);
    assert!(!morph.fit(Some(Shape { w: 500.2, h: 38.0 })));
    assert!(morph.fit(Some(Shape { w: 800.0, h: 600.0 })));
    assert_eq!(morph.w.value(), 500.0);
    assert_eq!(morph.w.target(), 800.0);
}

#[test]
fn only_the_active_layer_fades_in() {
    let mut morph = Morph::new();
    let layer = Panel::System.layer();
    morph.fade(Some(layer));
    assert_eq!(morph.layers[layer].target(), 1.0);
    assert_eq!(morph.layers[COLLAPSED_LAYER].target(), 0.0);
}

#[test]
fn a_passing_panel_comes_in_from_the_other_side() {
    let mut morph = Morph::new();
    let (out, into) = (Panel::System.layer(), Panel::Dashboard.layer());
    morph.slide(out, into, true);
    assert_eq!(morph.slides[out].target(), -1.0);
    assert_eq!(morph.slides[into].value(), 1.0);
    assert_eq!(morph.slides[into].target(), 0.0);
    assert_eq!(morph.layers[into].value(), 1.0);
}

#[test]
fn a_closing_panel_is_told_once_everything_stops() {
    let mut stage = stage();
    stage.locals.push(("part", Box::<Part>::default()));
    stage.closing = Some("part");
    stage.finish_closing();
    assert_eq!(stage.closing, None);
    let part = stage.local("part").unwrap().as_any();
    assert!(part.downcast_ref::<Part>().unwrap().closed);
}

#[test]
fn a_settled_stage_stays_still() {
    let mut stage = stage();
    stage.advance(0.016, Instant::now());
    assert!(stage.still(|local| local.is_settled()));
}
