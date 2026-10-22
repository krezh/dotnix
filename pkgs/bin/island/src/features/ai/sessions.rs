use super::{
    Ai, asking, chat, format, harness, hook,
    session::{Activity, LineKind, Live, LiveLine, Session, Step, StepState},
    state::{Source, State},
    view::View,
};
use crate::{
    components::{
        card,
        card::{Look, Motion},
        item,
    },
    features::PanelCx,
    panel::{
        Block, Change, Code, CodeLine, CodeMotion, FrameView, Item, Lead, Mark, Piece, Text, Tone,
        Trail, Transcript, Turn,
    },
    theme::{ACCENT, GREEN, PEACH, YELLOW},
};

/// As a notification's icon, so the session cards read as the same kind of card.
const ROW_LEAD: f32 = 36.0;

/// Each state in its own color: waiting on you, working, or done and ready for your next prompt.
fn status(activity: Activity) -> (&'static str, u32) {
    match activity {
        Activity::Waiting => ("Waiting", YELLOW),
        Activity::Working => ("Working", ACCENT),
        Activity::Idle => ("Ready", GREEN),
    }
}

/// What the session card says under the name: its reply's first line once done, else what it
/// is doing.
fn summary(session: &Session) -> String {
    if session.activity == Activity::Idle && !session.reply().is_empty() {
        let plain = crate::markdown::parse(session.reply()).text;
        if let Some(first) = plain.split(['.', '\n']).find(|part| !part.trim().is_empty()) {
            return first.trim().to_owned();
        }
    }
    session.doing.clone()
}

/// The working directory with the home folder as ~.
fn folder(cwd: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && cwd.starts_with(&home) => format!("~{}", &cwd[home.len()..]),
        _ => cwd.to_owned(),
    }
}

/// "claude-haiku-5-5" as "Haiku 5.5"; anything else as it is.
pub(super) fn model_name(model: &str) -> String {
    let model = model.split('[').next().unwrap_or(model);
    let Some(rest) = model.strip_prefix("claude-") else {
        return model.to_owned();
    };
    let mut parts = rest.split('-');
    let Some(family) = parts.next() else {
        return model.to_owned();
    };
    let mut name: String = family
        .chars()
        .enumerate()
        .map(|(at, char)| if at == 0 { char.to_ascii_uppercase() } else { char })
        .collect();
    let version: Vec<&str> = parts.take_while(|part| part.len() <= 2).collect();
    if !version.is_empty() {
        name.push(' ');
        name.push_str(&version.join("."));
    }
    name
}

/// Waiting ones first, then working, then idle, the most recently active first within each.
pub(super) fn ordered(state: &State) -> Vec<&Session> {
    let mut sessions: Vec<&Session> = state.sessions.iter().collect();
    sessions.sort_by(|left, right| {
        left.activity
            .cmp(&right.activity)
            .then(right.active_at.cmp(&left.active_at))
    });
    sessions
}

fn mark(color: u32, size: f32) -> Lead {
    Lead {
        mark: Some(Mark::Glyph(
            harness::glyph(Source::Claude).to_path_buf(),
            color,
        )),
        tint: Some(color),
        glow: None,
        faded: false,
        size: Some(size),
        on_click: None,
        top: false,
    }
}

/// One card per session down the left, as notifications stack, the shown one tinted; no card
/// around them, so nothing is boxed twice or stretched to the height beside it.
fn list(sessions: &[&Session], shown: &str, p: &PanelCx<'_>) -> Block {
    let now = format::now();
    let text_w = card::Size::Small.width() - 2.0 * card::RAISED_PAD - ROW_LEAD - item::LEAD_GAP;
    let rows = sessions
        .iter()
        .map(|session| {
            let id = session.id.clone();
            let (label, color) = status(session.activity);
            Block::Frame(
                FrameView {
                    id: format!("ai-session-{}", session.id).into(),
                    look: Look::Raised { urgent: false },
                    on_click: Some(
                        p.on_local::<Ai>(move |_, view| view.session = Some(id.clone())),
                    ),
                    on_dismiss: None,
                    countdown: None,
                    selected: session.id == shown,
                    motion: Motion::Appear,
                    shown_height: None,
                },
                vec![Block::Item(
                    Item {
                        id: format!("ai-session-{}-item", session.id).into(),
                        lead: Some(mark(color, ROW_LEAD)),
                        overline: Some(format::countdown(now, session.active_at).into()),
                        title: session.project.clone().into(),
                        title_color: None,
                        detail: "".into(),
                        trail: vec![Trail::Status(label.into(), color)],
                    },
                    vec![Block::Text(Text {
                        lines: Some(1),
                        min_width: Some(text_w),
                        max_width: Some(text_w),
                        ..Text::new(Tone::Body, summary(session))
                    })],
                )],
            )
        })
        .collect();
    Block::Grid(1, rows)
}

fn step_color(state: StepState) -> u32 {
    match state {
        StepState::Done => GREEN,
        StepState::Now => ACCENT,
        StepState::Waiting => YELLOW,
    }
}

fn live_line(line: &LiveLine) -> CodeLine {
    CodeLine {
        number: line.number,
        text: line.text.clone().into(),
        change: match line.kind {
            LineKind::Same => Change::Same,
            LineKind::Added => Change::Added,
            LineKind::Removed => Change::Removed,
        },
    }
}

/// What a step read, changed or ran, as Claude Code reported it. `fresh`, when it arrived just
/// now, it rises in; what was already there is shown still.
fn live_code(session: &str, step: &Step, live: &Live, fresh: bool) -> Code {
    let (heading, lines) = match live {
        Live::Reading { file, lines } => {
            let range = match (lines.first().and_then(|l| l.number), lines.last().and_then(|l| l.number)) {
                (Some(first), Some(last)) => format!("lines {first}–{last}"),
                _ => "reading…".to_owned(),
            };
            ((file.clone(), range), lines.iter().map(live_line).collect())
        }
        Live::Editing { file, lines } => {
            let added = lines.iter().filter(|l| l.kind == LineKind::Added).count();
            let removed = lines.iter().filter(|l| l.kind == LineKind::Removed).count();
            (
                (file.clone(), format!("+{added} −{removed}")),
                lines.iter().map(live_line).collect(),
            )
        }
        // The whole command, wrapped, then what it printed.
        Live::Running {
            command,
            output,
            done,
        } => (
            (
                "Command".to_owned(),
                if *done { "done" } else { "running…" }.to_owned(),
            ),
            command
                .lines()
                .enumerate()
                .map(|(at, line)| {
                    CodeLine::plain(if at == 0 {
                        format!("$ {line}")
                    } else {
                        format!("  {line}")
                    })
                })
                .chain(output.iter().map(|line| CodeLine::plain(line.clone())))
                .collect(),
        ),
    };
    let wrap = matches!(live, Live::Running { .. });
    Code {
        key: format!("ai-step-{session}-{}", step.seq).into(),
        heading: Some((heading.0.into(), heading.1.into())),
        lines,
        wrap,
        copy: None,
        motion: if fresh {
            CodeMotion::Appear
        } else {
            CodeMotion::Still
        },
    }
}

/// One step as a line with its state's dot; picked, it opens on what it read, changed or ran.
/// The step being worked on is open on its own.
fn step_block(session: &Session, step: &Step, following: bool, view: &View, p: &PanelCx<'_>) -> Block {
    let open = view.step_open(step.seq, following);
    let seq = step.seq;
    let fresh = view.fresh(&session.id, seq);
    let mut under = Vec::new();
    if open && let Some(live) = &step.live {
        under.push(Block::Code(live_code(&session.id, step, live, fresh)));
    }
    Block::Frame(
        FrameView {
            id: format!("ai-step-{}-{seq}", session.id).into(),
            look: Look::Inset,
            on_click: step
                .live
                .is_some()
                .then(|| p.on_local::<Ai>(move |_, view| view.toggle_step(seq))),
            on_dismiss: None,
            countdown: None,
            selected: false,
            motion: if fresh { Motion::Appear } else { Motion::Still },
            shown_height: None,
        },
        vec![Block::Item(
            Item {
                id: format!("ai-step-{}-{seq}-item", session.id).into(),
                lead: None,
                overline: None,
                title: hook::label(&step.verb, &step.target).into(),
                title_color: None,
                detail: "".into(),
                trail: vec![
                    Trail::Status("".into(), step_color(step.state)),
                    Trail::Faint(match (step.live.is_some(), open) {
                        (false, _) => "".into(),
                        (true, true) => "▾".into(),
                        (true, false) => "▸".into(),
                    }),
                ],
            },
            under,
        )],
    )
}

/// `appear` for turns that arrived while the panel showed; history is shown still.
fn conversation(session: &Session, part: String, turns: Vec<Turn>, appear: bool) -> Option<Block> {
    (!turns.is_empty()).then(|| {
        Block::Transcript(Transcript {
            id: format!("ai-session-{}-{part}", session.id).into(),
            turns,
            glyph: Some(harness::glyph(Source::Claude).to_path_buf()),
            color: PEACH,
            appear_from: if appear { 0 } else { usize::MAX },
        })
    })
}

/// Each prompt, the steps it led to, and the reply that ended it, oldest first; while it works,
/// the last reply waits with dots, after whatever it asks of you.
fn timeline(session: &Session, state: &State, view: &View, p: &PanelCx<'_>) -> Vec<Block> {
    let model = if session.model.is_empty() {
        "Claude".to_owned()
    } else {
        model_name(&session.model)
    };
    let working = session.activity == Activity::Working;
    let last = session.history.len().saturating_sub(1);
    let mut blocks = Vec::new();
    for (index, exchange) in session.history.iter().enumerate() {
        let latest = index == last;
        if !exchange.prompt.is_empty() {
            blocks.extend(conversation(
                session,
                format!("{index}-prompt"),
                vec![Turn::Mine(Text::new(Tone::Reading, exchange.prompt.clone()))],
                view.fresh(&session.id, exchange.seq),
            ));
        }
        let newest = exchange.steps.len().saturating_sub(1);
        for (at, step) in exchange.steps.iter().enumerate() {
            let following = latest && working && at == newest;
            blocks.push(step_block(session, step, following, view, p));
        }
        if latest {
            for request in state
                .requests
                .iter()
                .filter(|request| request.session == session.id)
            {
                blocks.push(asking::frame(request, view, p, None, false));
            }
        }
        let turn = if latest && working {
            Some(Turn::Waiting {
                overline: format!("{model} · working").into(),
            })
        } else if !exchange.reply.is_empty() {
            Some(Turn::Theirs {
                overline: model.clone().into(),
                pieces: chat::pieces(&exchange.reply),
                streaming: false,
                actions: Vec::new(),
            })
        } else if latest && !session.doing.is_empty() && session.activity == Activity::Idle {
            Some(Turn::Theirs {
                overline: model.clone().into(),
                pieces: vec![Piece::Text(Text::new(Tone::Reading, session.doing.clone()))],
                streaming: false,
                actions: Vec::new(),
            })
        } else {
            None
        };
        let new_reply = view.fresh(&session.id, exchange.reply_seq.max(exchange.seq));
        blocks.extend(conversation(
            session,
            format!("{index}-reply"),
            turn.into_iter().collect(),
            new_reply,
        ));
    }
    if session.history.is_empty() {
        blocks.extend(conversation(
            session,
            "start".to_owned(),
            vec![Turn::Theirs {
                overline: model.into(),
                pieces: vec![Piece::Text(Text::new(Tone::Reading, session.doing.clone()))],
                streaming: false,
                actions: Vec::new(),
            }],
            false,
        ));
    }
    blocks
}

fn detail(session: &Session, state: &State, view: &View, p: &PanelCx<'_>) -> Block {
    let now = format::now();
    let (label, color) = status(session.activity);
    let model = if session.model.is_empty() {
        "Not reported".to_owned()
    } else {
        model_name(&session.model)
    };
    let mode = if session.mode.is_empty() {
        "default".to_owned()
    } else {
        session.mode.clone()
    };
    // Three chips fill one row, as on a tile; the mode goes with the folder.
    let mut blocks = vec![
        Block::Hero(
            session.project.clone().into(),
            vec![format!("{} · {mode} mode", folder(&session.cwd)).into()],
        ),
        // The state in its color, the model in Claude's; the time stays plain.
        Block::Facts(vec![
            ("Status".into(), label.into(), Some(color)),
            (
                "Model".into(),
                model.into(),
                (!session.model.is_empty()).then_some(PEACH),
            ),
            (
                "Running".into(),
                format::countdown(now, session.started_at).into(),
                None,
            ),
        ]),
    ];
    blocks.extend(timeline(session, state, view, p));
    Block::Card(None, card::Size::Reading, blocks)
}

pub(super) fn blocks(state: &State, view: &View, p: &PanelCx<'_>) -> Vec<Block> {
    let sessions = ordered(state);
    let Some(first) = sessions.first() else {
        return vec![Block::Text(Text {
            min_width: Some(card::Size::Reading.width()),
            ..Text::new(
                Tone::Note,
                "No Claude Code sessions. One shows here from when it starts until it ends.",
            )
        })];
    };
    let shown = view
        .session
        .as_deref()
        .and_then(|id| sessions.iter().find(|session| session.id == id))
        .unwrap_or(first);
    vec![Block::Columns(vec![
        list(&sessions, &shown.id, p),
        detail(shown, state, view, p),
    ])]
}

#[cfg(test)]
#[path = "sessions_test.rs"]
mod tests;
