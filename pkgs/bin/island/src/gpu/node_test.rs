use taffy::prelude::TaffyMaxContent;

use super::*;

fn layout(node: &Node) -> (TaffyTree<Context>, Laid) {
    let mut taffy: TaffyTree<Context> = TaffyTree::new();
    let laid = tree(node, Words::base(1.0), &mut taffy);
    taffy
        .compute_layout_with_measure(laid.id, Size::MAX_CONTENT, |inputs, _, _, style| {
            compute_leaf_layout(inputs, style, |_, _| 0.0, |_, _| Size::ZERO)
        })
        .unwrap();
    (taffy, laid)
}

fn rect(taffy: &TaffyTree<Context>, laid: &Laid) -> [f32; 4] {
    let layout = taffy.layout(laid.id).unwrap();
    [
        layout.location.x,
        layout.location.y,
        layout.size.width,
        layout.size.height,
    ]
}

#[test]
fn an_absolute_child_fills_what_holds_it() {
    let node = div()
        .w(200.0)
        .h(50.0)
        .child(div().absolute().inset(0.0))
        .child(div().absolute().top(0.0).left(0.0).h_full().w_share(0.25));
    let (taffy, laid) = layout(&node);
    assert_eq!(rect(&taffy, &laid.children[0]), [0.0, 0.0, 200.0, 50.0]);
    assert_eq!(rect(&taffy, &laid.children[1]), [0.0, 0.0, 50.0, 50.0]);
}

#[test]
fn a_track_inside_a_column_holds_its_fill() {
    let track = div()
        .w_full()
        .h(10.0)
        .child(div().absolute().top(0.0).left(0.0).h_full().w_share(0.5));
    let node = div().w(300.0).child(div().w_full().py(3.0).child(track));
    let (taffy, laid) = layout(&node);
    let track = &laid.children[0].children[0];
    assert_eq!(rect(&taffy, track), [0.0, 3.0, 300.0, 10.0]);
    assert_eq!(rect(&taffy, &track.children[0]), [0.0, 0.0, 150.0, 10.0]);
}

#[test]
fn a_growing_area_holds_its_absolute_chart() {
    let area = div()
        .flex_1()
        .min_h(40.0)
        .child(div().absolute().inset(0.0));
    let node = div().w(240.0).h(160.0).p(10.0).col().child(area);
    let (taffy, laid) = layout(&node);
    let area = &laid.children[0];
    assert_eq!(rect(&taffy, area), [10.0, 10.0, 220.0, 140.0]);
    assert_eq!(rect(&taffy, &area.children[0]), [0.0, 0.0, 220.0, 140.0]);
}

#[test]
fn a_pulsing_mark_never_fades_out_altogether() {
    use crate::components::badge::pulsed;
    assert_eq!(pulsed(0.0), 1.0);
    assert!((pulsed(0.5) - 0.45).abs() < 1e-5);
    assert_eq!(pulsed(1.0), pulsed(0.0));
}
