//! Discord-style Markdown, as chat apps and bots pass it through. A card shows its text as one
//! flowing line, so line breaks join the lines up; a longer reply keeps its paragraphs and fenced
//! code. Anything that does not close is kept as written.

use std::{cell::RefCell, collections::HashMap, ops::Range, thread::LocalKey};

const REMEMBERED: usize = 256;

thread_local! {
    static TEXTS: RefCell<HashMap<String, Text>> = RefCell::default();
    static BLOCKS: RefCell<HashMap<String, Vec<Block>>> = RefCell::default();
}

fn remembered<T: Clone>(
    known: &'static LocalKey<RefCell<HashMap<String, T>>>,
    markdown: &str,
    read: impl FnOnce(&str) -> T,
) -> T {
    known.with_borrow_mut(|known| {
        if let Some(found) = known.get(markdown) {
            return found.clone();
        }
        if known.len() >= REMEMBERED {
            known.clear();
        }
        let found = read(markdown);
        known.insert(markdown.to_owned(), found.clone());
        found
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub code: bool,
    pub spoiler: bool,
    /// An index into [`Text::links`].
    pub link: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Text {
    pub text: String,
    /// In order, never overlapping.
    pub spans: Vec<(Range<usize>, Style)>,
    pub links: Vec<String>,
}

impl Text {
    fn push(&mut self, part: &str, style: Style) {
        if part.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(part);
        if style == Style::default() {
            return;
        }
        match self.spans.last_mut() {
            Some((range, last)) if *last == style && range.end == start => {
                range.end = self.text.len()
            }
            _ => self.spans.push((start..self.text.len(), style)),
        }
    }

    /// In the order of [`Text::links`].
    pub fn link_ranges(&self) -> Vec<Range<usize>> {
        (0..self.links.len())
            .map(|link| {
                let mut ranges = self
                    .spans
                    .iter()
                    .filter(|(_, style)| style.link == Some(link))
                    .map(|(range, _)| range);
                let first = ranges.next().map_or(0..0, Clone::clone);
                ranges.fold(first, |all, range| all.start..range.end)
            })
            .collect()
    }
}

pub fn parse(markdown: &str) -> Text {
    remembered(&TEXTS, markdown, read_text)
}

fn read_text(markdown: &str) -> Text {
    let mut text = Text::default();
    let line = flow(markdown);
    inline(&line, Style::default(), &mut text);
    text
}

/// One entry of a list, under the entries `depth` levels out.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub depth: usize,
    /// Its number in a numbered list; a bullet without.
    pub number: Option<u64>,
    pub text: Text,
}

/// One stretch of a longer reply.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    /// Its lines kept apart.
    Paragraph(Text),
    Heading(Text),
    /// Entries one after another, nested by how far each is indented.
    List(Vec<Item>),
    /// Quoted lines, kept apart.
    Quote(Text),
    /// Cells under column headings.
    Table {
        head: Vec<Text>,
        rows: Vec<Vec<Text>>,
    },
    /// Kept line for line, with the language named after the opening fence, if any.
    Code {
        language: String,
        lines: Vec<String>,
    },
}

fn styled_text(source: &str) -> Text {
    let mut text = Text::default();
    inline(source, Style::default(), &mut text);
    text
}

/// A list entry's start: how far it is indented, its number if numbered, and what follows.
fn list_item(raw: &str) -> Option<(usize, Option<u64>, &str)> {
    let indent: usize = raw
        .chars()
        .take_while(|char| char.is_whitespace())
        .map(|char| if char == '\t' { 4 } else { 1 })
        .sum();
    let line = raw.trim_start();
    if let Some(rest) = ["- ", "* ", "+ "]
        .iter()
        .find_map(|marker| line.strip_prefix(marker))
    {
        return Some((indent, None, rest));
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let rest = line[digits..]
        .strip_prefix(". ")
        .or_else(|| line[digits..].strip_prefix(") "))?;
    (digits > 0 && digits < 10).then(|| (indent, line[..digits].parse().ok(), rest))
}

fn table_row(line: &str) -> Vec<String> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = line.strip_suffix('|').unwrap_or(line);
    line.split('|').map(|cell| cell.trim().to_owned()).collect()
}

fn table_rule(line: &str) -> bool {
    let cells = table_row(line);
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let cell = cell.trim_matches(':');
            !cell.is_empty() && cell.chars().all(|char| char == '-')
        })
}

/// What [`read_blocks`] is gathering.
enum Open {
    None,
    Paragraph(Vec<String>),
    /// Each entry's indent, number and words, and whether a blank line came after the last.
    List(Vec<(usize, Option<u64>, String)>, bool),
    Quote(Vec<String>),
    Table(Vec<String>),
}

fn close(open: &mut Open, blocks: &mut Vec<Block>) {
    match std::mem::replace(open, Open::None) {
        Open::None => {}
        Open::Paragraph(lines) => blocks.push(Block::Paragraph(styled_text(&lines.join("\n")))),
        Open::Quote(lines) => blocks.push(Block::Quote(styled_text(&lines.join("\n")))),
        Open::List(entries, _) => blocks.push(Block::List(nested(entries))),
        Open::Table(lines) => {
            if lines.len() >= 2 && table_rule(&lines[1]) {
                let cells = |line: &str| {
                    table_row(line)
                        .iter()
                        .map(|cell| styled_text(cell))
                        .collect()
                };
                blocks.push(Block::Table {
                    head: cells(&lines[0]),
                    rows: lines[2..].iter().map(|line| cells(line)).collect(),
                });
            } else {
                blocks.push(Block::Paragraph(styled_text(&lines.join("\n"))));
            }
        }
    }
}

/// Depths from indents, each deeper indent one level in; numbers counting on from the first of
/// each run.
fn nested(entries: Vec<(usize, Option<u64>, String)>) -> Vec<Item> {
    let mut indents: Vec<usize> = Vec::new();
    let mut last: Vec<Option<u64>> = Vec::new();
    entries
        .into_iter()
        .map(|(indent, number, words)| {
            while indents.last().is_some_and(|open| indent < *open) {
                indents.pop();
            }
            if indents.last() != Some(&indent) {
                indents.push(indent);
            }
            let depth = indents.len() - 1;
            last.truncate(depth + 1);
            last.resize(depth + 1, None);
            let number = number.map(|given| last[depth].map_or(given, |before| before + 1));
            last[depth] = number;
            Item {
                depth,
                number,
                text: styled_text(&words),
            }
        })
        .collect()
}

/// Paragraphs split at blank lines, with headings, lists, quotes, tables and fenced code on their
/// own; a fence still open, such as one arriving, runs to the end.
pub fn blocks(markdown: &str) -> Vec<Block> {
    remembered(&BLOCKS, markdown, read_blocks)
}

fn read_blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut open = Open::None;
    let mut code: Option<(String, Vec<String>)> = None;
    for raw in markdown.lines() {
        if let Some(rest) = raw.trim_start().strip_prefix("```") {
            match code.take() {
                Some((language, lines)) => blocks.push(Block::Code { language, lines }),
                None => {
                    close(&mut open, &mut blocks);
                    code = Some((rest.trim().to_owned(), Vec::new()));
                }
            }
            continue;
        }
        if let Some((_, code_lines)) = &mut code {
            code_lines.push(raw.trim_end().to_owned());
            continue;
        }
        let line = raw.trim();
        if line.is_empty() {
            match &mut open {
                // A blank line between entries keeps the list going.
                Open::List(_, blank) => *blank = true,
                _ => close(&mut open, &mut blocks),
            }
            continue;
        }
        if let Some(heading) = ["### ", "## ", "# "]
            .iter()
            .find_map(|marker| line.strip_prefix(marker))
        {
            close(&mut open, &mut blocks);
            blocks.push(Block::Heading(styled_text(heading.trim())));
            continue;
        }
        if let Some((indent, number, words)) = list_item(raw) {
            if !matches!(open, Open::List(..)) {
                close(&mut open, &mut blocks);
                open = Open::List(Vec::new(), false);
            }
            if let Open::List(entries, blank) = &mut open {
                entries.push((indent, number, words.trim().to_owned()));
                *blank = false;
            }
            continue;
        }
        if let Open::List(entries, blank) = &mut open
            && !*blank
            && raw.starts_with(char::is_whitespace)
            && let Some(last) = entries.last_mut()
        {
            last.2.push(' ');
            last.2.push_str(line);
            continue;
        }
        if let Some(quoted) = [">>> ", "> "]
            .iter()
            .find_map(|marker| line.strip_prefix(marker))
            .or_else(|| (line == ">").then_some(""))
        {
            if !matches!(open, Open::Quote(_)) {
                close(&mut open, &mut blocks);
                open = Open::Quote(Vec::new());
            }
            if let Open::Quote(lines) = &mut open {
                lines.push(quoted.trim_end().to_owned());
            }
            continue;
        }
        if line.starts_with('|') {
            if !matches!(open, Open::Table(_)) {
                close(&mut open, &mut blocks);
                open = Open::Table(Vec::new());
            }
            if let Open::Table(lines) = &mut open {
                lines.push(line.to_owned());
            }
            continue;
        }
        if !matches!(open, Open::Paragraph(_)) {
            close(&mut open, &mut blocks);
            open = Open::Paragraph(Vec::new());
        }
        if let Open::Paragraph(lines) = &mut open {
            lines.push(line.strip_prefix("-# ").unwrap_or(line).to_owned());
        }
    }
    close(&mut open, &mut blocks);
    if let Some((language, code_lines)) = code {
        blocks.push(Block::Code {
            language,
            lines: code_lines,
        });
    }
    blocks
}

/// `text` without its marks, with each link's address after its words, unless they are the
/// address.
fn unmarked(text: &Text) -> String {
    let mut plain = text.text.clone();
    // From the end, so the ranges before stay where they are.
    for (range, url) in text.link_ranges().into_iter().zip(&text.links).rev() {
        if text.text[range.clone()] != *url {
            plain.insert_str(range.end, &format!(" ({url})"));
        }
    }
    plain
}

/// The text as [`blocks`] shows it, without the marks: blocks apart by a blank line, list entries
/// and table rows a line each, code line for line, and each link's address after its words,
/// unless they are the address.
pub fn plain(markdown: &str) -> String {
    blocks(markdown)
        .into_iter()
        .map(|block| match block {
            Block::Paragraph(text) | Block::Heading(text) | Block::Quote(text) => unmarked(&text),
            Block::List(items) => items
                .iter()
                .map(|item| {
                    let marker = item
                        .number
                        .map_or_else(|| "•".to_owned(), |number| format!("{number}."));
                    format!(
                        "{}{marker} {}",
                        "  ".repeat(item.depth),
                        unmarked(&item.text)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Block::Table { head, rows } => std::iter::once(&head)
                .chain(&rows)
                .map(|row| row.iter().map(unmarked).collect::<Vec<_>>().join("\t"))
                .collect::<Vec<_>>()
                .join("\n"),
            Block::Code { lines, .. } => lines.join("\n"),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn unblock(line: &str) -> (&str, bool) {
    let trimmed = line.trim_start();
    for marker in ["### ", "## ", "# "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return (rest, true);
        }
    }
    for marker in ["-# ", ">>> ", "> "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return (rest, false);
        }
    }
    (trimmed, false)
}

/// A line that ends a sentence is joined to the next with a space, any other with a dot.
fn flow(markdown: &str) -> String {
    let mut joined = String::new();
    let mut fenced = false;
    for raw in markdown.lines() {
        if raw.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        let (line, heading) = if fenced {
            (raw.trim(), false)
        } else {
            unblock(raw)
        };
        let line = match line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            Some(item) if !fenced => format!("• {item}"),
            _ => line.to_owned(),
        };
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if !joined.is_empty() {
            let sentence = joined.ends_with(['.', '!', '?', ':', ';', ',', '…']);
            joined.push_str(if sentence || fenced { " " } else { " · " });
        }
        if heading {
            joined.push_str("**");
            joined.push_str(line);
            joined.push_str("**");
        } else {
            joined.push_str(line);
        }
    }
    joined
}

/// A `_` inside a word does not start or end italics.
fn wordy(char: Option<char>) -> bool {
    char.is_some_and(char::is_alphanumeric)
}

/// Not at once, and for a single `*` or `_` not after a space, nor, for `_`, inside a word. In a
/// longer run of the same character, such as `***`, it closes at the run's end, leaving the start
/// to what opened inside.
fn closing_at(rest: &str, closing: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = rest[from..].find(closing) {
        let mut at = from + found;
        let repeated = closing.chars().next().unwrap_or_default();
        if closing.len() > 1 {
            while rest[at + closing.len()..].starts_with(repeated) {
                at += repeated.len_utf8();
            }
        }
        let before = rest[..at].chars().next_back();
        let after = rest[at + closing.len()..].chars().next();
        let single = closing == "*" || closing == "_";
        let fits = at > 0
            && !(single && before.is_some_and(char::is_whitespace))
            && !(closing == "_" && wordy(after))
            && !(closing == "*" && rest[at..].starts_with("**"));
        if fits {
            return Some(at);
        }
        from = at + closing.len();
    }
    None
}

fn inline(source: &str, style: Style, text: &mut Text) {
    let mut plain = String::new();
    let mut at = 0;
    let flush = |plain: &mut String, text: &mut Text| {
        text.push(plain, style);
        plain.clear();
    };
    while at < source.len() {
        let rest = &source[at..];
        let previous = source[..at].chars().next_back();
        if let Some(escaped) = rest
            .strip_prefix('\\')
            .and_then(|after| after.chars().next())
            && "\\*_~`|[]()<>#-".contains(escaped)
        {
            plain.push(escaped);
            at += 1 + escaped.len_utf8();
            continue;
        }
        if let Some(emoji) = custom_emoji(rest) {
            plain.push_str(&format!(":{}:", emoji.0));
            at += emoji.1;
            continue;
        }
        let code = ["```", "`"].into_iter().find_map(|fence| {
            let inner = rest.strip_prefix(fence)?;
            let end = inner.find(fence).filter(|end| *end > 0)?;
            Some((inner[..end].trim(), fence.len() * 2 + end))
        });
        if let Some((content, length)) = code {
            flush(&mut plain, text);
            text.push(
                content,
                Style {
                    code: true,
                    ..style
                },
            );
            at += length;
            continue;
        }
        if let Some((label, url, length)) = link(rest) {
            flush(&mut plain, text);
            let index = text.links.len();
            text.links.push(url.to_owned());
            inline(
                label,
                Style {
                    link: Some(index),
                    underline: true,
                    ..style
                },
                text,
            );
            at += length;
            continue;
        }
        let pairs: [(&str, fn(Style) -> Style); 6] = [
            ("**", |style| Style {
                bold: true,
                ..style
            }),
            ("__", |style| Style {
                underline: true,
                ..style
            }),
            ("~~", |style| Style {
                strike: true,
                ..style
            }),
            ("||", |style| Style {
                spoiler: true,
                ..style
            }),
            ("*", |style| Style {
                italic: true,
                ..style
            }),
            ("_", |style| Style {
                italic: true,
                ..style
            }),
        ];
        let opened = pairs.into_iter().find_map(|(marker, styled)| {
            let inner = rest.strip_prefix(marker)?;
            if marker.len() == 1 && inner.starts_with(char::is_whitespace) {
                return None;
            }
            if marker == "_" && wordy(previous) {
                return None;
            }
            let end = closing_at(inner, marker)?;
            Some((&inner[..end], styled(style), marker.len() * 2 + end))
        });
        if let Some((inner, styled, length)) = opened {
            flush(&mut plain, text);
            inline(inner, styled, text);
            at += length;
            continue;
        }
        let char = rest.chars().next().unwrap_or_default();
        plain.push(char);
        at += char.len_utf8();
    }
    flush(&mut plain, text);
}

/// `[label](url)`: its label, its URL and its length.
fn link(rest: &str) -> Option<(&str, &str, usize)> {
    let inner = rest.strip_prefix('[')?;
    let label_end = inner.find(']')?;
    if !inner[label_end..].starts_with("](") {
        return None;
    }
    let after = &inner[label_end + 2..];
    let url_end = after.find(')')?;
    let url = after[..url_end]
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>');
    let label = &inner[..label_end];
    (!label.is_empty() && (url.starts_with("http://") || url.starts_with("https://"))).then_some((
        label,
        url,
        1 + label_end + 2 + url_end + 1,
    ))
}

/// A custom emoji at the start of `rest`, `<:name:id>` or `<a:name:id>`: its name and its length.
fn custom_emoji(rest: &str) -> Option<(&str, usize)> {
    let inner = rest
        .strip_prefix("<a:")
        .or_else(|| rest.strip_prefix("<:"))?;
    let end = inner.find('>')?;
    let (name, id) = inner[..end].split_once(':')?;
    (!name.is_empty() && !id.is_empty() && id.chars().all(|char| char.is_ascii_digit()))
        .then_some((name, rest.len() - inner.len() + end + 1))
}

#[cfg(test)]
#[path = "markdown_test.rs"]
mod tests;
