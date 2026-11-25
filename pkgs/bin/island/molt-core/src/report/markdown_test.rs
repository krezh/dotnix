use super::*;

#[test]
fn code_spans_are_plain_tt() {
    let blocks = markdown_blocks("Handle **empty** `--auth`");
    assert_eq!(blocks[0].markup, "Handle <b>empty</b> <tt>--auth</tt>");
}

#[test]
fn markup_reads_as_text_with_styles_and_links() {
    let (text, styled, links) =
        spans("Use <b>bold</b> and <tt>a &lt; b</tt> <a href=\"https://x.dev/?a=1&amp;b=2\">here</a>");
    assert_eq!(text, "Use bold and a < b here");
    assert_eq!(
        styled,
        vec![
            (
                4..8,
                Inline {
                    bold: true,
                    ..Inline::default()
                }
            ),
            (
                13..18,
                Inline {
                    code: true,
                    ..Inline::default()
                }
            ),
        ]
    );
    assert_eq!(links, vec![(19..23, "https://x.dev/?a=1&b=2".to_owned())]);
}

#[test]
fn tones_headings_by_their_words() {
    assert_eq!(heading_tone("Bug Fixes"), Some(Hue::Peach));
    assert_eq!(heading_tone("Security fixes"), Some(Hue::Maroon));
    assert_eq!(heading_tone("Prefix handling renewed"), None);
}
