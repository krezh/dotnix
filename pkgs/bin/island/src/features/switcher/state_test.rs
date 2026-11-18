use std::sync::Arc;

use super::{Local, Previews};
use crate::{
    features::Local as _,
    wm::{Place, Window},
};

fn window(address: &str, class: &str) -> Window {
    Window {
        address: address.to_owned(),
        id: address.to_owned(),
        class: class.to_owned(),
        title: format!("{class} {address}"),
        place: Place::Workspace(1),
        monitor: "DP-1".to_owned(),
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        floating: false,
        urgent: false,
    }
}

fn opened(view: &str) -> Local {
    let mut local = Local::default();
    let previews = Previews {
        windows: Arc::new(vec![
            window("a", "kitty"),
            window("b", "zed"),
            window("c", "kitty"),
            window("d", "zen"),
        ]),
        ..Previews::default()
    };
    local.sync(&previews, None);
    local.panel_opened();
    local.show(view);
    local
}

fn picked(local: &Local) -> &str {
    &local.picked().expect("a window is picked").address
}

#[test]
fn opens_on_the_window_used_before_the_focused_one() {
    assert_eq!(picked(&opened("next")), "b");
}

#[test]
fn opens_backwards_on_the_last_window() {
    assert_eq!(picked(&opened("prev")), "d");
}

#[test]
fn steps_round_past_the_last_window() {
    let mut local = opened("next");
    for _ in 0..3 {
        local.show("next");
    }
    assert_eq!(picked(&local), "a");
}

#[test]
fn keeps_to_the_focused_app_and_back() {
    let mut local = opened("same");
    let shown: Vec<&str> = local
        .showing()
        .map(|window| window.address.as_str())
        .collect();
    assert_eq!(shown, ["a", "c"]);
    assert_eq!(picked(&local), "c");
    local.show("same");
    assert_eq!(local.shown_count(), 4);
}

#[test]
fn a_closed_window_leaves_the_pick_on_a_window() {
    let mut local = opened("prev");
    local.forget("d");
    assert_eq!(picked(&local), "c");
}
