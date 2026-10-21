use super::{Style, parse};

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
    let reply = "jj has no stash.\nStart a change:\n\n```sh\njj new @-\n```\n- keep **it**\n- or squash";
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
        super::Block::Paragraph(text) => {
            assert_eq!(text.text, "• keep it\n• or squash");
            // "•" takes three bytes.
            assert_eq!(text.spans, vec![(9..11, bold())]);
        }
        other => panic!("expected a paragraph, got {other:?}"),
    }
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
