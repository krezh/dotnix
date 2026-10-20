use super::*;
#[test]
fn parses_release_markdown_into_readable_blocks() {
    assert_eq!(
        markdown_blocks(
            "## @oh-my-pi/pi-agent-core\n\n### Fixed\n\n- Handle **empty** `--auth`\n\n[Details](https://example.test)\n\n---"
        ),
        vec![
            MarkdownBlock {
                kind: MarkdownBlockKind::Heading(2),
                markup: "@oh-my-pi/pi-agent-core".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Heading(3),
                markup: "Fixed".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Item(1),
                markup: "Handle <b>empty</b> <tt><span foreground=\"#fab387\">--auth</span></tt>"
                    .to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Paragraph,
                markup: "<a href=\"https://example.test\">Details</a>".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Rule,
                markup: String::new(),
            },
        ]
    );
}

#[test]
fn parses_html_release_notes_like_markdown() {
    let code = |text: &str| format!("<tt><span foreground=\"#fab387\">{text}</span></tt>");
    assert_eq!(
        markdown_blocks(concat!(
            "<!-- generated\nby a tool -->\n",
            "<h2>Repo Overview</h2>\n",
            "<ul>\n",
            "  <li>Open <strong>Repo Overview</strong> with <code>Cmd+O</code>,\n",
            "  or Settings &gt; Jujutsu.</li>\n",
            "  <li>See <a href=\"https://example.test/?a=1&amp;b=2\">docs</a><br>now</li>\n",
            "  <li><strong>unclosed</li>\n",
            "</ul>\n",
            "\n",
            "<p>Fixed a <em>crash</em> when a < b.</p>\n",
        )),
        vec![
            MarkdownBlock {
                kind: MarkdownBlockKind::Heading(2),
                markup: "Repo Overview".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Item(1),
                markup: format!(
                    "Open <b>Repo Overview</b> with {}, or Settings &gt; Jujutsu.",
                    code("Cmd+O")
                ),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Item(1),
                markup: "See <a href=\"https://example.test/?a=1&amp;b=2\">docs</a>\nnow"
                    .to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Item(1),
                markup: "<b>unclosed</b>".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Paragraph,
                markup: "Fixed a <i>crash</i> when a &lt; b.".to_owned(),
            },
        ]
    );
}

#[test]
fn parses_nested_lists_preformatted_and_split_html_tags() {
    assert_eq!(
        markdown_blocks(concat!(
            "<ul>\n",
            "<li>Outer<ul><li>Inner <code>x</code></li></ul></li>\n",
            "</ul>\n",
            "<pre><code>fn main() {\n    a &lt; b\n}</code></pre>\n",
            "<hr>\n",
            "<p>Read <a\n href='https://example.test'>more</a></p>\n",
        )),
        vec![
            MarkdownBlock {
                kind: MarkdownBlockKind::Item(1),
                markup: "Outer".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Item(2),
                markup: "Inner <tt><span foreground=\"#fab387\">x</span></tt>".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Code,
                markup: "fn main() {\n    a &lt; b\n}".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Rule,
                markup: String::new(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Paragraph,
                markup: "Read <a href=\"https://example.test\">more</a>".to_owned(),
            },
        ]
    );
}

#[test]
fn tones_headings_by_their_words() {
    assert_eq!(heading_tone("Bug Fixes"), Some(theme::PEACH));
    assert_eq!(heading_tone("<b>Fixed</b>"), Some(theme::PEACH));
    assert_eq!(heading_tone("New Features"), Some(theme::GREEN));
    assert_eq!(heading_tone("⚠ BREAKING CHANGES"), Some(theme::RED));
    assert_eq!(heading_tone("Removed &amp; deprecated"), Some(theme::RED));
    assert_eq!(heading_tone("Security fixes"), Some(theme::MAROON));
    // Words match from their start only: "prefix" is not a fix, "renewed" is not new.
    assert_eq!(heading_tone("Prefix handling renewed"), None);
    assert_eq!(heading_tone("@oh-my-pi/pi-agent-core"), None);
}

#[test]
fn strips_headerlinks_and_pilcrow_from_headings() {
    assert_eq!(
        markdown_blocks(concat!(
            "<h1>Mesa 26.2.5 Release Notes / 2026-10-14<a class=\"headerlink\" href=\"#mesa-26-2-5-release-notes-2026-10-14\" title=\"Link to this heading\">¶</a></h1>\n",
            "<h2>Bug fixes ¶</h2>\n",
            "<h3>Changes <a href=\"#changes\">#</a></h3>\n",
            "<p>See <a href=\"https://mesa3d.org\">docs</a></p>",
        )),
        vec![
            MarkdownBlock {
                kind: MarkdownBlockKind::Heading(1),
                markup: "Mesa 26.2.5 Release Notes / 2026-10-14".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Heading(2),
                markup: "Bug fixes".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Heading(3),
                markup: "Changes".to_owned(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Paragraph,
                markup: "See <a href=\"https://mesa3d.org\">docs</a>".to_owned(),
            },
        ]
    );
}

#[test]
fn renders_option_html_with_entities_inline_code_and_multiline_example() {
    assert_eq!(
        markdown_blocks("<p>Enable <code>a &lt; b</code> &amp; friends.</p>\n<pre><code>{\n  enabled = false;\n  count = 0;\n}</code></pre>"),
        vec![
            MarkdownBlock {
                kind: MarkdownBlockKind::Paragraph,
                markup: "Enable <tt><span foreground=\"#fab387\">a &lt; b</span></tt> &amp; friends.".into(),
            },
            MarkdownBlock {
                kind: MarkdownBlockKind::Code,
                markup: "{\n  enabled = false;\n  count = 0;\n}".into(),
            },
        ]
    );
}
