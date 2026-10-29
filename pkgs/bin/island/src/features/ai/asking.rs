use std::time::Duration;

use super::{
    Ai, format, harness,
    session::{Answer, Ask, Request},
    state::{Hold, Source},
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
    theme::{ACCENT, OVERLAY2, PEACH, SUBTEXT1},
};

/// As wide as a notification, so the two read as one kind of thing.
const MIN_W: f32 = 380.0;
const LEAD: f32 = 36.0;
/// Jelly in Claude's mark's place: large enough for its face, as in the panel's header.
const JELLY_LEAD: f32 = 52.0;
/// The check box or radio dot before each option.
const CHOICE: f32 = 20.0;

/// The lead's side, which the text and buttons under the title line up past.
fn lead(view: &View) -> f32 {
    if view.pose().is_some() {
        JELLY_LEAD
    } else {
        LEAD
    }
}

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
        color: None,
        label: label.to_owned().into(),
        active,
        badge: None,
        on_click,
    }
}

fn buttons(buttons: Vec<IconButton>, lead: f32) -> Block {
    Block::Buttons(ButtonRow {
        buttons,
        compact: true,
        indent: lead + item::LEAD_GAP,
        menu: None,
        end: false,
    })
}

fn permission_blocks(request: &Request, at: &str, view: &View, p: &PanelCx<'_>) -> Vec<Block> {
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
    let mut row = vec![button(
        format!("{at}-deny"),
        "Deny",
        false,
        answer(Answer::Deny),
    )];
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
        min_width: Some(MIN_W - lead(view) - item::LEAD_GAP),
        ..Text::new(Tone::Caption(OVERLAY2), reason.clone())
    }));
    vec![
        Block::Item(item_for(request, at, 0, view), under),
        buttons(row, lead(view)),
    ]
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
        min_width: Some(MIN_W - lead(view) - item::LEAD_GAP),
        ..Text::new(
            Tone::Caption(SUBTEXT1),
            format!(
                "{} · {} of {}{}",
                question.header,
                step + 1,
                questions.len(),
                if multi { " · pick one or more" } else { "" }
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
                shown_height: None,
            },
            vec![Block::Item(
                Item {
                    id: format!("{at}-q{step}-option-{index}-item").into(),
                    lead: Some(Lead {
                        mark: Some(Mark::Choice {
                            key: format!("{at}-q{step}-option-{index}-choice").into(),
                            picked,
                            many: multi,
                        }),
                        tint: None,
                        glow: None,
                        faded: false,
                        size: Some(CHOICE),
                        on_click: None,
                        top: false,
                    }),
                    overline: None,
                    title: label.clone().into(),
                    title_color: None,
                    wrap: true,
                    detail: "".into(),
                    tinted: Vec::new(),
                    trail: Vec::new(),
                },
                (!description.is_empty())
                    .then(|| Block::Text(Text::new(Tone::Caption(OVERLAY2), description.clone())))
                    .into_iter()
                    .collect(),
            )],
        ));
    }
    let labels: Vec<Vec<String>> = questions
        .iter()
        .map(|question| {
            question
                .options
                .iter()
                .map(|(label, _)| label.clone())
                .collect()
        })
        .collect();
    // Without a pick, Next skips the question; Send waits until some question has one.
    let ready = !chosen.is_empty();
    let answered = picks.chosen.iter().any(|chosen| !chosen.is_empty());
    // A question that takes several answers counts them on the button.
    let label = |text: &str| {
        if multi && ready {
            format!("{text} ({})", chosen.len())
        } else {
            text.to_owned()
        }
    };
    let next = if last {
        let send = p.on_local::<Ai>(move |ai, view| {
            let picks = view.picks(id);
            if picks.chosen.iter().all(Vec::is_empty) {
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
        button(format!("{at}-send"), &label("Send answers"), answered, send)
    } else {
        button(
            format!("{at}-next"),
            &if ready {
                label("Next")
            } else {
                "Skip".to_owned()
            },
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
        Block::Item(item_for(request, at, step, view), under),
        buttons(vec![terminal, next], lead(view)),
    ]
}

/// Led by Jelly when it is on, otherwise Claude's mark.
fn item_for(request: &Request, at: &str, step: usize, view: &View) -> Item {
    let now = format::now();
    let pose = view.pose();
    // Jelly stands at the top of the card, beside its first lines.
    let top = pose.is_some();
    let tint = pose.is_none().then_some(PEACH);
    let mark = pose.map_or_else(
        || Mark::Glyph(harness::glyph(Source::Claude).to_path_buf(), PEACH),
        Mark::Mascot,
    );
    Item {
        id: format!("{at}-item").into(),
        lead: Some(Lead {
            mark: Some(mark),
            tint,
            glow: None,
            faded: false,
            size: Some(lead(view)),
            on_click: None,
            top,
        }),
        overline: Some(format!("Claude Code · {}", request.project).into()),
        title: title(request, step).into(),
        title_color: None,
        wrap: true,
        detail: "".into(),
        tinted: Vec::new(),
        trail: vec![Trail::Faint(format::ago(request.received_at, now).into())],
    }
}

/// The time `request` has left as of the last hold or release, paused while held.
fn countdown(request: &Request, hold: Hold) -> Option<Countdown> {
    if request.lifetime <= 0 {
        return None;
    }
    let from = hold
        .since
        .unwrap_or(hold.released_at)
        .max(request.received_at);
    let left_ms = (request.expires_at - from).max(0);
    Some(Countdown {
        left: (left_ms as f32 / request.lifetime as f32).min(1.0),
        running: hold
            .since
            .is_none()
            .then(|| Duration::from_millis(u64::try_from(left_ms).unwrap_or(0))),
        round: hold.round,
        color: ACCENT,
    })
}

/// One request as a card of its own, with what it asks and the ways to answer. `countdown` shows
/// the time it has left, as a pop-up does; `leaving` slides out one just answered.
pub(super) fn frame(
    request: &Request,
    view: &View,
    p: &PanelCx<'_>,
    countdown: Option<Hold>,
    leaving: bool,
) -> Block {
    let at = format!("ai-request-{}", request.id);
    let blocks = match request.ask {
        Ask::Permission { .. } => permission_blocks(request, &at, view, p),
        Ask::Questions { .. } => question_blocks(request, &at, view, p),
    };
    Block::Frame(
        FrameView {
            id: at.clone().into(),
            look: Look::Raised { urgent: false },
            on_click: None,
            on_dismiss: None,
            countdown: countdown.and_then(|hold| self::countdown(request, hold)),
            selected: false,
            motion: if leaving {
                Motion::Leave
            } else {
                Motion::Appear
            },
            shown_height: None,
        },
        blocks,
    )
}

#[cfg(test)]
#[path = "asking_test.rs"]
mod tests;
