use super::{shared, stagger};

#[test]
fn keeps_what_two_texts_share() {
    assert_eq!(shared("Check now", "Checking…"), (5, 0));
    assert_eq!(shared("12%", "13%"), (1, 1));
    assert_eq!(shared("same", "same"), (4, 0));
    assert_eq!(shared("", "new"), (0, 0));
}

#[test]
fn shared_parts_never_overlap() {
    let (start, end) = shared("aa", "aaa");
    assert!(start + end <= 2);
    let (start, end) = shared("aba", "a");
    assert!(start + end <= 1);
}

#[test]
fn splits_on_characters_not_bytes() {
    let (start, end) = shared("Checking…", "Checked…");
    assert_eq!(&"Checking…"[..start], "Check");
    assert_eq!(&"Checking…"["Checking…".len() - end..], "…");
}

#[test]
fn a_lone_change_takes_the_whole_morph() {
    assert_eq!(stagger(0.0, 0, 1), 0.0);
    assert_eq!(stagger(0.5, 0, 1), 0.5);
    assert_eq!(stagger(1.0, 0, 1), 1.0);
}

#[test]
fn changes_run_along_the_text() {
    let count = 4;
    assert_eq!(stagger(0.0, 0, count), 0.0);
    assert_eq!(stagger(1.0, count - 1, count), 1.0);
    assert!(stagger(0.3, 0, count) > stagger(0.3, 1, count));
    assert!(stagger(0.3, 1, count) > stagger(0.3, 3, count));
    assert_eq!(stagger(0.2, 3, count), 0.0);
}

#[test]
fn every_change_finishes_with_the_morph() {
    for count in 1..12 {
        for index in 0..count {
            assert_eq!(stagger(1.0, index, count), 1.0, "{index} of {count}");
        }
    }
}
