use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use serde::Deserialize;
#[derive(Clone, Deserialize)]
pub(crate) struct ChangelogOutput {
    pub(crate) pname: String,
    pub(crate) version: String,
    pub(crate) description: Option<String>,
    pub(crate) releases: Vec<ChangelogRelease>,
}
#[derive(Clone, Deserialize)]
pub(crate) struct ChangelogRelease {
    pub(crate) tag: String,
    pub(crate) body: String,
}
pub(crate) fn parse_changelog(json: &[u8]) -> Result<ChangelogOutput, String> {
    serde_json::from_slice(json)
        .map_err(|error| format!("nix-changelog returned invalid data: {error}"))
}
pub(crate) fn markdown_to_text(markdown: &str) -> String {
    let mut output = String::with_capacity(markdown.len());
    let mut link_destinations = Vec::new();
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Item) => {
                ensure_line_break(&mut output);
                output.push_str("• ");
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_destinations.push(dest_url.into_string());
            }
            Event::End(TagEnd::Link) => {
                if let Some(destination) = link_destinations.pop()
                    && !destination.is_empty()
                {
                    output.push_str(" (");
                    output.push_str(&destination);
                    output.push(')');
                }
            }
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::Item
                | TagEnd::CodeBlock
                | TagEnd::BlockQuote(_),
            )
            | Event::HardBreak => ensure_line_break(&mut output),
            Event::SoftBreak => output.push(' '),
            Event::Rule => {
                ensure_line_break(&mut output);
                output.push_str("────────");
                ensure_line_break(&mut output);
            }
            Event::Text(text) | Event::Code(text) | Event::InlineHtml(text) => {
                output.push_str(&text);
            }
            _ => {}
        }
    }
    output.trim_end().to_owned()
}

fn ensure_line_break(output: &mut String) {
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
}
