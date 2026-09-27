use gtk::glib;
use gtk::prelude::*;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::changelog::ChangelogOutput;
use crate::{clear, label};

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
                output.push_markup("<tt>");
                output
                    .markup
                    .push_str(&glib::markup_escape_text(text.as_ref()));
                output.markup.push_str("</tt>");
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

pub(crate) fn render(content: &gtk::Box, changelog: &ChangelogOutput) {
    clear(content);

    let metadata = gtk::Box::new(gtk::Orientation::Vertical, 8);
    metadata.add_css_class("changelog-metadata");
    let identity = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    identity.append(&label(&changelog.pname, &["changelog-package"], 0.0));
    identity.append(&label(&changelog.version, &["changelog-version"], 0.0));
    metadata.append(&identity);
    if let Some(description) = changelog.description.as_deref() {
        let description = label(description, &["changelog-description"], 0.0);
        description.set_ellipsize(gtk::pango::EllipsizeMode::None);
        description.set_wrap(true);
        description.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        metadata.append(&description);
    }
    content.append(&metadata);

    if changelog.releases.is_empty() {
        content.append(&label(
            "No release notes found for this version.",
            &["no-changes"],
            0.0,
        ));
        return;
    }

    for release in &changelog.releases {
        let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
        section.add_css_class("release-section");
        section.append(&label(&release.tag, &["release-tag"], 0.0));
        for block in markdown_blocks(&release.body) {
            append_markdown_block(&section, block);
        }
        content.append(&section);
    }
}

fn append_markdown_block(section: &gtk::Box, block: MarkdownBlock) {
    match block.kind {
        MarkdownBlockKind::Rule => {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.add_css_class("markdown-separator");
            section.append(&separator);
        }
        MarkdownBlockKind::Item(depth) => {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            row.add_css_class("markdown-list-item");
            row.set_margin_start(((depth - 1) * 18) as i32);
            let bullet = label("•", &["markdown-bullet"], 0.5);
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
            section.append(&markdown_label(&block.markup, &["markdown-heading", class]));
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
