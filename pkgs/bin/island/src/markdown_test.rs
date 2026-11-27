use super::{REMEMBERED, Style, blocks, parse, plain};

#[test]
fn remembered_results_match_a_fresh_read() {
    let text = "**bold** and `code`";
    assert_eq!(parse(text), parse(text));
    let reply = "# Title\n\n```rust\nlet x = 1;\n```";
    assert_eq!(blocks(reply), blocks(reply));
}

#[test]
fn remembering_stays_bounded_and_correct() {
    for index in 0..REMEMBERED * 2 {
        let text = format!("*{index}*");
        assert_eq!(parse(&text).text, index.to_string());
    }
    assert!(super::TEXTS.with_borrow(|known| known.len()) <= REMEMBERED);
}

#[test]
fn plain_text_drops_the_marks_and_keeps_code() {
    assert_eq!(plain("**2**"), "2");
    assert_eq!(
        plain("# Sum\nIt is *two*.\n\n```rust\nlet x = 1 + 1;\n```"),
        "Sum\n\nIt is two.\n\nlet x = 1 + 1;"
    );
}

#[test]
fn plain_text_keeps_each_links_address() {
    assert_eq!(
        plain("See [the docs](https://example.com/docs) for more."),
        "See the docs (https://example.com/docs) for more."
    );
    assert_eq!(
        plain("[https://example.com](https://example.com)"),
        "https://example.com"
    );
}

fn bold() -> Style {
    Style {
        bold: true,
        ..Style::default()
    }
}

/// The text of `markdown` with each styled stretch shown, as `[stretch]`.
fn marked(markdown: &str) -> String {
    let parsed = parse(markdown);
    let mut shown = String::new();
    let mut at = 0;
    for (range, _) in &parsed.spans {
        shown.push_str(&parsed.text[at..range.start]);
        shown.push('[');
        shown.push_str(&parsed.text[range.clone()]);
        shown.push(']');
        at = range.end;
    }
    shown.push_str(&parsed.text[at..]);
    shown
}

#[test]
fn the_plex_bot_reads_as_text() {
    let parsed = parse("📺 Episode **22x07** of **Taskmaster** has been added!\nResolution: 1080p");
    assert_eq!(
        parsed.text,
        "📺 Episode 22x07 of Taskmaster has been added! Resolution: 1080p"
    );
    let bold: Vec<&str> = parsed
        .spans
        .iter()
        .filter(|(_, style)| *style == bold())
        .map(|(range, _)| &parsed.text[range.clone()])
        .collect();
    assert_eq!(bold, ["22x07", "Taskmaster"]);
}

#[test]
fn lines_join_into_one() {
    assert_eq!(parse("first\nsecond").text, "first · second");
    assert_eq!(parse("Done.\n\nNext").text, "Done. Next");
    assert_eq!(parse("\n  \nonly\n").text, "only");
}

#[test]
fn inline_styles() {
    assert_eq!(marked("a *b* _c_ __d__ ~~e~~ `f`"), "a [b] [c] [d] [e] [f]");
    let parsed = parse("~~gone~~");
    assert!(parsed.spans[0].1.strike);
    let parsed = parse("__under__");
    assert!(parsed.spans[0].1.underline && !parsed.spans[0].1.italic);
}

#[test]
fn styles_nest() {
    let parsed = parse("**bold *both***");
    assert_eq!(parsed.text, "bold both");
    assert_eq!(parsed.spans.len(), 2);
    assert!(parsed.spans[1].1.bold && parsed.spans[1].1.italic);
}

#[test]
fn unclosed_markers_stay() {
    assert_eq!(parse("2 * 3 = 6").text, "2 * 3 = 6");
    assert_eq!(parse("**open").text, "**open");
    assert_eq!(parse("a * b * c").text, "a * b * c");
}

#[test]
fn underscores_inside_words_are_not_italics() {
    assert_eq!(parse("file_name_here").text, "file_name_here");
    assert!(parse("file_name_here").spans.is_empty());
    assert_eq!(marked("_whole_ word"), "[whole] word");
}

#[test]
fn escapes_keep_markers() {
    assert_eq!(parse(r"\*not italic\*").text, "*not italic*");
}

#[test]
fn code_is_taken_as_written() {
    let parsed = parse("run `**not bold**` now");
    assert_eq!(parsed.text, "run **not bold** now");
    assert!(parsed.spans[0].1.code && !parsed.spans[0].1.bold);
    assert_eq!(parse("```\nlet x = 1;\n```").text, "let x = 1;");
}

#[test]
fn masked_links() {
    let parsed = parse("see [the docs](https://example.com) now");
    assert_eq!(parsed.text, "see the docs now");
    assert_eq!(parsed.links, ["https://example.com"]);
    assert_eq!(&parsed.text[parsed.link_ranges()[0].clone()], "the docs");
    assert_eq!(parse("[x](not a url)").text, "[x](not a url)");
}

#[test]
fn block_markers_go() {
    assert_eq!(marked("# Title\nbody"), "[Title] · body");
    assert_eq!(parse("> quoted").text, "quoted");
    assert_eq!(parse("-# small print").text, "small print");
    assert_eq!(parse("- one\n- two").text, "• one · • two");
}

#[test]
fn spoilers_and_custom_emoji() {
    let parsed = parse("it was ||him||");
    assert_eq!(parsed.text, "it was him");
    assert!(parsed.spans[0].1.spoiler);
    assert_eq!(parse("hi <:wave:123456>").text, "hi :wave:");
    assert_eq!(parse("a <b> c").text, "a <b> c");
}

#[test]
fn blocks_keep_paragraphs_lists_and_fenced_code() {
    let reply =
        "jj has no stash.\nStart a change:\n\n```sh\njj new @-\n```\n- keep **it**\n- or squash";
    let parsed = super::blocks(reply);
    assert_eq!(parsed.len(), 3);
    match &parsed[0] {
        super::Block::Paragraph(text) => assert_eq!(text.text, "jj has no stash.\nStart a change:"),
        other => panic!("expected a paragraph, got {other:?}"),
    }
    assert_eq!(
        parsed[1],
        super::Block::Code {
            language: "sh".to_owned(),
            lines: vec!["jj new @-".to_owned()],
        }
    );
    match &parsed[2] {
        super::Block::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].text.text, "keep it");
            assert_eq!(items[0].text.spans, vec![(5..7, bold())]);
            assert_eq!((items[1].depth, items[1].number), (0, None));
        }
        other => panic!("expected a list, got {other:?}"),
    }
}

fn texts(items: &[super::Item]) -> Vec<(usize, Option<u64>, &str)> {
    items
        .iter()
        .map(|item| (item.depth, item.number, item.text.text.as_str()))
        .collect()
}

#[test]
fn lists_nest_by_indent_and_number_on() {
    let parsed = super::blocks(
        "- **Materials:**\n  - 30 Blood\n    across lines\n  - 3 Bars\n- Gold\n\n1. Farm\n1. Buy",
    );
    match &parsed[..] {
        [super::Block::List(items)] => assert_eq!(
            texts(items),
            vec![
                (0, None, "Materials:"),
                (1, None, "30 Blood across lines"),
                (1, None, "3 Bars"),
                (0, None, "Gold"),
                (0, Some(1), "Farm"),
                (0, Some(2), "Buy"),
            ]
        ),
        other => panic!("expected one list, got {other:?}"),
    }
}

#[test]
fn headings_quotes_and_tables_stand_apart() {
    let parsed = super::blocks(
        "## Quickest route\n| Mount | Gold |\n|---|--:|\n| Dreadsteed | 150 g |\n> From 2019.\n> Maybe more.",
    );
    assert_eq!(parsed.len(), 3);
    match &parsed[0] {
        super::Block::Heading(text) => assert_eq!(text.text, "Quickest route"),
        other => panic!("expected a heading, got {other:?}"),
    }
    match &parsed[1] {
        super::Block::Table { head, rows } => {
            assert_eq!(head[1].text, "Gold");
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0].text, "Dreadsteed");
        }
        other => panic!("expected a table, got {other:?}"),
    }
    match &parsed[2] {
        super::Block::Quote(text) => assert_eq!(text.text, "From 2019.\nMaybe more."),
        other => panic!("expected a quote, got {other:?}"),
    }
}

#[test]
fn a_table_still_arriving_reads_as_a_paragraph() {
    let parsed = super::blocks("| Mount | Gold |");
    assert!(matches!(&parsed[..], [super::Block::Paragraph(_)]));
}

#[test]
fn a_fence_still_arriving_runs_to_the_end() {
    let parsed = super::blocks("Run:\n```\nnix build");
    assert_eq!(
        parsed[1],
        super::Block::Code {
            language: String::new(),
            lines: vec!["nix build".to_owned()],
        }
    );
}
