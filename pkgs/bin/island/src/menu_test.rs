use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{Bounds, point, px, size};

use super::{Menu, MenuNode, Pick};

fn noop() -> Pick {
    Rc::new(|_, _, _| {})
}

fn item(id: i32, label: &str) -> MenuNode {
    MenuNode {
        id,
        label: label.to_owned(),
        enabled: true,
        separator: false,
        toggle: None,
        children: Vec::new(),
    }
}

fn root(children: Vec<MenuNode>) -> MenuNode {
    MenuNode {
        id: 0,
        label: String::new(),
        enabled: true,
        separator: false,
        toggle: None,
        children,
    }
}

fn measure(menu: &mut Menu, w: f32, h: f32) {
    menu.pending.set(Some(menu.key()));
    menu.size.set(Some(size(px(w), px(h))));
    assert!(menu.collect());
}

fn settle(menu: &mut Menu) {
    let mut now = Instant::now();
    for _ in 0..200 {
        if menu.is_settled() {
            break;
        }
        now += Duration::from_millis(16);
        menu.advance(0.016, now);
    }
    assert!(menu.is_settled());
}

#[test]
fn replacing_an_open_menu_slides_its_anchor_and_crossfades_into_the_new_size() {
    let body = Bounds::new(point(px(200.0), px(10.0)), size(px(600.0), px(38.0)));
    let mut menu = Menu::new(
        "first".to_owned(),
        root(vec![item(1, "One")]),
        noop(),
        350.0,
    );
    assert!(!menu.is_settled());
    measure(&mut menu, 180.0, 90.0);
    settle(&mut menu);

    let first_hang = menu.hang(body, 19.0, 1.0);
    assert_eq!(f32::from(first_hang.bounds.origin.x), 260.0);
    assert_eq!(f32::from(first_hang.bounds.size.width), 180.0);
    assert_eq!(f32::from(first_hang.bounds.size.height), 90.0);

    menu.replace(
        "second".to_owned(),
        root(vec![item(2, "Two"), item(3, "Three")]),
        noop(),
        550.0,
    );
    assert!(!menu.is_settled());
    assert!(menu.leaving.is_some());
    assert_eq!(menu.content.value(), 0.0);
    assert_eq!(menu.w.value(), 180.0);
    assert_eq!(menu.h.value(), 90.0);

    measure(&mut menu, 240.0, 140.0);
    let now = Instant::now() + Duration::from_millis(16);
    menu.advance(0.016, now);
    let mid_hang = menu.hang(body, 19.0, 1.0);
    assert!(f32::from(mid_hang.bounds.origin.x) > 260.0);
    assert!(f32::from(mid_hang.bounds.origin.x) < 430.0);
    assert!(menu.w.value() > 180.0 && menu.w.value() < 240.0);
    assert!(menu.h.value() > 90.0 && menu.h.value() < 140.0);

    settle(&mut menu);
    assert!(menu.leaving.is_none());
    let final_hang = menu.hang(body, 19.0, 1.0);
    assert_eq!(f32::from(final_hang.bounds.origin.x), 430.0);
    assert_eq!(f32::from(final_hang.bounds.size.width), 240.0);
    assert_eq!(f32::from(final_hang.bounds.size.height), 140.0);
    assert_eq!(menu.content.value(), 1.0);
}

#[test]
fn replacing_a_clamped_menu_slides_immediately_from_its_drawn_edge() {
    let body = Bounds::new(point(px(200.0), px(10.0)), size(px(600.0), px(38.0)));
    let mut menu = Menu::new("edge".to_owned(), root(vec![item(1, "One")]), noop(), 795.0);
    measure(&mut menu, 200.0, 100.0);
    settle(&mut menu);

    let clamped = menu.hang(body, 19.0, 1.0);
    let right_limit = 800.0 - 19.0 - 10.0 - 200.0;
    assert_eq!(f32::from(clamped.bounds.origin.x), right_limit);

    menu.replace(
        "inward".to_owned(),
        root(vec![item(2, "Two")]),
        noop(),
        500.0,
    );
    let now = Instant::now() + Duration::from_millis(16);
    menu.advance(0.016, now);
    let moved = menu.hang(body, 19.0, 1.0);
    assert!(f32::from(moved.bounds.origin.x) < right_limit);
}

#[test]
fn entering_and_leaving_a_submenu_crossfades_and_resizes() {
    let mut sub = item(10, "More");
    sub.children = vec![item(11, "Inner A"), item(12, "Inner B")];
    let mut menu = Menu::new("tray".to_owned(), root(vec![sub]), noop(), 400.0);
    measure(&mut menu, 160.0, 60.0);
    settle(&mut menu);

    menu.enter(0);
    assert!(!menu.is_settled());
    assert!(menu.leaving.is_some());
    assert_eq!(menu.content.value(), 0.0);
    measure(&mut menu, 220.0, 120.0);
    settle(&mut menu);
    assert_eq!(menu.w.value(), 220.0);
    assert_eq!(menu.h.value(), 120.0);

    menu.back();
    assert!(!menu.is_settled());
    assert!(menu.leaving.is_some());
    measure(&mut menu, 160.0, 60.0);
    settle(&mut menu);
    assert_eq!(menu.w.value(), 160.0);
    assert_eq!(menu.h.value(), 60.0);
}
