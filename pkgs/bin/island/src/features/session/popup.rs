use std::time::Duration;

use super::{
    Session,
    local::Exit,
    state::{App, Leaving, Mood, Phase},
};
use crate::{
    features::PanelCx,
    panel::{
        Block, ButtonRow, Header, Hero, IconButton, Item, Lead, Mark, PanelView, Place, Placed,
        SharedString, Slider, Text, Tone, Trail,
    },
    theme::{CRITICAL, RED, YELLOW, app_color},
    ui::INSET,
    units::{plural, running},
};

/// The card's width, and the room each app's row takes.
const CARD_W: f32 = 500.0;
const ROW_H: f32 = 48.0;
const APP_LEAD: f32 = 30.0;
/// How far a closing row slides right as it fades.
const SLIDE: f32 = 14.0;
/// Rows shown before the rest scroll.
const MOST_ROWS: f32 = 7.0;

fn heading(leaving: &Leaving, p: &PanelCx<'_>) -> Vec<SharedString> {
    let elapsed = leaving.since.elapsed();
    p.redraw_in(Duration::from_secs(1) - Duration::from_nanos(elapsed.subsec_nanos().into()));
    let took = running(elapsed.as_secs());
    let line = match &leaving.phase {
        Phase::Finding => "Finding the open apps…".to_owned(),
        Phase::Failed(error) => error.clone(),
        Phase::Closing | Phase::Off => match leaving.open() {
            0 => format!("All apps closed · {took}"),
            open => format!("{} still open · {took}", plural(open, "app", "apps")),
        },
    };
    let line = if leaving.rehearsal {
        format!("{line} · Rehearsal")
    } else {
        line
    };
    vec![line.into()]
}

fn row(app: &App) -> Block {
    let mark = match &app.icon {
        Some(icon) => Mark::Image(icon.clone()),
        None => Mark::Letter(app.name.clone().into(), app_color(&app.name)),
    };
    let trail = match app.mood {
        Mood::Closing => Trail::Faint("Closing…".into()),
        Mood::Lingering => Trail::Text("Not responding".into(), YELLOW),
        Mood::Killed => Trail::Text("Killed".into(), CRITICAL),
    };
    let under = (!app.title.is_empty()).then(|| {
        let mut title = Text::new(Tone::Body, app.title.clone());
        title.lines = Some(1);
        Block::Text(title)
    });
    Block::Item(
        Item {
            id: format!("session-app-{}", app.key).into(),
            lead: Some(Lead {
                mark: Some(mark),
                tint: None,
                glow: None,
                faded: false,
                size: Some(APP_LEAD),
                on_click: None,
                top: false,
            }),
            overline: None,
            title: app.name.clone().into(),
            title_color: None,
            wrap: false,
            detail: SharedString::default(),
            tinted: Vec::new(),
            trail: vec![trail],
        },
        under.into_iter().collect(),
    )
}

/// Each app's row, those closing sliding out and fading as the rows below move up. The list keeps
/// the room the `total` found took, even with none left, so the card and its buttons stay put.
fn rows(exit: &Exit, total: usize) -> Block {
    let mut y = 0.0;
    let mut still = 0.0;
    let mut blocks = Vec::new();
    for slot in exit.rows.iter() {
        let shown = slot.presence.value().clamp(0.0, 1.0);
        let leaving = slot.presence.target() == 0.0;
        let height = ROW_H * shown;
        if !leaving {
            still += ROW_H;
        }
        if shown > 0.001 {
            let place = Place {
                inset: 0.0,
                offset: if leaving { (1.0 - shown) * SLIDE } else { 0.0 },
                y,
                height,
                opacity: shown,
                from_bottom: false,
                drawn: Some(ROW_H),
            };
            blocks.push((place, row(&slot.item)));
        }
        y += height;
    }
    let room = ROW_H * (total as f32).min(MOST_ROWS);
    Block::Placed(Placed {
        width: CARD_W - 2.0 * INSET,
        height: y.max(room),
        visible: Some(room),
        still: still.max(room),
        rewind: None,
        blocks,
    })
}

impl Session {
    pub(super) fn exit_view(&self, leaving: &Leaving, p: &PanelCx<'_>) -> Option<PanelView> {
        let exit = p.local::<Exit>()?;
        let action = leaving.action;
        let tint = action.color();
        let hero = Block::Hero(Hero {
            lead: Some(Lead {
                mark: p
                    .glyph(&format!("session-{}", action.word()))
                    .map(|path| Mark::Glyph(path, tint)),
                tint: Some(tint),
                glow: None,
                faded: false,
                size: Some(44.0),
                on_click: None,
                top: false,
            }),
            name: format!("{}…", action.doing()).into(),
            lines: heading(leaving, p),
            heading: true,
        });
        let closed = if leaving.total == 0 {
            1.0
        } else {
            (leaving.total - leaving.open()) as f32 / leaving.total as f32
        };
        let progress = Block::Slider(Slider {
            value: closed,
            color: tint,
            signal: None,
            on_change: None,
            label: None,
            thin: true,
        });
        let failed = matches!(leaving.phase, Phase::Failed(_));
        let cancel = IconButton {
            glyph: None,
            color: None,
            label: "Cancel".into(),
            active: false,
            badge: None,
            on_click: p.call::<Self>(Self::cancel),
        };
        let force = IconButton {
            glyph: None,
            color: Some(RED),
            label: "Force quit".into(),
            active: false,
            badge: None,
            on_click: p.call::<Self>(Self::force),
        };
        let buttons = Block::Buttons(ButtonRow {
            buttons: if failed {
                vec![cancel]
            } else {
                vec![cancel, force]
            },
            compact: false,
            indent: 0.0,
            menu: None,
            end: false,
            fill: true,
        });
        let blocks = vec![hero, progress, rows(exit, leaving.total), buttons];
        Some(PanelView {
            scroll: false,
            header: Header::None,
            blocks,
        })
    }
}
