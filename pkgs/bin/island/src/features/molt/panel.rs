use super::{
    cache::now,
    runtime::Molt,
    state::{Updates, WatchedPr},
};
use crate::{
    components::card,
    features::PanelCx,
    launch::{self, Launch},
    panel::{Block, Button, Header, Line, PanelView, on},
    units::{self, plural},
};

const CHECK: &str = "Refresh";
const CHECKING: &str = "Refreshing…";

fn ago(at: u64) -> String {
    if at == 0 {
        return "unknown time".to_owned();
    }
    units::ago(i64::try_from(now().saturating_sub(at)).unwrap_or(i64::MAX))
}

fn open() {
    launch::spawn(Launch {
        id: "io.github.krezh.Molt".to_owned(),
        name: "Molt".to_owned(),
        argv: vec!["molt".to_owned()],
        directory: None,
    });
}

pub(super) fn panel(state: &Updates, p: &PanelCx<'_>) -> PanelView {
    let buttons = vec![
        Button {
            id: "molt-check".into(),
            label: if state.checking { CHECKING } else { CHECK }.into(),
            glyph: None,
            bare: false,
            active: state.checking,
            toggle: false,
            fits: vec![CHECK.into(), CHECKING.into()],
            busy: state.refreshing,
            enabled: true,
            on_click: p.call(Molt::refresh),
        },
        Button {
            id: "molt-open".into(),
            label: "Open Molt".into(),
            glyph: None,
            bare: false,
            active: false,
            toggle: false,
            fits: Vec::new(),
            busy: false,
            enabled: true,
            on_click: on(|ui| {
                open();
                ui.collapse();
            }),
        },
    ];
    let card = |block: Block| Block::Card(None, card::Size::Large, vec![block]);
    let block = match (&state.error, &state.report) {
        (Some(error), _) => card(Block::note(format!("The last check failed: {error}"))),
        (None, None) if state.checking => card(Block::note("Checking for updates…")),
        (None, None) => card(Block::note("Not checked yet")),
        (None, Some(report)) if report.commits.is_empty() => {
            card(Block::note(if report.incoming == 0 {
                "Up to date"
            } else {
                "Check again to list the commits"
            }))
        }
        (None, Some(report)) => card(Block::Lines(
            report
                .commits
                .iter()
                .map(|commit| Line {
                    summary: commit.summary.clone().into(),
                    detail: format!(
                        "{} · {} · {}",
                        commit.author,
                        ago(u64::try_from(commit.time).unwrap_or(0)),
                        commit.id
                    )
                    .into(),
                })
                .collect(),
        )),
    };
    let title = match state.incoming() {
        0 => "Updates".to_owned(),
        incoming => plural(incoming, "incoming change", "incoming changes"),
    };
    let mut blocks = vec![block];
    if let Some(watchlist) = state
        .report
        .as_ref()
        .and_then(|report| report.watchlist.as_ref())
        .filter(|watchlist| !watchlist.prs.is_empty())
    {
        let title = match watchlist.ready {
            0 => "Watchlist".to_owned(),
            ready => format!("Watchlist · {ready} ready"),
        };
        blocks.push(Block::Card(
            Some(title.into()),
            card::Size::Large,
            vec![Block::Routes(
                watchlist.prs.iter().map(WatchedPr::row).collect(),
            )],
        ));
    }
    PanelView {
        scroll: false,
        header: Header::Title(title.into(), buttons),
        blocks,
    }
}
