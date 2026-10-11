use gtk::glib;
use gtk::prelude::*;
use nix_changelog::Changelog;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::ui::theme;
use crate::ui::label;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkdownBlockKind {
    Heading(u8),
    Paragraph,
    Item(usize),
    Quote,
    Code,
    Rule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MarkdownBlock {
    pub(crate) kind: MarkdownBlockKind,
    pub(crate) markup: String,
}

struct MarkdownBlocks {
    blocks: Vec<MarkdownBlock>,
    current: Option<MarkdownBlockKind>,
    markup: String,
    list_depth: usize,
}

impl MarkdownBlocks {
    fn begin(&mut self, kind: MarkdownBlockKind) {
        self.finish();
        self.current = Some(kind);
    }

    fn finish(&mut self) {
        let Some(kind) = self.current.take() else {
            return;
        };
        let markup = std::mem::take(&mut self.markup);
        if !markup.trim().is_empty() {
            self.blocks.push(MarkdownBlock { kind, markup });
        }
    }

    fn ensure_paragraph(&mut self) {
        if self.current.is_none() {
            self.current = Some(MarkdownBlockKind::Paragraph);
        }
    }

    fn push_escaped(&mut self, text: &str) {
        self.ensure_paragraph();
        self.markup.push_str(&glib::markup_escape_text(text));
    }

    fn push_markup(&mut self, markup: &str) {
        self.ensure_paragraph();
        self.markup.push_str(markup);
    }
}

pub(crate) fn markdown_blocks(markdown: &str) -> Vec<MarkdownBlock> {
    let mut output = MarkdownBlocks {
        blocks: Vec::new(),
        current: None,
        markup: String::with_capacity(markdown.len()),
        list_depth: 0,
    };
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                output.begin(MarkdownBlockKind::Heading(heading_level(level)));
            }
            Event::End(TagEnd::Heading(_)) => output.finish(),
            Event::Start(Tag::Paragraph) => output.ensure_paragraph(),
            Event::End(TagEnd::Paragraph)
                if matches!(
                    output.current,
                    Some(MarkdownBlockKind::Paragraph | MarkdownBlockKind::Quote)
                ) =>
            {
                output.finish();
            }
            Event::Start(Tag::List(_)) => output.list_depth += 1,
            Event::End(TagEnd::List(_)) => {
                output.list_depth = output.list_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                output.begin(MarkdownBlockKind::Item(output.list_depth.max(1)));
            }
            Event::End(TagEnd::Item) => output.finish(),
            Event::Start(Tag::BlockQuote(_)) => output.begin(MarkdownBlockKind::Quote),
            Event::End(TagEnd::BlockQuote(_)) => output.finish(),
            Event::Start(Tag::CodeBlock(_)) => output.begin(MarkdownBlockKind::Code),
            Event::End(TagEnd::CodeBlock) => output.finish(),
            Event::Rule => {
                output.finish();
                output.blocks.push(MarkdownBlock {
                    kind: MarkdownBlockKind::Rule,
                    markup: String::new(),
                });
            }
            Event::Start(Tag::Strong) => output.push_markup("<b>"),
            Event::End(TagEnd::Strong) => output.push_markup("</b>"),
            Event::Start(Tag::Emphasis) => output.push_markup("<i>"),
            Event::End(TagEnd::Emphasis) => output.push_markup("</i>"),
            Event::Start(Tag::Strikethrough) => output.push_markup("<s>"),
            Event::End(TagEnd::Strikethrough) => output.push_markup("</s>"),
            Event::Start(Tag::Link { dest_url, .. }) => {
                output.push_markup("<a href=\"");
                output
                    .markup
                    .push_str(&glib::markup_escape_text(dest_url.as_ref()));
                output.markup.push_str("\">");
            }
            Event::End(TagEnd::Link) => output.push_markup("</a>"),
            Event::Code(text) => {
                output.push_markup(&format!(
                    "<tt><span foreground=\"{}\">",
                    crate::ui::theme::CODE
                ));
                output
                    .markup
                    .push_str(&glib::markup_escape_text(text.as_ref()));
                output.markup.push_str("</span></tt>");
            }
            Event::Text(text) | Event::InlineHtml(text) | Event::Html(text) => {
                output.push_escaped(text.as_ref());
            }
            Event::SoftBreak => output.push_markup(" "),
            Event::HardBreak => output.push_markup("\n"),
            Event::TaskListMarker(checked) => {
                output.push_escaped(if checked { "☑ " } else { "☐ " });
            }
            _ => {}
        }
    }
    output.finish();
    output.blocks
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Fills the empty `content` with the description and one section per release; returns the
/// sections in release order so callers can jump between them.
pub(crate) fn render(content: &gtk::Box, changelog: &Changelog) -> Vec<gtk::Box> {
    if let Some(description) = changelog.description.as_deref() {
        let description = label(description, &["changelog-description"], 0.0);
        description.set_ellipsize(gtk::pango::EllipsizeMode::None);
        description.set_wrap(true);
        description.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        content.append(&description);
    }

    if changelog.releases.is_empty() {
        let empty = label(
            "No release notes found for this version",
            &["changelog-empty"],
            0.5,
        );
        empty.set_vexpand(true);
        empty.set_valign(gtk::Align::Center);
        content.append(&empty);
        return Vec::new();
    }

    let mut sections = Vec::with_capacity(changelog.releases.len());
    for (index, release) in changelog.releases.iter().enumerate() {
        if index > 0 {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.add_css_class("release-separator");
            content.append(&separator);
        }
        let section = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let tag = label(
            &release.tag,
            &["release-tag", "chip", "tinted", theme::tone(theme::VERSION)],
            0.0,
        );
        tag.set_halign(gtk::Align::Start);
        tag.set_tooltip_text(Some(&release.tag));
        section.append(&tag);
        // Headings like "Bug Fixes" tint themselves and the items under them, until the next
        // heading at the same or a higher level.
        let mut tone: Option<(u8, &'static str)> = None;
        for block in markdown_blocks(&release.body) {
            if let MarkdownBlockKind::Heading(level) = block.kind {
                let own = heading_tone(&block.markup).map(theme::tone);
                if tone.is_none_or(|(tone_level, _)| level <= tone_level) {
                    tone = own.map(|class| (level, class));
                }
                let class = own.or(tone.map(|(_, class)| class));
                append_markdown_block(&section, block, class);
            } else {
                append_markdown_block(&section, block, tone.map(|(_, class)| class));
            }
        }
        content.append(&section);
        sections.push(section);
    }
    sections
}

/// The colour of a changelog section, by what its heading says the section holds: features
/// green, fixes peach, breaking changes red, security maroon. Stems match the start of a word,
/// so "prefix" is no fix.
fn heading_tone(markup: &str) -> Option<theme::Color> {
    let title = strip_markup(markup).to_lowercase();
    let words = title
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let has = |stems: &[&str]| {
        words
            .iter()
            .any(|word| stems.iter().any(|stem| word.starts_with(stem)))
    };
    if has(&["breaking", "removed", "removal", "deprecat", "incompatib"]) {
        Some(theme::RED)
    } else if has(&["security", "vulnerab", "cve"]) {
        Some(theme::MAROON)
    } else if has(&["fix", "bug"]) {
        Some(theme::PEACH)
    } else if has(&["feature", "added", "new", "enhancement", "improvement"]) {
        Some(theme::GREEN)
    } else {
        None
    }
}

/// The visible text of Pango markup: tags dropped, the entities `markdown_blocks` emits decoded.
fn strip_markup(markup: &str) -> String {
    let mut text = String::with_capacity(markup.len());
    let mut in_tag = false;
    for character in markup.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// `tone` is the section's tone class; headings and bullets are drawn in it.
fn append_markdown_block(section: &gtk::Box, block: MarkdownBlock, tone: Option<&'static str>) {
    let toned = |classes: &[&'static str]| {
        let mut classes = classes.to_vec();
        if let Some(tone) = tone {
            classes.extend(["toned", tone]);
        }
        classes
    };
    match block.kind {
        MarkdownBlockKind::Rule => {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.add_css_class("markdown-separator");
            section.append(&separator);
        }
        MarkdownBlockKind::Item(depth) => {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            row.set_margin_start(((depth - 1) * 18) as i32);
            let bullet = label("•", &toned(&["markdown-bullet"]), 0.5);
            bullet.set_valign(gtk::Align::Start);
            row.append(&bullet);
            let body = markdown_label(&block.markup, &["markdown-text"]);
            body.set_hexpand(true);
            row.append(&body);
            section.append(&row);
        }
        MarkdownBlockKind::Heading(level) => {
            let class = match level {
                1 => "markdown-heading-1",
                2 => "markdown-heading-2",
                _ => "markdown-heading-3",
            };
            section.append(&markdown_label(
                &block.markup,
                &toned(&["markdown-heading", class]),
            ));
        }
        MarkdownBlockKind::Paragraph => {
            section.append(&markdown_label(&block.markup, &["markdown-text"]));
        }
        MarkdownBlockKind::Quote => {
            section.append(&markdown_label(
                &block.markup,
                &["markdown-text", "markdown-quote"],
            ));
        }
        MarkdownBlockKind::Code => {
            section.append(&markdown_label(
                &block.markup,
                &["markdown-text", "markdown-code"],
            ));
        }
    }
}

fn markdown_label(markup: &str, classes: &[&str]) -> gtk::Label {
    let value = label("", classes, 0.0);
    value.set_markup(markup);
    value.set_ellipsize(gtk::pango::EllipsizeMode::None);
    value.set_selectable(true);
    value.set_wrap(true);
    value.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    value
}

#[cfg(test)]
mod tests {
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
}
