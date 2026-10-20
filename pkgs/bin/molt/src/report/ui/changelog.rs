use gtk::glib;
use gtk::prelude::*;
use nix_changelog::Changelog;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::ui::label;
use crate::ui::theme;

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
    /// Closing Pango tags for inline elements opened by raw HTML, innermost last. Closed when the
    /// block ends, so unbalanced HTML never yields invalid markup.
    open_inline: Vec<&'static str>,
    /// A raw HTML tag split across lines of an HTML block.
    html_pending: String,
    in_comment: bool,
    preformatted: bool,
    /// Depth of HTML elements whose content should be ignored (e.g. `<a class="headerlink">¶</a>`).
    skip_depth: usize,
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
        if kind != MarkdownBlockKind::Code {
            self.markup
                .truncate(self.markup.trim_end_matches(' ').len());
        }
        while let Some(close) = self.open_inline.pop() {
            self.markup.push_str(close);
        }
        let mut markup = std::mem::take(&mut self.markup);
        if matches!(kind, MarkdownBlockKind::Heading(_)) {
            strip_heading_permalinks(&mut markup);
        }
        if !markup.trim().is_empty() {
            self.blocks.push(MarkdownBlock { kind, markup });
        }
    }

    fn finish_if(&mut self, matches: fn(MarkdownBlockKind) -> bool) {
        if self.current.is_some_and(matches) {
            self.finish();
        }
    }

    fn open_inline(&mut self, open: &str, close: &'static str) {
        self.push_markup(open);
        self.open_inline.push(close);
    }

    fn close_inline(&mut self, close: &'static str) {
        if self.open_inline.last() == Some(&close) {
            self.open_inline.pop();
            self.markup.push_str(close);
        }
    }

    /// Raw HTML from a release body: GitHub renders it, so its common block and inline tags map
    /// onto the same blocks and Pango markup as Markdown. Unknown tags are dropped, comments too.
    fn push_html(&mut self, html: &str) {
        let joined;
        let mut rest = if self.html_pending.is_empty() {
            html
        } else {
            joined = std::mem::take(&mut self.html_pending) + html;
            joined.as_str()
        };
        while !rest.is_empty() {
            if self.in_comment {
                let Some(end) = rest.find("-->") else {
                    return;
                };
                self.in_comment = false;
                rest = &rest[end + 3..];
                continue;
            }
            let Some(start) = html_tag_start(rest) else {
                self.push_html_text(rest);
                return;
            };
            self.push_html_text(&rest[..start]);
            rest = &rest[start..];
            if let Some(comment) = rest.strip_prefix("<!--") {
                self.in_comment = true;
                rest = comment;
                continue;
            }
            let Some(end) = html_tag_end(rest) else {
                self.html_pending = rest.to_owned();
                return;
            };
            self.html_tag(&rest[1..end]);
            rest = &rest[end + 1..];
        }
    }

    /// A tag left unterminated when its HTML block ended was text after all.
    fn flush_html(&mut self) {
        let pending = std::mem::take(&mut self.html_pending);
        self.push_html_text(&pending);
    }

    /// HTML text: entities decoded and, outside `<pre>`, whitespace runs collapsed to one space.
    fn push_html_text(&mut self, raw: &str) {
        if self.skip_depth > 0 {
            return;
        }
        let text = decode_entities(raw);
        if self.preformatted {
            let text = if self.markup.is_empty() {
                text.strip_prefix('\n').unwrap_or(&text)
            } else {
                &text
            };
            if !text.is_empty() {
                self.push_escaped(text);
            }
            return;
        }
        let mut collapsed = String::with_capacity(text.len());
        let mut space = self.current.is_none() || self.markup.ends_with([' ', '\n']);
        for character in text.chars() {
            if character.is_ascii_whitespace() {
                if !space {
                    collapsed.push(' ');
                    space = true;
                }
            } else {
                collapsed.push(character);
                space = false;
            }
        }
        if !collapsed.is_empty() {
            self.push_escaped(&collapsed);
        }
    }

    /// `tag` is the text between `<` and `>`.
    fn html_tag(&mut self, tag: &str) {
        let (closing, tag) = match tag.strip_prefix('/') {
            Some(tag) => (true, tag),
            None => (false, tag),
        };
        let name_end = tag
            .find(|character: char| !character.is_ascii_alphanumeric())
            .unwrap_or(tag.len());
        let mut buffer = [0u8; 10];
        let Some(name) = buffer.get_mut(..name_end) else {
            return;
        };
        name.copy_from_slice(&tag.as_bytes()[..name_end]);
        name.make_ascii_lowercase();
        let attributes = &tag[name_end..];
        if self.skip_depth > 0 {
            if closing {
                self.skip_depth -= 1;
            } else {
                self.skip_depth += 1;
            }
            return;
        }
        match (&*name, closing) {
            ([b'h', level @ b'1'..=b'6'], false) => {
                self.begin(MarkdownBlockKind::Heading(level - b'0'));
            }
            ([b'h', b'1'..=b'6'], true) => {
                self.finish_if(|kind| matches!(kind, MarkdownBlockKind::Heading(_)));
            }
            (b"p", false) => {
                if !matches!(
                    self.current,
                    Some(MarkdownBlockKind::Item(_) | MarkdownBlockKind::Quote)
                ) {
                    self.begin(MarkdownBlockKind::Paragraph);
                }
            }
            (b"p" | b"div" | b"details" | b"summary" | b"table" | b"tr", _) => {
                self.finish_if(|kind| kind == MarkdownBlockKind::Paragraph);
            }
            (b"ul" | b"ol", false) => {
                self.finish_if(|kind| kind == MarkdownBlockKind::Paragraph);
                self.list_depth += 1;
            }
            (b"ul" | b"ol", true) => self.list_depth = self.list_depth.saturating_sub(1),
            (b"li", false) => self.begin(MarkdownBlockKind::Item(self.list_depth.max(1))),
            (b"li", true) => self.finish_if(|kind| matches!(kind, MarkdownBlockKind::Item(_))),
            (b"blockquote", false) => self.begin(MarkdownBlockKind::Quote),
            (b"blockquote", true) => self.finish_if(|kind| kind == MarkdownBlockKind::Quote),
            (b"pre", false) => {
                self.begin(MarkdownBlockKind::Code);
                self.preformatted = true;
            }
            (b"pre", true) => {
                self.finish_if(|kind| kind == MarkdownBlockKind::Code);
                self.preformatted = false;
            }
            (b"hr", _) => {
                self.finish();
                self.blocks.push(MarkdownBlock {
                    kind: MarkdownBlockKind::Rule,
                    markup: String::new(),
                });
            }
            (b"br", _) => self.push_markup("\n"),
            (b"b" | b"strong", false) => self.open_inline("<b>", "</b>"),
            (b"b" | b"strong", true) => self.close_inline("</b>"),
            (b"i" | b"em" | b"cite", false) => self.open_inline("<i>", "</i>"),
            (b"i" | b"em" | b"cite", true) => self.close_inline("</i>"),
            (b"s" | b"del" | b"strike", false) => self.open_inline("<s>", "</s>"),
            (b"s" | b"del" | b"strike", true) => self.close_inline("</s>"),
            (b"u" | b"ins", false) => self.open_inline("<u>", "</u>"),
            (b"u" | b"ins", true) => self.close_inline("</u>"),
            (b"sub", false) => self.open_inline("<sub>", "</sub>"),
            (b"sub", true) => self.close_inline("</sub>"),
            (b"sup", false) => self.open_inline("<sup>", "</sup>"),
            (b"sup", true) => self.close_inline("</sup>"),
            // Inside `<pre>` the block is already monospace.
            (b"code" | b"kbd" | b"samp" | b"tt", _) if self.preformatted => {}
            (b"code" | b"kbd" | b"samp" | b"tt", false) => self.open_inline(
                &format!("<tt><span foreground=\"{}\">", crate::ui::theme::CODE),
                "</span></tt>",
            ),
            (b"code" | b"kbd" | b"samp" | b"tt", true) => self.close_inline("</span></tt>"),
            (b"a", false) => {
                let is_headerlink = html_attribute(attributes, "class")
                    .is_some_and(|c| c == "headerlink" || c == "anchor" || c == "hash-link");
                if is_headerlink {
                    self.skip_depth = 1;
                } else if let Some(href) = html_attribute(attributes, "href") {
                    let href = glib::markup_escape_text(&decode_entities(href));
                    self.open_inline(&format!("<a href=\"{href}\">"), "</a>");
                }
            }
            (b"a", true) => self.close_inline("</a>"),
            _ => {}
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

/// Where the next tag starts: a `<` that opens an element, end tag, comment, or declaration,
/// not a lone `<` in text.
fn html_tag_start(html: &str) -> Option<usize> {
    html.match_indices('<')
        .map(|(index, _)| index)
        .find(|&index| {
            html[index + 1..]
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_alphabetic() || matches!(next, '/' | '!' | '?'))
        })
}

/// The index of the `>` closing the tag `html` starts with; quoted attribute values may hold `>`.
fn html_tag_end(html: &str) -> Option<usize> {
    let mut quote = None;
    for (index, character) in html.char_indices() {
        match (quote, character) {
            (None, '"' | '\'') => quote = Some(character),
            (Some(open), _) if character == open => quote = None,
            (None, '>') => return Some(index),
            _ => {}
        }
    }
    None
}

/// The raw value of attribute `wanted` in a tag's attribute text, entities still encoded.
fn html_attribute<'a>(mut attributes: &'a str, wanted: &str) -> Option<&'a str> {
    loop {
        attributes = attributes.trim_start_matches(|character: char| {
            character.is_ascii_whitespace() || character == '/'
        });
        if attributes.is_empty() {
            return None;
        }
        let name_end = attributes
            .find(|character: char| {
                character.is_ascii_whitespace() || matches!(character, '=' | '/')
            })
            .unwrap_or(attributes.len());
        let name = &attributes[..name_end];
        attributes = attributes[name_end..].trim_start();
        let mut value = "";
        if let Some(rest) = attributes.strip_prefix('=') {
            let rest = rest.trim_start();
            let (found, remainder) = match rest.chars().next() {
                Some(quote @ ('"' | '\'')) => {
                    let inner = &rest[1..];
                    let end = inner.find(quote).unwrap_or(inner.len());
                    (&inner[..end], inner.get(end + 1..).unwrap_or(""))
                }
                _ => {
                    let end = rest
                        .find(|character: char| character.is_ascii_whitespace())
                        .unwrap_or(rest.len());
                    (&rest[..end], &rest[end..])
                }
            };
            value = found;
            attributes = remainder;
        }
        if name.eq_ignore_ascii_case(wanted) {
            return Some(value);
        }
    }
}

/// Decodes the character references release notes use: the XML five, `&nbsp;`, a few
/// typographic names, and numeric references. Unknown names stay as written.
fn decode_entities(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.contains('&') {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        decoded.push_str(&rest[..start]);
        rest = &rest[start..];
        let character = rest[1..].find(';').and_then(|end| {
            let name = &rest[1..end + 1];
            let character = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                "mdash" => Some('—'),
                "ndash" => Some('–'),
                "hellip" => Some('…'),
                _ => name.strip_prefix('#').and_then(|number| {
                    match number.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => number.parse().ok(),
                    }
                    .and_then(char::from_u32)
                }),
            };
            character.map(|character| (character, end + 2))
        });
        match character {
            Some((character, length)) => {
                decoded.push(character);
                rest = &rest[length..];
            }
            None => {
                decoded.push('&');
                rest = &rest[1..];
            }
        }
    }
    decoded.push_str(rest);
    std::borrow::Cow::Owned(decoded)
}

pub(crate) fn markdown_blocks(markdown: &str) -> Vec<MarkdownBlock> {
    let mut output = MarkdownBlocks {
        blocks: Vec::new(),
        current: None,
        markup: String::with_capacity(markdown.len()),
        list_depth: 0,
        open_inline: Vec::new(),
        html_pending: String::new(),
        in_comment: false,
        preformatted: false,
        skip_depth: 0,
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
            Event::Text(text) => output.push_escaped(text.as_ref()),
            Event::InlineHtml(html) | Event::Html(html) => output.push_html(html.as_ref()),
            Event::End(TagEnd::HtmlBlock) => output.flush_html(),
            Event::SoftBreak => output.push_markup(" "),
            Event::HardBreak => output.push_markup("\n"),
            Event::TaskListMarker(checked) => {
                output.push_escaped(if checked { "☑ " } else { "☐ " });
            }
            _ => {}
        }
    }
    output.flush_html();
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
        if !release.tag.is_empty() {
            let tag = label(
                &release.tag,
                &["release-tag", "chip", "tinted", theme::tone(theme::VERSION)],
                0.0,
            );
            tag.set_halign(gtk::Align::Start);
            tag.set_tooltip_text(Some(&release.tag));
            section.append(&tag);
        }
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

/// Strips trailing permalink anchors such as `<a href="#...">¶</a>` or `#` and trailing
/// pilcrow symbols from heading markup.
fn strip_heading_permalinks(markup: &mut String) {
    if let Some(open) = markup.rfind("<a href=\"#")
        && markup[open..].ends_with("</a>")
    {
        let inner = &markup[open + 10..markup.len() - 4];
        if let Some(content_start) = inner.find('>') {
            let text = &inner[content_start + 1..];
            if text == "¶" || text == "#" || text.is_empty() {
                markup.truncate(open);
            }
        }
    }
    while markup.ends_with([' ', '\t', '\n', '¶']) {
        markup.pop();
    }
}

/// `tone` is the section's tone class; headings and bullets are drawn in it.
pub(crate) fn append_markdown_block(section: &gtk::Box, block: MarkdownBlock, tone: Option<&'static str>) {
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
#[path = "changelog_test.rs"]
mod tests;
