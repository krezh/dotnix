use super::*;

#[test]
fn two_lists_pass_whole_without_overlapping() {
    let list = |height: f32| {
        Block::Placed(Placed {
            width: 700.0,
            height,
            visible: Some(height),
            still: height,
            rewind: None,
            blocks: vec![(place(0.0, 60.0, 1.0), Block::note("Card"))],
        })
    };
    let Block::Placed(passing) = across(list(100.0), list(300.0), 0.25, true) else {
        panic!("one list");
    };
    let [(left, _), (right, _)] = passing.blocks.as_slice() else {
        panic!("both lists");
    };
    assert_eq!((left.opacity, right.opacity), (1.0, 1.0), "neither fades");
    assert!(left.offset < 0.0 && right.offset > 0.0, "they move apart");
    assert!(
        right.offset - left.offset >= passing.width,
        "a full width apart, so no card sits over another"
    );
    assert_eq!(passing.visible, Some(150.0), "the panel's size follows");
}

#[test]
fn passing_lists_show_the_edges_of_the_one_coming_in_from_the_start() {
    let list = |still: f32| {
        Block::Placed(Placed {
            width: 700.0,
            height: still.max(200.0),
            visible: Some(200.0),
            still: still.max(200.0),
            rewind: None,
            blocks: vec![(place(0.0, 60.0, 1.0), Block::note("Card"))],
        })
    };
    let out_of_view = |left: f32, right: f32, at: f32| {
        let Block::Placed(passing) = across(list(left), list(right), at, true) else {
            panic!("one list");
        };
        passing.still - passing.visible.unwrap_or(0.0)
    };
    assert_eq!(
        out_of_view(120.0, 900.0, 0.0),
        700.0,
        "a long one coming in shows at once"
    );
    assert_eq!(out_of_view(120.0, 900.0, 0.5), 700.0);
    assert_eq!(
        out_of_view(900.0, 120.0, 0.0),
        0.0,
        "a short one coming in shows none"
    );
}

#[test]
fn the_list_left_behind_scrolls_back_to_the_top_on_the_way_across() {
    let list = |still: f32| {
        Block::Placed(Placed {
            width: 700.0,
            height: still,
            visible: Some(200.0),
            still,
            rewind: None,
            blocks: vec![(place(0.0, 60.0, 1.0), Block::note("Card"))],
        })
    };
    let rewind = |at: f32, rightwards: bool| {
        let Block::Placed(passing) = across(list(900.0), list(500.0), at, rightwards) else {
            panic!("one list");
        };
        passing.rewind
    };
    assert_eq!(
        rewind(0.0, true),
        Some(700.0),
        "as far as the leaving one went"
    );
    assert_eq!(rewind(1.0, true), Some(0.0), "back at the top once across");
    assert_eq!(
        rewind(1.0, false),
        Some(300.0),
        "the other way, the right one is leaving"
    );
    assert_eq!(rewind(0.0, false), Some(0.0));
}
