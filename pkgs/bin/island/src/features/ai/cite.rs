use std::ops::Range;

use crate::markdown;

/// Where a reply read something: the site's name and the page's address.
#[derive(Clone, Debug, PartialEq)]
pub struct Cited {
    pub site: String,
    pub url: String,
}

fn heading(line: &str) -> bool {
    let words = line
        .trim()
        .trim_start_matches('#')
        .trim()
        .trim_matches('*')
        .trim_end_matches(':')
        .trim_matches('*')
        .trim_end_matches(':')
        .to_lowercase();
    words == "sources" || words == "source"
}

fn entry(line: &str) -> bool {
    let line = line.trim_start();
    if ["- ", "* ", "+ ", "-", "*", "["]
        .iter()
        .any(|marker| line.starts_with(marker))
        || line.starts_with("http")
    {
        return true;
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && line[digits..].starts_with(['.', ')'])
}

/// The page a source line names: its first link, or else the first address written out.
fn address(line: &str) -> Option<String> {
    if let Some(url) = markdown::parse(line).links.into_iter().next() {
        return Some(url);
    }
    let start = line.find("https://").or_else(|| line.find("http://"))?;
    let url = line[start..]
        .split(|char: char| char.is_whitespace() || char == '>')
        .next()?
        .trim_end_matches(['.', ',', ')', ';']);
    Some(url.to_owned())
}

/// The page's host, without `www.`.
pub fn site(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    host.strip_prefix("www.").unwrap_or(host).to_owned()
}

/// The reply without the list of sources it ends in, and those sources in order. The list goes as
/// soon as its heading shows, so it never flashes up as text while it arrives.
pub fn split(reply: &str) -> (&str, Vec<Cited>) {
    let mut offset = 0;
    let mut found = None;
    for line in reply.split_inclusive('\n') {
        if heading(line) {
            found = Some(offset);
        }
        offset += line.len();
    }
    let Some(at) = found else {
        return (reply, Vec::new());
    };
    let after = &reply[at..];
    let mut lines = after.lines();
    lines.next();
    let listed: Vec<&str> = lines.filter(|line| !line.trim().is_empty()).collect();
    if !listed.iter().all(|line| entry(line)) {
        return (reply, Vec::new());
    }
    let cited = listed
        .iter()
        .filter_map(|line| address(line))
        .map(|url| Cited {
            site: site(&url),
            url,
        })
        .collect();
    (reply[..at].trim_end(), cited)
}

const RAISED: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

/// A `[n]` at the start of `rest`, not a link's words: n and its length.
fn marker(rest: &str) -> Option<(usize, usize)> {
    let inner = rest.strip_prefix('[')?;
    let digits = inner.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 2 || !inner[digits..].starts_with(']') {
        return None;
    }
    let length = digits + 2;
    if rest[length..].starts_with('(') {
        return None;
    }
    Some((inner[..digits].parse().ok()?, length))
}

/// `text` with each `[n]` outside code raised as a small number, and where each now is, with its
/// n.
pub fn raise(text: markdown::Text) -> (markdown::Text, Vec<(Range<usize>, usize)>) {
    if !text.text.contains('[') {
        return (text, Vec::new());
    }
    let in_code = |at: usize| {
        text.spans
            .iter()
            .any(|(range, style)| style.code && range.contains(&at))
    };
    let mut out = String::with_capacity(text.text.len());
    let mut moves: Vec<(usize, isize)> = Vec::new();
    let mut raised = Vec::new();
    let mut at = 0;
    while at < text.text.len() {
        let rest = &text.text[at..];
        if let Some((number, length)) = marker(rest).filter(|_| !in_code(at)) {
            let digits: String = number
                .to_string()
                .chars()
                .filter_map(|digit| digit.to_digit(10))
                .map(|digit| RAISED[digit as usize])
                .collect();
            // It sits against the word it is about, and apart from a number just before it.
            let space = usize::from(out.ends_with(' '));
            out.truncate(out.len() - space);
            let follows = raised
                .last()
                .is_some_and(|(range, _): &(Range<usize>, usize)| range.end == out.len());
            let parted = if follows { "\u{2009}" } else { "" };
            out.push_str(parted);
            let start = out.len();
            out.push_str(&digits);
            raised.push((start..out.len(), number));
            moves.push((
                at + length,
                (parted.len() + digits.len()) as isize - (length + space) as isize,
            ));
            at += length;
            continue;
        }
        let char = rest.chars().next().unwrap_or_default();
        out.push(char);
        at += char.len_utf8();
    }
    let shift = |position: usize| {
        let moved: isize = moves
            .iter()
            .filter(|(after, _)| *after <= position)
            .map(|(_, by)| by)
            .sum();
        (position as isize + moved).max(0) as usize
    };
    let spans = text
        .spans
        .into_iter()
        .map(|(range, style)| (shift(range.start)..shift(range.end), style))
        .collect();
    (
        markdown::Text {
            text: out,
            spans,
            links: text.links,
        },
        raised,
    )
}

#[cfg(test)]
#[path = "cite_test.rs"]
mod tests;
