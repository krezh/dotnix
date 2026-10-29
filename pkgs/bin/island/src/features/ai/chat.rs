use super::{
    Ai,
    conversation::{Chat, Line, MODELS},
    harness,
    state::{Source, State},
    view::View,
};
use crate::{
    components::{card, item},
    copy,
    features::PanelCx,
    markdown,
    panel::{
        Block, Button, ButtonRow, Code, CodeLine, CodeMotion, IconButton, InputView, Item, Lead,
        Mark, Piece, Text, TextStyle, Tone, Transcript, Turn, on,
    },
    theme::{PEACH, RED},
};

const INPUT_LINES: usize = 6;
const LEAD: f32 = 40.0;
/// Ways to start, each filling the field with the start of a question.
const STARTERS: [(&str, &str); 3] = [
    ("Explain a command", "Explain what this command does: "),
    ("Write a jq filter", "Write a jq filter that "),
    ("Ask about Nix", "In Nix, how do I "),
];

fn styled(parsed: markdown::Text) -> Text {
    let links = parsed.link_ranges().into_iter().zip(parsed.links).collect();
    let spans = parsed
        .spans
        .into_iter()
        .map(|(range, style)| {
            let style = TextStyle {
                bold: style.bold,
                italic: style.italic,
                underline: style.underline,
                strike: style.strike,
                code: style.code,
                covered: style.spoiler,
                link: style.link.is_some(),
            };
            (range, style)
        })
        .collect();
    Text {
        spans,
        links,
        ..Text::new(Tone::Reading, parsed.text)
    }
}

fn copier(text: String) -> crate::panel::Handler {
    on(move |_, _| copy::text(text.clone()))
}

/// A reply's Markdown as text and code, each code block with a Copy chip.
pub(super) fn pieces(reply: &str) -> Vec<Piece> {
    markdown::blocks(reply)
        .into_iter()
        .map(|block| match block {
            markdown::Block::Paragraph(parsed) => Piece::Text(styled(parsed)),
            markdown::Block::Code { language, lines } => Piece::Code(Code {
                key: "".into(),
                heading: (!language.is_empty()).then(|| (language.into(), "".into())),
                copy: Some(copier(lines.join("\n"))),
                lines: lines.into_iter().map(CodeLine::plain).collect(),
                wrap: false,
                motion: CodeMotion::Still,
            }),
        })
        .collect()
}

/// Who answered, and how long it took or that it is still writing.
fn overline(line: &Line, streaming: bool) -> String {
    let model = MODELS[line.model.min(MODELS.len() - 1)].0;
    match (streaming, line.took_ms) {
        (true, _) => format!("{model} · writing…"),
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
            Turn::Theirs {
                overline: overline(line, streaming).into(),
                pieces: pieces(&line.text),
                streaming,
                actions,
            }
        })
        .collect();
    if chat.busy && chat.lines.last().is_some_and(|line| line.mine) {
        let model = chat.lines.last().map_or(chat.model, |line| line.model);
        turns.push(Turn::Waiting {
            overline: format!("{} · thinking…", MODELS[model.min(MODELS.len() - 1)].0).into(),
        });
    }
    turns
}

/// What an empty chat shows: the mark, what it is for, and a few ways to start.
fn welcome(p: &PanelCx<'_>) -> Vec<Block> {
    let starters = STARTERS
        .iter()
        .enumerate()
        .map(|(index, (label, start))| IconButton {
            id: format!("ai-chat-starter-{index}").into(),
            glyph: None,
            color: None,
            label: (*label).into(),
            active: false,
            badge: None,
            on_click: p.on_local::<Ai>(move |_, view| view.draft = (*start).to_owned()),
        })
        .collect();
    vec![
        Block::Item(
            Item {
                id: "ai-chat-welcome".into(),
                lead: Some(Lead {
                    mark: Some(Mark::Glyph(
                        harness::glyph(Source::Claude).to_path_buf(),
                        PEACH,
                    )),
                    tint: Some(PEACH),
                    glow: None,
                    faded: false,
                    size: Some(LEAD),
                    on_click: None,
                    top: false,
                }),
                overline: Some("Claude".into()),
                title: "Ask something quick".into(),
                title_color: None,
                wrap: false,
                detail: "".into(),
                tinted: Vec::new(),
                trail: Vec::new(),
            },
            vec![Block::Text(Text::new(
                Tone::Body,
                "Nothing here is saved. The chat clears with New chat, or after 10 quiet minutes.",
            ))],
        ),
        Block::Buttons(ButtonRow {
            buttons: starters,
            compact: true,
            indent: LEAD + item::LEAD_GAP,
            menu: None,
            end: false,
        }),
    ]
}

pub(super) fn panel(
    _ai: &Ai,
    state: &State,
    view: &View,
    p: &PanelCx<'_>,
) -> (Vec<Button>, Vec<Block>) {
    let chat = &state.chat;
    let model = chat.model.min(MODELS.len() - 1);
    let next_model = (model + 1) % MODELS.len();
    let buttons = vec![
        Button {
            id: "ai-chat-model".into(),
            label: MODELS[model].0.into(),
            active: false,
            toggle: false,
            fits: MODELS.iter().map(|(name, _)| (*name).into()).collect(),
            busy: false,
            enabled: !chat.busy,
            on_click: p.call::<Ai>(move |ai| ai.chat_model(next_model)),
        },
        Button {
            id: "ai-chat-new".into(),
            label: "New chat".into(),
            active: false,
            toggle: false,
            fits: Vec::new(),
            busy: false,
            enabled: !chat.lines.is_empty(),
            on_click: p.call::<Ai>(Ai::chat_new),
        },
    ];
    let mut blocks = if chat.lines.is_empty() {
        welcome(p)
    } else {
        vec![Block::Transcript(Transcript {
            id: "ai-chat".into(),
            turns: turns(chat, p),
            glyph: Some(harness::glyph(Source::Claude).to_path_buf()),
            color: PEACH,
            appear_from: view.seen_lines,
        })]
    };
    if let Some(error) = &chat.error {
        blocks.push(Block::caption(error.clone(), RED));
    }
    let ready = !chat.busy && !view.draft.trim().is_empty();
    // The field comes after the card, so it stays at the panel's foot.
    let mut blocks = vec![Block::Card(None, card::Size::Full, blocks)];
    blocks.push(Block::Input(InputView {
        id: "ai-chat-input".into(),
        text: view.draft.clone().into(),
        placeholder: "Ask Claude…".into(),
        max_lines: INPUT_LINES,
        send_glyph: p.glyph("ai-send"),
        on_send: ready
            .then(|| p.on_local::<Ai>(|ai, view| ai.chat_send(std::mem::take(&mut view.draft)))),
        busy: chat.busy,
        secret: false,
    }));
    (buttons, blocks)
}
