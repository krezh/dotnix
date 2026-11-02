use super::*;

fn even(text: &str) -> (Vec<(usize, f32)>, f32) {
    let glyphs: Vec<(usize, f32)> = text
        .char_indices()
        .enumerate()
        .map(|(place, (index, _))| (index, place as f32 * 10.0))
        .collect();
    let total = glyphs.len() as f32 * 10.0;
    (glyphs, total)
}

fn rows(text: &str, room: f32) -> Vec<String> {
    let (glyphs, total) = even(text);
    let mut from = 0;
    let mut rows = Vec::new();
    for start in row_starts(text, &glyphs, total, room, None) {
        let at = glyphs[start].0;
        rows.push(text[from..at].to_owned());
        from = at;
    }
    rows.push(text[from..].to_owned());
    rows
}

#[test]
fn a_row_breaks_before_the_word_that_does_not_fit() {
    assert_eq!(rows("aaa bbb ccc", 60.0), ["aaa ", "bbb ", "ccc"]);
}

#[test]
fn a_space_past_the_edge_takes_its_word_along() {
    assert_eq!(rows("aaa bbb ccc", 75.0), ["aaa ", "bbb ccc"]);
    assert_eq!(rows("aaa bbb ccc", 80.0), ["aaa bbb ", "ccc"]);
}

#[test]
fn a_word_too_long_for_a_row_breaks_inside() {
    assert_eq!(rows("aaaaaa", 30.0), ["aaa", "aaa"]);
}

#[test]
fn rows_stop_at_the_most_allowed() {
    let text = "aaa bbb ccc";
    let (glyphs, total) = even(text);
    assert_eq!(row_starts(text, &glyphs, total, 40.0, Some(2)).len(), 1);
}

#[test]
fn a_right_to_left_paragraph_reads_along_from_the_right() {
    let glyphs = [(0, 20.0, 10.0), (1, 10.0, 10.0), (2, 0.0, 10.0)];
    assert_eq!(along(&glyphs), (vec![(0, 0.0), (1, 10.0), (2, 20.0)], 30.0));
}

#[test]
fn a_left_to_right_paragraph_reads_along_from_the_left() {
    let glyphs = [(0, 5.0, 10.0), (1, 15.0, 10.0)];
    assert_eq!(along(&glyphs), (vec![(0, 0.0), (1, 10.0)], 20.0));
}

#[test]
fn no_glyphs_go_nowhere() {
    assert_eq!(along(&[]), (Vec::new(), 0.0));
}

#[test]
fn glyphs_out_of_text_order_still_break_in_row_order() {
    let text = "ab \u{5d0}\u{5d1}\u{5d2} \u{5d3}\u{5d4}\u{5d5} \u{5d6}\u{5d7}\u{5d8}";
    let at = |bytes: &[usize]| {
        let across = bytes.iter().enumerate();
        across
            .map(|(place, &index)| (index, place as f32 * 10.0))
            .collect::<Vec<_>>()
    };
    let glyphs = at(&[0, 1, 2, 21, 19, 17, 16, 14, 12, 10, 9, 7, 5, 3]);
    let starts = row_starts(text, &glyphs, 140.0, 60.0, None);
    assert!(!starts.is_empty());
    assert!(starts.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(starts.iter().all(|start| *start < glyphs.len()));
}

#[test]
fn a_cut_line_drops_trailing_spaces_and_punctuation() {
    let mut advance = |_| 10.0;
    assert_eq!(
        shortened_line("hello, world", 75.0, &mut advance).as_deref(),
        Some("hello…")
    );
    assert_eq!(shortened_line("hello", 75.0, &mut advance), None);
}

#[test]
fn cut_rows_keep_the_rows_before_the_last() {
    let mut advance = |_| 10.0;
    assert_eq!(
        shortened_rows("aaa bbb ccc ddd", 40.0, 2, &mut advance).as_deref(),
        Some("aaa bbb…")
    );
    assert_eq!(shortened_rows("aaa bbb", 40.0, 2, &mut advance), None);
}

#[test]
fn the_ellipsis_takes_the_style_it_cuts_through() {
    let styles = [(0..3, 'a'), (3..8, 'b'), (8..10, 'c')];
    let length = 5 + ELLIPSIS.len_utf8();
    assert_eq!(cut_styles(&styles, length), [(0..3, 'a'), (3..length, 'b')]);
}
