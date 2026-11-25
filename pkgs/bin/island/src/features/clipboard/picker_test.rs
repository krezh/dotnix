use super::{Layout, Picker};
use crate::components::list::ROWS;

#[test]
fn labels_start_each_group() {
    let layout = Layout {
        count: 10,
        pinned: 2,
    };
    assert_eq!(layout.section(0), Some("Pinned"));
    assert_eq!(layout.section(1), None);
    assert_eq!(layout.section(2), Some("Recent"));
    let unpinned = Layout {
        count: 3,
        pinned: 0,
    };
    assert_eq!(unpinned.section(0), Some("Recent"));
    let all_pinned = Layout {
        count: 3,
        pinned: 3,
    };
    assert_eq!(all_pinned.section(3), None);
}

#[test]
fn labels_in_view_take_room_from_rows() {
    let layout = Layout {
        count: 20,
        pinned: 2,
    };
    assert_eq!(layout.whole(0), ROWS - 1);
    assert_eq!(layout.whole(1), ROWS - 1);
    assert_eq!(layout.whole(3), ROWS);
}

#[test]
fn keeps_the_selection_clear_of_the_faded_edge() {
    let layout = Layout {
        count: 20,
        pinned: 2,
    };
    let mut picker = Picker::default();
    for _ in 0..ROWS - 1 {
        picker.step(1, layout);
    }
    assert_eq!(picker.selection.selected, ROWS - 1);
    assert_eq!(picker.selection.top, 1);
    picker.step(-(ROWS as i32), layout);
    assert_eq!(picker.selection.top, 0);
}
