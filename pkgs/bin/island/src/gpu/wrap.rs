//! Where text breaks into rows and how it is cut short to fit.

use std::{collections::HashSet, ops::Range};

use icu_segmenter::LineSegmenter;

pub const ELLIPSIS: char = '…';

const MOST_INDENT: usize = 256;

fn breaks(text: &str) -> impl Iterator<Item = usize> {
    LineSegmenter::new_for_non_complex_scripts(Default::default()).segment_str(text)
}

/// `glyphs`, each a byte index, where it starts and how wide, as `row_starts` takes them: how far
/// each is along the way the line reads, and how long the line is. A right-to-left paragraph lays
/// its glyphs out from the right.
pub fn along(glyphs: &[(usize, f32, f32)]) -> (Vec<(usize, f32)>, f32) {
    let left = glyphs.iter().map(|glyph| glyph.1).fold(f32::MAX, f32::min);
    let right = glyphs
        .iter()
        .map(|glyph| glyph.1 + glyph.2)
        .fold(f32::MIN, f32::max);
    let backwards = glyphs
        .first()
        .zip(glyphs.last())
        .is_some_and(|(first, last)| first.1 > last.1);
    let spots = glyphs.iter().map(|&(index, x, w)| {
        let spot = if backwards { right - x - w } else { x - left };
        (index, spot)
    });
    (spots.collect(), (right - left).max(0.0))
}

/// Which of `glyphs`, each a byte index in `text` and how far along, start a new row in `room`.
/// `total` is where the last one ends. A row's trailing spaces count, so one pushed over the edge
/// takes the word before it along. Past `most` rows, the last runs on.
pub fn row_starts(
    text: &str,
    glyphs: &[(usize, f32)],
    total: f32,
    room: f32,
    most: Option<usize>,
) -> Vec<usize> {
    let indices: HashSet<usize> = glyphs.iter().map(|glyph| glyph.0).collect();
    let allowed: Vec<usize> = breaks(text)
        .filter(|index| indices.contains(index))
        .collect();
    let mut kept: Vec<(usize, usize, f32)> = Vec::with_capacity(glyphs.len());
    for (place, &(index, x)) in glyphs.iter().enumerate() {
        if kept.last().is_none_or(|last| last.1 != index) {
            kept.push((place, index, x));
        }
    }
    let mut starts = Vec::new();
    let mut segment: Option<Range<usize>> = None;
    let mut candidate = None;
    let (mut row, mut row_x) = (0, 0.0);
    for (at, &(place, index, x)) in kept.iter().enumerate() {
        if text.as_bytes()[index] != b' '
            && segment
                .as_ref()
                .is_none_or(|segment| !segment.contains(&index))
        {
            let next = allowed.partition_point(|allowed| *allowed <= index);
            if segment.is_some() {
                candidate = Some((place, x));
            }
            let start = allowed[..next].last().copied().unwrap_or(0);
            let end = allowed.get(next).copied().unwrap_or(text.len());
            segment = Some(start..end);
        }
        let next_x = kept.get(at + 1).map_or(total, |next| next.2);
        if next_x - row_x > room && place > row {
            if most.is_some_and(|most| starts.len() >= most.saturating_sub(1)) {
                break;
            }
            (row, row_x) = candidate.take().unwrap_or((place, x));
            starts.push(row);
        }
    }
    starts
}

fn ended(text: &str) -> String {
    let kept = text.trim_end_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation());
    format!("{kept}{ELLIPSIS}")
}

/// `text` on one line, ending in an ellipsis if its characters run past `room`.
pub fn shortened_line(
    text: &str,
    room: f32,
    advance: &mut dyn FnMut(char) -> f32,
) -> Option<String> {
    let affix = advance(ELLIPSIS);
    let (mut width, mut cut) = (0.0, 0);
    for (index, c) in text.char_indices() {
        if width + affix < room {
            cut = index;
        }
        width += advance(c);
        if f32::floor(width) > room {
            return Some(ended(&text[..cut]));
        }
    }
    None
}

/// `text` over at most `most` rows, ending in an ellipsis if its characters run past the last.
pub fn shortened_rows(
    text: &str,
    room: f32,
    most: usize,
    advance: &mut dyn FnMut(char) -> f32,
) -> Option<String> {
    if most <= 1 {
        return shortened_line(text, room * most as f32, advance);
    }
    let affix = advance(ELLIPSIS);
    let (mut width, mut row) = (0.0, 0);
    let mut first_ink = None;
    let (mut candidate, mut candidate_width) = (0, 0.0);
    let mut row_start = 0;
    let mut indent: Option<usize> = None;
    let mut cut = 0;
    let mut allowed = breaks(text).peekable();
    for (index, c) in text.char_indices() {
        let breaks_here = allowed.next_if_eq(&index).is_some();
        if c == '\n' {
            if row >= most - 1 && !text[index + 1..].trim().is_empty() {
                return Some(ended(&text[..cut]));
            }
            row += 1;
            width = 0.0;
            first_ink = None;
            row_start = index + 1;
            indent = None;
            cut = index + 1;
            continue;
        }
        let wide = advance(c);
        if breaks_here && first_ink.is_some() {
            (candidate, candidate_width) = (index, width);
        }
        if c != ' ' && first_ink.is_none() {
            first_ink = Some(index);
        }
        width += wide;
        if row < most - 1 {
            if width > room && index > row_start {
                if let (None, Some(ink)) = (indent, first_ink) {
                    indent = Some(MOST_INDENT.min(ink - row_start));
                }
                if candidate > row_start {
                    row_start = candidate;
                    width -= candidate_width;
                } else {
                    row_start = index;
                    width = wide;
                }
                if let Some(indent) = indent {
                    width += advance(' ') * indent as f32;
                }
                row += 1;
                cut = row_start;
            }
        } else {
            if width + affix <= room {
                cut = index + c.len_utf8();
            }
            if width > room {
                return Some(ended(&text[..cut]));
            }
        }
    }
    None
}

/// `styles` for what is left of a text, the ellipsis ending `length` in the style cut through.
pub fn cut_styles<T: Copy>(styles: &[(Range<usize>, T)], length: usize) -> Vec<(Range<usize>, T)> {
    let kept = length - ELLIPSIS.len_utf8();
    styles
        .iter()
        .filter(|(range, _)| range.start <= kept && range.start < range.end)
        .map(|(range, style)| {
            let end = if range.end <= kept { range.end } else { length };
            (range.start..end, *style)
        })
        .collect()
}

#[cfg(test)]
#[path = "wrap_test.rs"]
mod tests;
