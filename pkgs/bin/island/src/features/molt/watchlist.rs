//! Molt's Watchlist, as molt-core words it: tracked pull requests on the left, the picked one's way
//! through the branches on the right, a field in the header to track another.

use molt_core::watchlist::{
    flow::Msg,
    present::{self, Step, Tone},
};

use super::{
    inputs::{body_h, row},
    parts::{frame, views},
    runtime::Molt,
    state::Updates,
    view::{Focus, Screen, View},
};
use crate::{
    components::card::{Look, Motion, RAISED_GAP, RAISED_PAD},
    features::PanelCx,
    launch,
    panel::{
        Block, Chip, FrameView, HeaderLead, IconButton, InputView, Item, Lead, Mark, RowIcon,
        RowLead, SearchRow, SearchView, Split, Tint, Trail, Views, on,
    },
    theme::{CRITICAL, GREEN, MAUVE, OVERLAY2, PEACH, SUBTEXT1, TEAL, YELLOW},
    ui::{HEADING_SIZE, TITLE_SIZE},
};

const LEAD: f32 = 36.0;
const LIST_W: f32 = 330.0;
const FIELD_H: f32 = 34.0;
const FIELD_GAP: f32 = 10.0;
const DOT: f32 = 12.0;
/// The detail's parts over its branches: the chips and buttons, the title, the status line, the
/// Branches line.
const DETAIL_HEAD: f32 = 32.0 + 26.0 + 26.0 + 24.0;

fn color(tone: Tone) -> u32 {
    match tone {
        Tone::Plain => OVERLAY2,
        Tone::Green => GREEN,
        Tone::Yellow => YELLOW,
        Tone::Mauve => MAUVE,
        Tone::Teal => TEAL,
        Tone::Peach => PEACH,
    }
}

fn opens(url: String, id: String) -> FrameView {
    FrameView {
        id: id.into(),
        look: Look::Plain,
        on_click: Some(on(move |_| launch::open(&url, "GitHub"))),
        on_dismiss: None,
        countdown: None,
        selected: false,
        motion: Motion::Still,
        shown_height: None,
        min_height: None,
    }
}

fn step(step: &Step, index: usize) -> Block {
    let (pill, tone) = &step.pill;
    let item = Item {
        id: format!("molt-watch-step-{index}").into(),
        lead: Some(Lead {
            mark: Some(Mark::Choice {
                picked: step.reached,
                many: false,
            }),
            size: Some(DOT),
            top: true,
            ..Default::default()
        }),
        title: step.name.clone().into(),
        trail: vec![Trail::Chip(Chip::new(pill.clone(), color(*tone)))],
        ..Default::default()
    };
    let block = Block::Item(
        item,
        vec![Block::caption(step.description.clone(), SUBTEXT1)],
    );
    match &step.url {
        Some(url) => Block::Frame(
            opens(url.clone(), format!("molt-watch-step-{index}")),
            vec![block],
        ),
        None => block,
    }
}

fn text_button(
    label: &str,
    primary: bool,
    enabled: bool,
    busy: bool,
    on_click: crate::panel::Handler,
) -> IconButton {
    IconButton {
        glyph: None,
        color: primary.then_some(crate::theme::ACCENT),
        label: label.into(),
        active: primary,
        badge: None,
        enabled,
        busy,
        on_click,
    }
}

pub(super) fn watchlist(state: &Updates, view: &View, p: &PanelCx<'_>) -> (Views, Vec<Block>) {
    let back = p.on_local::<Molt>(|_, view: &mut View| view.open(Screen::Home));
    let Some(screen) = &state.watchlist else {
        return (
            views(None, Some(back), p),
            vec![Block::note("Reading the watchlist…")],
        );
    };
    let shown = present::view(screen);
    let mut header = views(
        Some(HeaderLead {
            mark: p
                .glyph("molt-watchlist")
                .map_or(Mark::Letter("W".into(), TEAL), |path| {
                    Mark::Glyph(path, TEAL)
                }),
            size: LEAD,
            overline: shown.subtitle.clone().into(),
            title: shown.title.into(),
            title_color: None,
            on_click: None,
            on_point: None,
        }),
        Some(back),
        p,
    );
    header.field = Some(InputView {
        text: shown.track.clone().into(),
        placeholder: shown.track_placeholder.into(),
        max_lines: 1,
        send_glyph: p.glyph("molt-track"),
        on_send: shown
            .track_enabled
            .then(|| p.call(|molt: &Molt| molt.watchlist(Msg::Track))),
        busy: screen.tracking,
        secret: false,
        focused: view.focus == Focus::Track,
        on_focus: Some(p.on_local::<Molt>(|_, view: &mut View| view.focus = Focus::Track)),
    });

    let body = body_h();
    let list_h = body - 2.0 * RAISED_PAD - FIELD_H - FIELD_GAP;
    let rows = shown
        .rows
        .iter()
        .map(|entry| {
            let id = entry.id;
            let (badge, tone) = entry.badge;
            SearchRow {
                name: entry.title.clone().into(),
                detail: entry.sub.clone().into(),
                icon: RowIcon::None,
                mono: false,
                section: None,
                mark: None,
                trail: vec![Trail::Chip(Chip::new(badge, color(tone)))],
                selected: entry.selected,
                on_click: p.call(move |molt: &Molt| molt.watchlist(Msg::Select(id))),
                on_hover: on(|_| {}),
            }
        })
        .collect();
    let list = Block::Frame(
        frame("molt-watch-list".to_owned(), Some(body)),
        vec![Block::Search(SearchView {
            glyph: p.glyph("molt-search"),
            query: shown.query.clone().into(),
            placeholder: shown.filter_placeholder.into(),
            empty: shown.empty.into(),
            filters: None,
            rows,
            pictures: false,
            on_scroll: std::rc::Rc::new(|_, _| {}),
            preview: None,
            fill: true,
            compact: true,
            list_height: Some(list_h),
            status: None,
            list_width: None,
            beside: Vec::new(),
            focused: view.focus == Focus::Filter,
            on_focus: Some(p.on_local::<Molt>(|_, view: &mut View| view.focus = Focus::Filter)),
            table: None,
        })],
    );

    let detail = match &shown.detail {
        None => vec![Block::note(shown.nothing)],
        Some(detail) => {
            let url = detail.url.clone();
            let chips = Item {
                id: "molt-watch-chips".into(),
                tinted: detail
                    .chips
                    .iter()
                    .map(|(words, tone)| Tint::Chip(words.clone().into(), color(*tone)))
                    .collect(),
                ..Default::default()
            };
            let mut blocks = vec![
                row(
                    vec![
                        text_button(
                            "Open on GitHub",
                            false,
                            true,
                            false,
                            on(move |_| launch::open(&url, "GitHub")),
                        ),
                        text_button(
                            "Untrack",
                            false,
                            true,
                            false,
                            p.call(|molt: &Molt| molt.watchlist(Msg::Untrack)),
                        ),
                    ],
                    Some(RowLead::Item(Box::new(chips), Vec::new())),
                ),
                Block::Item(
                    Item {
                        id: "molt-watch-title".into(),
                        title: detail.title.clone().into(),
                        title_size: Some(HEADING_SIZE),
                        ..Default::default()
                    },
                    Vec::new(),
                ),
            ];
            let status = Item {
                id: "molt-watch-status".into(),
                label: Some("Status".into()),
                title: detail
                    .commit
                    .as_ref()
                    .map(|(commit, _)| commit.clone())
                    .unwrap_or_default()
                    .into(),
                title_mono: true,
                title_color: Some(MAUVE),
                detail: detail.meta.clone().into(),
                ..Default::default()
            };
            blocks.push(match &detail.commit {
                Some((_, address)) => Block::Frame(
                    opens(address.clone(), "molt-watch-commit".to_owned()),
                    vec![Block::Item(status, Vec::new())],
                ),
                None => Block::Item(status, Vec::new()),
            });
            blocks.push(Block::Item(
                Item {
                    id: "molt-watch-branches".into(),
                    title: "Branches".into(),
                    title_size: Some(TITLE_SIZE),
                    trail: vec![Trail::Faint(detail.summary.clone().into())],
                    ..Default::default()
                },
                Vec::new(),
            ));
            let room = body - 2.0 * RAISED_PAD - DETAIL_HEAD - 4.0 * RAISED_GAP;
            blocks.push(Block::Scrolled(
                "molt-watch-steps".into(),
                room.max(0.0),
                detail
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(index, each)| step(each, index))
                    .collect(),
            ));
            blocks
        }
    };
    let detail = Block::Frame(frame("molt-watch-detail".to_owned(), Some(body)), detail);

    let lead = shown.status.as_ref().map(|(words, error)| {
        if *error {
            RowLead::Chip(Chip::new(words.clone(), CRITICAL))
        } else {
            RowLead::Note(words.clone().into())
        }
    });
    (
        header,
        vec![
            Block::Columns(
                Split::Lead {
                    width: LIST_W,
                    height: body,
                },
                vec![list, detail],
            ),
            row(
                vec![text_button(
                    "Refresh all",
                    false,
                    !shown.refreshing,
                    shown.refreshing,
                    p.call(|molt: &Molt| molt.watchlist(Msg::Refresh)),
                )],
                lead,
            ),
        ],
    )
}
