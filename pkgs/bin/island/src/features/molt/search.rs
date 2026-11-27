//! Molt's Search, as molt-core words it: the catalog's packages and options for what is typed, the
//! picked one beside the list, and checking it against the user's own flake.

use molt_core::search::{
    flow::Msg,
    model::SearchScope,
    present::{self, Detail, Kind, LocalView},
};

use super::{
    inputs::{body_h, row},
    parts::{frame, views},
    runtime::Molt,
    state::Updates,
    view::{Focus, Screen, View},
};
use crate::{
    components::{
        card::RAISED_PAD,
        text::{Style as TextStyle, Text, Tone as TextTone},
    },
    copy,
    features::PanelCx,
    launch,
    panel::{
        Block, Chip, Filters, HeaderLead, IconButton, Item, Mark, RowIcon, RowLead, SearchRow,
        SearchView, TabLabel, Views, on,
    },
    theme::{ACCENT, BLUE, CRITICAL, GREEN, OVERLAY2, PEACH, SAPPHIRE, SUBTEXT1},
    ui::HEADING_SIZE,
};

const LEAD: f32 = 36.0;
const LIST_W: f32 = 470.0;
/// The field, the categories and the space under them, over the list.
const OVER_LIST: f32 = 34.0 + 10.0 + 40.0 + 10.0;
/// The note over the field, and the space under it.
const NOTE_H: f32 = 16.0 + 12.0;

fn kind(kind: Kind) -> (&'static str, u32) {
    match kind {
        Kind::Package => ("molt-package", GREEN),
        Kind::NixOsOption => ("molt-option", BLUE),
        Kind::HomeManagerOption => ("molt-home-manager", PEACH),
    }
}

fn plain_button(
    label: &str,
    primary: bool,
    enabled: bool,
    on_click: crate::panel::Handler,
) -> IconButton {
    IconButton {
        glyph: None,
        color: primary.then_some(ACCENT),
        label: label.into(),
        active: primary,
        badge: None,
        enabled,
        busy: false,
        on_click,
    }
}

fn caption(words: &'static str) -> Block {
    Block::Text(Text::new(TextTone::Heading, words))
}

fn body(words: String, code: bool, muted: bool) -> Block {
    if muted {
        return Block::caption(words, OVERLAY2);
    }
    let mut text = Text::new(TextTone::Body, words.clone());
    if code {
        text.spans = vec![(
            0..words.len(),
            TextStyle {
                code: true,
                ..TextStyle::default()
            },
        )];
    }
    Block::Text(text)
}

fn detail(detail: &Detail, p: &PanelCx<'_>) -> Vec<Block> {
    let (_, color) = kind(detail.kind);
    let (copy_label, copy_text) = detail.copy_name.clone();
    let snippet = detail.snippet.clone();
    let mut blocks = vec![
        row(
            vec![
                plain_button(
                    copy_label,
                    false,
                    true,
                    on(move |_| copy::text(copy_text.clone())),
                ),
                plain_button(
                    "Copy snippet",
                    false,
                    true,
                    on(move |_| copy::text(snippet.clone())),
                ),
            ],
            Some(RowLead::Chip(Chip::new(detail.category, color))),
        ),
        Block::Item(
            Item {
                id: "molt-search-name".into(),
                title: detail.name.clone().into(),
                title_mono: true,
                title_size: Some(HEADING_SIZE),
                wrap: true,
                ..Default::default()
            },
            Vec::new(),
        ),
    ];
    for section in &detail.sections {
        blocks.push(caption(section.caption));
        blocks.push(body(section.text.clone(), section.code, section.muted));
    }
    if !detail.links.is_empty() {
        blocks.push(row(
            detail
                .links
                .iter()
                .map(|(label, url)| {
                    let url = url.clone();
                    plain_button(
                        label,
                        false,
                        true,
                        on(move |_| launch::open(&url, "Nixpkgs")),
                    )
                })
                .collect(),
            None,
        ));
    }
    if let Some(local) = &detail.local {
        blocks.push(caption("In your flake"));
        let check = |label: &'static str| {
            row(
                vec![plain_button(
                    label,
                    false,
                    true,
                    p.call(|molt: &Molt| molt.search(Msg::CheckLocally)),
                )],
                None,
            )
        };
        match local {
            LocalView::Idle { note, action } => {
                blocks.push(Block::caption(*note, SUBTEXT1));
                blocks.push(check(action));
            }
            LocalView::Running(note) => blocks.push(Block::caption(*note, SUBTEXT1)),
            LocalView::Done {
                summary,
                value,
                attribute,
                action,
            } => {
                blocks.push(Block::caption(*summary, SUBTEXT1));
                blocks.push(body(value.clone(), true, false));
                blocks.push(Block::caption(attribute.clone(), OVERLAY2));
                blocks.push(check(action));
            }
            LocalView::Failed { error, action } => {
                blocks.push(Block::caption(error.clone(), CRITICAL));
                blocks.push(check(action));
            }
        }
    }
    blocks
}

pub(super) fn search(state: &Updates, view: &View, p: &PanelCx<'_>) -> (Views, Vec<Block>) {
    let back = p.on_local::<Molt>(|_, view: &mut View| view.open(Screen::Home));
    let Some(screen) = &state.search else {
        return (
            views(None, Some(back), p),
            vec![Block::note("Opening Search…")],
        );
    };
    let shown = present::view(screen);
    let header = views(
        Some(HeaderLead {
            mark: p
                .glyph("molt-search")
                .map_or(Mark::Letter("S".into(), SAPPHIRE), |path| {
                    Mark::Glyph(path, SAPPHIRE)
                }),
            size: LEAD,
            overline: shown.subtitle.into(),
            title: shown.title.into(),
            title_color: None,
            on_click: None,
            on_point: None,
        }),
        Some(back),
        p,
    );
    let height = body_h();
    let list_h = height - NOTE_H - OVER_LIST;
    let rows = shown
        .rows
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let (glyph, color) = kind(entry.kind);
            SearchRow {
                name: entry.name.clone().into(),
                detail: entry.detail.clone().into(),
                icon: p
                    .glyph(glyph)
                    .map_or(RowIcon::None, |path| RowIcon::Tinted(path, color)),
                mono: true,
                section: None,
                mark: None,
                trail: Vec::new(),
                selected: entry.selected,
                on_click: p.call(move |molt: &Molt| molt.search(Msg::Select(index))),
                on_hover: on(|_| {}),
            }
        })
        .collect();
    let beside = vec![Block::Frame(
        frame("molt-search-detail".to_owned(), None),
        vec![Block::Scrolled(
            "molt-search-detail-body".into(),
            list_h - 2.0 * RAISED_PAD,
            match &shown.detail {
                Some(picked) => detail(picked, p),
                None => vec![Block::note(shown.nothing)],
            },
        )],
    )];
    let mut status = shown.status.clone();
    if let Some(notice) = &shown.notice {
        status = format!("{status} · {notice}");
    }
    let search = Block::Search(SearchView {
        glyph: p.glyph("molt-search"),
        query: shown.query.clone().into(),
        placeholder: shown.placeholder.into(),
        empty: shown.status.clone().into(),
        filters: Some(Filters {
            tabs: shown
                .scopes
                .iter()
                .map(|label| TabLabel {
                    label: label.clone().into(),
                    glyph: None,
                    count: None,
                })
                .collect(),
            selected: shown.scope,
            on_select: p.on_local_with::<Molt, usize>(|molt, _, index| {
                molt.search(Msg::Scope(
                    SearchScope::ALL[index.min(SearchScope::ALL.len() - 1)],
                ));
            }),
        }),
        rows,
        pictures: false,
        on_scroll: std::rc::Rc::new(|_, _| {}),
        preview: None,
        fill: false,
        compact: true,
        list_height: Some(list_h),
        status: Some(status.into()),
        list_width: Some(LIST_W),
        beside,
        focused: view.focus == Focus::Filter,
        on_focus: Some(p.on_local::<Molt>(|_, view: &mut View| view.focus = Focus::Filter)),
        table: None,
    });
    let mut footer = vec![
        plain_button(
            "Previous",
            false,
            shown.previous,
            p.call(|molt: &Molt| molt.search(Msg::Previous)),
        ),
        plain_button(
            "Next",
            false,
            shown.next,
            p.call(|molt: &Molt| molt.search(Msg::Next)),
        ),
    ];
    if shown.failed {
        footer.insert(
            0,
            plain_button(
                "Retry",
                false,
                true,
                p.call(|molt: &Molt| molt.search(Msg::Retry)),
            ),
        );
    }
    (
        header,
        vec![
            Block::caption(shown.note, SUBTEXT1),
            search,
            row(footer, None),
        ],
    )
}
