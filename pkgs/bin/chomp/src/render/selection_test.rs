use super::*;

#[test]
fn test_rect_from_points() {
    let rect = Rect::from_points(10, 10, 50, 40);
    assert_eq!(rect.x, 10);
    assert_eq!(rect.y, 10);
    assert_eq!(rect.width, 40);
    assert_eq!(rect.height, 30);

    // Test with reversed points
    let rect = Rect::from_points(50, 40, 10, 10);
    assert_eq!(rect.x, 10);
    assert_eq!(rect.y, 10);
    assert_eq!(rect.width, 40);
    assert_eq!(rect.height, 30);
}

#[test]
fn test_selection_single_point() {
    let mut sel = Selection::new();
    sel.start_selection(100, 200);
    sel.update_drag(100, 200, 101, 201);

    let rect = sel.get_selection().expect("Selection should be valid");
    assert_eq!(rect.x, 100);
    assert_eq!(rect.y, 200);
    assert_eq!(rect.width, 1);
    assert_eq!(rect.height, 1);
}

#[test]
fn test_selection_region() {
    let mut sel = Selection::new();
    sel.start_selection(10, 10);

    sel.update_drag(10, 10, 50, 40);

    let rect = sel.get_selection().expect("Selection should be valid");
    assert_eq!(rect.width, 40);
    assert_eq!(rect.height, 30);
}
