use super::{
    Ai,
    cite::{self, Cited},
    conversation::{Chat, Line, MODELS, Steps, model_name},
    harness,
    state::{self, State},
    view::View,
};
use crate::{
    components::{card, item, tabs},
    copy,
    features::PanelCx,
    launch, markdown,
    panel::{
        Block, Button, ButtonRow, Cell, Choice, Code, CodeLine, CodeMotion, Column, Detail,
        Dropdown, Entry, IconButton, InputView, Item, Lead, Mark, Piece, SharedString, Source,
        Table, TableRow, Text, TextStyle, Tone, Transcript, Turn, Width, on,
    },
    theme::{OVERLAY2, PEACH, RED, SUBTEXT0, TEAL, TEXT},
    units::plural,
};

const INPUT_LINES: usize = 6;
const LEAD: f32 = 40.0;
/// Ways to start, each filling the field with the start of a question.
const STARTERS: [(&str, &str); 3] = [
    ("Explain a command", "Explain what this command does: "),
    ("Write a jq filter", "Write a jq filter that "),
    ("Ask about Nix", "In Nix, how do I "),
];

/// Its marks as styles, each `[n]` raised in teal and opening source n when there is one, and all
/// of it bold when `strong`.
fn styled(parsed: markdown::Text, cited: &[Cited], strong: bool) -> Text {
    let (parsed, raised) = cite::raise(parsed);
    let mut links: Vec<_> = parsed.link_ranges().into_iter().zip(parsed.links).collect();
    let mut spans: Vec<_> = raised
        .iter()
        .map(|(range, number)| {
            let source = number.checked_sub(1).and_then(|index| cited.get(index));
            if let Some(source) = source {
                links.push((range.clone(), source.url.clone()));
            }
            let style = TextStyle {
                bold: true,
                color: Some(TEAL),
                link: source.is_some(),
                ..TextStyle::default()
            };
            (range.clone(), style)
        })
        .collect();
    spans.extend(parsed.spans.into_iter().map(|(range, style)| {
        let style = TextStyle {
            bold: style.bold || strong,
            italic: style.italic,
            underline: style.underline,
            strike: style.strike,
            code: style.code,
            covered: style.spoiler,
            link: style.link.is_some(),
            color: None,
        };
        (range, style)
    }));
    if strong {
        spans.push((
            0..parsed.text.len(),
            TextStyle {
                bold: true,
                ..TextStyle::default()
            },
        ));
    }
    Text {
        spans,
        links,
        ..Text::new(Tone::Reading, parsed.text)
    }
}

fn copier(text: String) -> crate::panel::Handler {
    on(move |_| copy::text(text.clone()))
}

/// Short and starting with a figure, such as an amount or a count.
fn figure(cell: &str) -> bool {
    cell.starts_with(|first: char| first.is_ascii_digit() || "$€£~+-−≈<>".contains(first))
        && cell.chars().filter(|char| char.is_alphabetic()).count() <= 4
}

fn table(head: Vec<markdown::Text>, rows: Vec<Vec<markdown::Text>>) -> Table {
    let columns = head
        .iter()
        .enumerate()
        .map(|(index, title)| Column {
            title: title.text.clone().into(),
            width: Width::Share(1.0),
            end: !rows.is_empty()
                && rows.iter().all(|row| {
                    row.get(index)
                        .is_none_or(|cell| cell.text.is_empty() || figure(&cell.text))
                }),
        })
        .collect();
    let rows = rows
        .into_iter()
        .map(|row| {
            let mut cells: Vec<Cell> = row
                .into_iter()
                .map(|cell| Cell::text(cell.text, TEXT))
                .collect();
            cells.resize_with(head.len(), || Cell::text("", TEXT));
            TableRow::cells(cells, None)
        })
        .collect();
    Table { columns, rows }
}

/// A reply's Markdown as text, lists, quotes, tables and code, each code block with a Copy chip.
fn pieces(reply: &str, cited: &[Cited]) -> Vec<Piece> {
    markdown::blocks(reply)
        .into_iter()
        .map(|block| match block {
            markdown::Block::Paragraph(parsed) => Piece::Text(styled(parsed, cited, false)),
            markdown::Block::Heading(parsed) => Piece::Text(styled(parsed, cited, true)),
            markdown::Block::Quote(parsed) => Piece::Quote(styled(parsed, cited, false)),
            markdown::Block::List(items) => Piece::List(
                items
                    .into_iter()
                    .map(|item| Entry {
                        depth: item.depth,
                        number: item.number,
                        text: styled(item.text, cited, false),
                    })
                    .collect(),
            ),
            markdown::Block::Table { head, rows } => Piece::Table(table(head, rows)),
            markdown::Block::Code { language, lines } => Piece::Code(Code {
                heading: (!language.is_empty()).then(|| (language.into(), "".into())),
                copy: Some(copier(lines.join("\n"))),
                lines: lines.into_iter().map(CodeLine::plain).collect(),
                wrap: false,
                motion: CodeMotion::Still,
                scroll: None,
            }),
        })
        .collect()
}

/// What a reply looked up: the first search, how many more, and how many pages it read.
fn detail(steps: &Steps, p: &PanelCx<'_>) -> Option<Detail> {
    let mut parts: Vec<(SharedString, u32)> = Vec::new();
    if let Some(first) = steps.searched.first() {
        parts.push(("Searched ".into(), OVERLAY2));
        parts.push((format!("“{first}”").into(), SUBTEXT0));
        let more = steps.searched.len() - 1;
        if more > 0 {
            parts.push((format!(" and {more} more").into(), OVERLAY2));
        }
    }
    if steps.read > 0 {
        let pages = plural(steps.read, "page", "pages");
        parts.push((
            if parts.is_empty() {
                format!("Read {pages}")
            } else {
                format!(" · read {pages}")
            }
            .into(),
            OVERLAY2,
        ));
    }
    (!parts.is_empty()).then(|| Detail {
        glyph: p.glyph("ai-search"),
        tint: TEAL,
        parts,
    })
}

/// Who answered, and how long it took or that it is still writing.
fn overline(line: &Line, streaming: bool, activity: Option<&str>) -> String {
    let model = model_name(line.model);
    match (streaming, line.took_ms) {
        (true, _) => format!("{model} · {}…", activity.unwrap_or("writing")),
        (false, Some(took)) => format!("{model} · {:.1}s", took as f64 / 1000.0),
        (false, None) => model.to_owned(),
    }
}

fn turns(chat: &Chat, p: &PanelCx<'_>) -> Vec<Turn> {
    let count = chat.lines.len();
    let mut turns: Vec<Turn> = chat
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if line.mine {
                return Turn::Mine(Text::new(Tone::Reading, line.text.clone()));
            }
            let last = index + 1 == count;
            let streaming = chat.busy && last;
            let mut actions = Vec::new();
            if !streaming {
                actions.push(("Copy".into(), copier(markdown::plain(&line.text))));
                if last && chat.error.is_some() {
                    actions.push(("Retry".into(), p.call::<Ai>(Ai::chat_retry)));
                }
            }
            let (body, cited) = cite::split(&line.text);
            let sources = cited
                .iter()
                .map(|source| {
                    let url = source.url.clone();
                    Source {
                        label: source.site.clone().into(),
                        on_click: on(move |_| launch::open(&url, "Browser")),
                    }
                })
                .collect();
            Turn::Theirs {
                overline: overline(line, streaming, chat.activity).into(),
                detail: detail(&line.steps, p),
                pieces: pieces(body, &cited),
                streaming,
                actions,
                sources,
                source_tint: TEAL,
            }
        })
        .collect();
    if chat.busy && chat.lines.last().is_some_and(|line| line.mine) {
        let model = chat.lines.last().map_or(chat.model, |line| line.model);
        turns.push(Turn::Waiting {
            overline: format!(
                "{} · {}…",
                model_name(model),
                chat.activity.unwrap_or("thinking")
            )
            .into(),
        });
    }
    turns
}

/// What an empty chat shows: the mark, what it is for, and a few ways to start.
fn welcome(p: &PanelCx<'_>) -> Vec<Block> {
    let starters = STARTERS
        .iter()
        .map(|(label, start)| {
            IconButton::text(
                *label,
                false,
                p.on_local::<Ai>(move |_, view| view.draft = (*start).to_owned()),
            )
        })
        .collect();
    vec![
        Block::Item(
            Item {
                id: "ai-chat-welcome".into(),
                lead: Some(Lead {
                    mark: Some(Mark::Glyph(
                        harness::glyph(state::Source::Claude).to_path_buf(),
                        PEACH,
                    )),
                    tint: Some(PEACH),
                    size: Some(LEAD),
                    ..Default::default()
                }),
                overline: Some("Claude".into()),
                title: "Ask something quick".into(),
                ..Default::default()
            },
            vec![Block::Text(Text::new(
                Tone::Body,
                "Nothing here is saved. The chat clears with New chat, or after 10 quiet minutes.",
            ))],
        ),
        Block::Buttons(ButtonRow {
            lead: None,
            fill: false,
            buttons: starters,
            compact: true,
            indent: LEAD + item::LEAD_GAP,
            menu: None,
            end: false,
        }),
    ]
}

/// Which model answers next.
pub(super) fn pick(state: &State, view: &View, p: &PanelCx<'_>) -> Dropdown {
    let chat = &state.chat;
    Dropdown {
        id: "ai-chat-model".into(),
        choices: MODELS
            .iter()
            .map(|(name, _)| Choice {
                label: (*name).into(),
                glyph: None,
                detail: None,
            })
            .collect(),
        selected: Some(chat.model.min(MODELS.len() - 1)),
        open: view.picking_model,
        height: Some(tabs::STRIP_H),
        on_toggle: p.on_local::<Ai>(|_, view| view.picking_model = !view.picking_model),
        on_select: p.on_local_with::<Ai, usize>(|ai, view, index| {
            view.picking_model = false;
            ai.chat_model(index.min(MODELS.len() - 1));
        }),
    }
}

pub(super) fn panel(
    _ai: &Ai,
    state: &State,
    view: &View,
    p: &PanelCx<'_>,
) -> (Vec<Button>, Vec<Block>) {
    let chat = &state.chat;
    let buttons =
        vec![Button::new("New chat", p.call::<Ai>(Ai::chat_new)).enabled(!chat.lines.is_empty())];
    let error = chat
        .error
        .as_ref()
        .map(|error| Block::caption(error.clone(), RED));
    // The turns sit on the panel itself; only the welcome has a card.
    let mut blocks = if chat.lines.is_empty() {
        let mut welcome = welcome(p);
        welcome.extend(error);
        vec![Block::Card(None, card::Size::Full, welcome)]
    } else {
        let mut blocks = vec![Block::Transcript(Transcript {
            turns: turns(chat, p),
            glyph: Some(harness::glyph(state::Source::Claude).to_path_buf()),
            color: PEACH,
            appear_from: view.seen_lines,
        })];
        blocks.extend(error);
        blocks
    };
    let ready = !chat.busy && !view.draft.trim().is_empty();
    blocks.push(Block::Input(InputView {
        text: view.draft.clone().into(),
        placeholder: "Ask Claude…".into(),
        max_lines: INPUT_LINES,
        send_glyph: p.glyph("ai-send"),
        on_send: ready
            .then(|| p.on_local::<Ai>(|ai, view| ai.chat_send(std::mem::take(&mut view.draft)))),
        busy: chat.busy,
        secret: false,
        focused: true,
        on_focus: None,
    }));
    (buttons, blocks)
}
