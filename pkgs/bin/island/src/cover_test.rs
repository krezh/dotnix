use gpui::{Bounds, point, px, size};

use super::outside;
use crate::island::SURFACE_W;

#[test]
fn the_cover_leaves_a_hole_for_the_island_and_its_menu() {
    let viewport = size(px(SURFACE_W + 200.0), px(800.0));
    let bar = Bounds::new(point(px(300.0), px(10.0)), size(px(500.0), px(38.0)));
    let menu = Bounds::new(point(px(320.0), px(48.0)), size(px(200.0), px(100.0)));
    let region = outside(viewport, &[bar, menu]);
    let covered = |x: f32, y: f32| {
        region
            .iter()
            .any(|rect| rect.contains(&point(px(x), px(y))))
    };
    assert!(!covered(100.0 + 400.0, 20.0));
    assert!(!covered(100.0 + 400.0, 100.0));
    assert!(covered(100.0 + 700.0, 100.0));
    assert!(covered(5.0, 5.0));
    assert!(covered(100.0 + 400.0, 400.0));
    let area: f32 = region
        .iter()
        .map(|rect| f32::from(rect.size.width) * f32::from(rect.size.height))
        .sum();
    assert_eq!(
        area,
        (SURFACE_W + 200.0) * 800.0 - 500.0 * 38.0 - 200.0 * 100.0
    );
}
