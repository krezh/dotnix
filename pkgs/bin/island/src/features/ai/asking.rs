use std::time::Duration;

use super::{
    Ai, format, harness,
    session::{Answer, Ask, Request},
    state::Source,
    view::View,
};
use crate::{
    components::{
        card::{Look, Motion},
        item,
    },
    features::PanelCx,
    panel::{
        Block, ButtonRow, Code, CodeLine, CodeMotion, Countdown, FrameView, IconButton, Item, Lead,
        Mark, Text, Tone, Trail,
    },
    theme::{OVERLAY2, PEACH, SUBTEXT1},
};

/// As wide as a notification, so the two read as one kind of thing.
const MIN_W: f32 = 380.0;
const LEAD: f32 = 36.0;

fn title(request: &Request, picks_step: usize) -> String {
    match &request.ask {
        Ask::Permission { tool, .. } => match tool.as_str() {
            "Bash" => "Run a command".to_owned(),
            "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => "Change a file".to_owned(),
            "Read" => "Read a file".to_owned(),
            "WebFetch" => "Fetch a page".to_owned(),
            other => format!("Use {other}"),
        },
        Ask::Questions { questions, .. } => questions
            .get(picks_step)
            .map(|question| question.text.clone())
            .unwrap_or_default(),
    }
}

fn button(id: String, label: &str, active: bool, on_click: crate::panel::Handler) -> IconButton {
    IconButton {
        id: id.into(),
        glyph: None,
        rising: None,
        label: label.to_owned().into(),
        active,
        badge: None,
        on_click,
    }
}

fn buttons(buttons: Vec<IconButton>) -> Block {
    Block::Buttons(ButtonRow {
        buttons,
        compact: true,
        indent: LEAD + item::LEAD_GAP,
    })
}

fn permission_blocks(request: &Request, at: &str, p: &PanelCx<'_>) -> Vec<Block> {
    let Ask::Permission {
        subject,
        reason,
        suggestion,
        ..
    } = &request.ask
    else {
        return Vec::new();
    };
    let id = request.id;
    let answer = |answer: Answer| p.call::<Ai>(move |ai| ai.answer(id, answer.clone()));
    let mut row = vec![button(format!("{at}-deny"), "Deny", false, answer(Answer::Deny))];
    if suggestion.is_some() {
        row.push(button(
            format!("{at}-always"),
            "Always allow",
            false,
            answer(Answer::Always),
        ));
    }
    row.push(button(
        format!("{at}-allow"),
        "Allow",
        true,
        answer(Answer::Allow),
    ));
    let mut under = vec![Block::Code(Code {
        key: "".into(),
        heading: None,
        lines: vec![CodeLine::plain(subject.clone())],
        wrap: true,
        copy: None,
        motion: CodeMotion::Still,
    })];
    under.push(Block::Text(Text {
        min_width: Some(MIN_W - LEAD - item::LEAD_GAP),
        ..Text::new(Tone::Caption(OVERLAY2), reason.clone())
    }));
    vec![Block::Item(item_for(request, at, 0), under), buttons(row)]
}

fn question_blocks(request: &Request, at: &str, view: &View, p: &PanelCx<'_>) -> Vec<Block> {
    let Ask::Questions { questions, .. } = &request.ask else {
        return Vec::new();
    };
    let id = request.id;
    let picks = view.picks(id);
    let step = picks.step.min(questions.len().saturating_sub(1));
    let question = &questions[step];
    let chosen = picks.chosen.get(step).cloned().unwrap_or_default();
    let last = step + 1 == questions.len();
    let multi = question.multi;
    let mut under = vec![Block::Text(Text {
        min_width: Some(MIN_W - LEAD - item::LEAD_GAP),
        ..Text::new(
            Tone::Caption(SUBTEXT1),
            format!(
                "{} · {} of {}{}",
                question.header,
                step + 1,
                questions.len(),
                if multi { " · pick any" } else { "" }
            ),
        )
    })];
    for (index, (label, description)) in question.options.iter().enumerate() {
        let picked = chosen.contains(&index);
        under.push(Block::Frame(
            FrameView {
                // Named for its question too, so the next question's options rise in.
                id: format!("{at}-q{step}-option-{index}").into(),
                look: Look::Inset,
                on_click: Some(p.on_local::<Ai>(move |_, view| view.pick(id, index, multi))),
                on_dismiss: None,
                countdown: None,
                selected: picked,
                motion: Motion::Appear,
            },
            vec![Block::Item(
                Item {
                    id: format!("{at}-q{step}-option-{index}-item").into(),
                    lead: None,
                    overline: None,
                    title: label.clone().into(),
                    title_color: None,
                    detail: description.clone().into(),
                    trail: Vec::new(),
                },
                Vec::new(),
            )],
        ));
    }
    let labels: Vec<Vec<String>> = questions
        .iter()
        .map(|question| question.options.iter().map(|(label, _)| label.clone()).collect())
        .collect();
    // Next and Send wait for a pick: plain until then, and doing nothing when pressed.
    let ready = !chosen.is_empty();
    let next = if last {
        let send = p.on_local::<Ai>(move |ai, view| {
            let picks = view.picks(id);
            if picks.chosen.get(picks.step).is_none_or(Vec::is_empty) {
                return;
            }
            let picked = labels
                .iter()
                .enumerate()
                .map(|(step, options)| {
                    picks
                        .chosen
                        .get(step)
                        .into_iter()
                        .flatten()
                        .filter_map(|index| options.get(*index).cloned())
                        .collect()
                })
                .collect();
            ai.answer(id, Answer::Picked(picked));
        });
        button(format!("{at}-send"), "Send answers", ready, send)
    } else {
        button(
            format!("{at}-next"),
            "Next",
            ready,
            p.on_local::<Ai>(move |_, view| view.next_question(id)),
        )
    };
    let terminal = button(
        format!("{at}-terminal"),
        "Reply in terminal",
        false,
        p.call::<Ai>(move |ai| ai.answer(id, Answer::Terminal)),
    );
    vec![
        Block::Item(item_for(request, at, step), under),
        buttons(vec![terminal, next]),
    ]
}

fn item_for(request: &Request, at: &str, step: usize) -> Item {
    let now = format::now();
    Item {
        id: format!("{at}-item").into(),
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
        }),
        overline: Some(format!("Claude Code · {}", request.project).into()),
        title: title(request, step).into(),
        title_color: None,
        detail: "".into(),
        trail: vec![Trail::Faint(format::ago(request.received_at, now).into())],
    }
}

/// One request as a card of its own, with what it asks and the ways to answer. `countdown` shows
/// the time it has left, as a pop-up does; `leaving` slides out one just answered.
pub(super) fn frame(
    request: &Request,
    view: &View,
    p: &PanelCx<'_>,
    countdown: bool,
    leaving: bool,
) -> Block {
    let at = format!("ai-request-{}", request.id);
    let blocks = match request.ask {
        Ask::Permission { .. } => permission_blocks(request, &at, p),
        Ask::Questions { .. } => question_blocks(request, &at, view, p),
    };
    let total = u64::try_from(request.expires_at - request.received_at).unwrap_or(0);
    Block::Frame(
        FrameView {
            id: at.clone().into(),
            look: Look::Raised { urgent: false },
            on_click: None,
            on_dismiss: None,
            countdown: (countdown && total > 0).then(|| Countdown {
                left: 1.0,
                running: Some(Duration::from_millis(total)),
                round: 0,
            }),
            selected: false,
            motion: if leaving {
                Motion::Leave
            } else {
                Motion::Appear
            },
        },
        blocks,
    )
}
