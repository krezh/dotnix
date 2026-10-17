mod desktop;
mod history;

use std::sync::Arc;

use gpui::Keystroke;
use tracing::error;

use super::{Feature, Feed, Local, Outcome, PanelCx};
use crate::{
    components::list::{self, ROWS},
    island::Mode,
    launch,
    panel::{AppRow, Block, Header, LauncherView, PanelView, on},
    supervisor::Ended,
};
use desktop::App;

#[derive(Clone, Default)]
pub struct Apps(Arc<Vec<App>>);

impl PartialEq for Apps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Apps {
    fn matching(&self, query: &str) -> Vec<&App> {
        desktop::search(&self.0, query, &history::ranks())
            .into_iter()
            .filter_map(|index| self.0.get(index))
            .collect()
    }
}

#[derive(Default)]
pub struct Search {
    query: String,
    selected: usize,
    top: usize,
}

impl Search {
    fn select(&mut self, position: usize, count: usize) {
        self.selected = position.min(count.saturating_sub(1));
        self.top = list::reveal(self.selected, self.top, count);
    }

    fn step(&mut self, step: i32, count: usize) {
        self.select(list::stepped(self.selected.min(count.saturating_sub(1)), step, count), count);
    }

    fn search(&mut self, change: impl FnOnce(&mut String)) {
        change(&mut self.query);
        self.selected = 0;
        self.top = 0;
    }
}

impl Local for Search {
    fn panel_closed(&mut self) {
        *self = Self::default();
    }
}

fn launch(app: &App) {
    match desktop::launch(app) {
        Some(launch) => {
            history::record(&app.id);
            launch::spawn(launch);
        }
        None => error!(app = app.name, exec = app.exec, "cannot launch: unusable Exec line"),
    }
}

#[derive(Clone)]
pub struct Launcher;

impl Feature for Launcher {
    const NAME: &'static str = "launcher";
    const PANEL: Option<&'static str> = Some("launcher");
    const KEYBOARD: bool = true;

    type State = Apps;
    type Local = Search;
    type Event = ();

    fn start(feed: Feed<Apps>) -> Self {
        feed.supervise(|feed| {
            history::load();
            feed.publish(Apps(Arc::new(desktop::load())));
            Ended::Unwanted
        });
        Self
    }

    fn panel<'a>(&self, apps: &Apps, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        let search = p.local::<Search>()?;
        let matching = apps.matching(&search.query);
        let count = matching.len();
        let selected = search.selected.min(count.saturating_sub(1));
        let top = list::reveal(selected, search.top, count);
        let rows = matching
            .iter()
            .enumerate()
            .skip(top)
            .take(ROWS)
            .map(|(position, app)| {
                let picked = (*app).clone();
                AppRow {
                    id: format!("app-{}", app.id).into(),
                    name: app.name.clone().into(),
                    detail: if app.generic.is_empty() { &app.comment } else { &app.generic }.clone().into(),
                    icon: app.icon.clone(),
                    preview: false,
                    selected: position == selected,
                    on_click: on(move |island, cx| {
                        launch(&picked);
                        island.set_mode(Mode::Collapsed, cx);
                    }),
                    on_hover: p.on_local::<Self>(move |_, search| search.select(position, count)),
                }
            })
            .collect();
        Some(PanelView {
            header: Header::None,
            blocks: vec![Block::Launcher(LauncherView {
                query: search.query.clone().into(),
                placeholder: "Search apps".into(),
                empty: "No apps found".into(),
                rows,
                on_step: p.on_local_with::<Self, i32>(move |_, search, step| search.step(step, count)),
                preview: None,
            })],
        })
    }

    fn key(&self, apps: &Apps, search: &mut Search, keystroke: &Keystroke) -> Outcome {
        let matching = apps.matching(&search.query);
        let count = matching.len();
        match keystroke.key.as_str() {
            "escape" => return Outcome::Close,
            "enter" => {
                if let Some(app) = matching.get(search.selected.min(count.saturating_sub(1))) {
                    launch(app);
                }
                return Outcome::Close;
            }
            "up" => search.step(-1, count),
            "down" | "tab" => search.step(1, count),
            "pageup" => search.step(-(ROWS as i32), count),
            "pagedown" => search.step(ROWS as i32, count),
            "backspace" => search.search(|query| {
                query.pop();
            }),
            "u" if keystroke.modifiers.control => search.search(String::clear),
            _ => {
                if let Some(text) = list::typed(keystroke) {
                    search.search(|query| query.push_str(text));
                }
            }
        }
        Outcome::Open
    }
}
