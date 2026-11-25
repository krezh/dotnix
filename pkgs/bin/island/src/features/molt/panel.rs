//! Molt's panel: the screen this island shows, and while it changes, the one leaving, a tool
//! growing out of its card on Home or shrinking back into it, and what a tool opens sliding in
//! over it from the side.

use molt_core::build::flow::Stage;

use super::{
    build::build,
    cleanup::cleanup,
    home::home,
    inputs::inputs,
    notes::{diff, notes},
    review::review,
    search::search,
    state::Updates,
    view::{Screen, View},
    watchlist::watchlist,
};
use crate::{
    features::PanelCx,
    panel::{Block, Grow, Header, PanelView, Passing, Views},
};

fn screen(screen: Screen, state: &Updates, view: &View, p: &PanelCx<'_>) -> (Views, Vec<Block>) {
    let build = state.build.as_ref();
    let review = state.review.as_ref();
    match screen {
        Screen::Home => home(state, p),
        Screen::Inputs => inputs(state, view, p),
        Screen::Watchlist => watchlist(state, view, p),
        Screen::Search => search(state, view, p),
        Screen::Cleanup => cleanup(state, view, p),
        Screen::Build(target) => {
            let built = build.filter(|build| {
                build.target == target && !matches!(build.stage, Stage::Ready(_))
            });
            match (built, review.filter(|review| review.report.target == target)) {
                (Some(build), _) => self::build(build, screen, view, p),
                (None, Some(review)) => self::review(review, screen, view, p),
                (None, None) => (
                    super::parts::views(None, Some(back(p)), p),
                    vec![Block::note("Starting the build…")],
                ),
            }
        }
        Screen::Notes(target) => match &state.notes {
            Some(shown) => notes(shown, target, p),
            None => (super::parts::views(None, Some(back(p)), p), Vec::new()),
        },
        Screen::Diff(_, index) => match review {
            Some(review) => diff(review, index, p),
            None => (super::parts::views(None, Some(back(p)), p), Vec::new()),
        },
    }
}

fn back(p: &PanelCx<'_>) -> crate::panel::Handler {
    p.on_local::<super::runtime::Molt>(|_, view: &mut View| view.open(Screen::Home))
}

pub(super) fn panel(state: &Updates, p: &PanelCx<'_>) -> PanelView {
    let Some(view) = p.local::<View>() else {
        let (header, blocks) = home(state, p);
        return PanelView {
            scroll: false,
            header: Header::Views(header),
            blocks,
        };
    };
    let (mut header, blocks) = screen(view.screen, state, view, p);
    header.passing = view.passing.from.map(|from| {
        let (leaving, leaving_blocks) = screen(from, state, view, p);
        let back = view.screen.depth() < from.depth();
        let tool = match (from, view.screen) {
            (Screen::Home, tool) | (tool, Screen::Home) => tool.card(),
            _ => None,
        };
        Passing {
            blocks: leaving_blocks,
            at: view.passing.at.value().clamp(0.0, 1.0),
            rightwards: !back,
            width: view.passing.width.clone(),
            grow: tool.map(|anchor| Grow {
                anchor: anchor.into(),
                back,
                tint: None,
            }),
            views: Some(Box::new(leaving)),
        }
    });
    PanelView {
        scroll: false,
        header: Header::Views(header),
        blocks,
    }
}
