use std::sync::Arc;

use gpui::{
    Bounds, Context, FocusHandle, KeyDownEvent, Keystroke, Render, WeakEntity, Window, div, point, prelude::*, px,
    size,
};

use crate::{
    desktop::{self, App},
    launch,
    shell::Shell,
};

pub const ROWS: usize = 8;

pub struct Search {
    pub apps: Arc<Vec<App>>,
    pub query: String,
    pub results: Vec<usize>,
    pub selected: usize,
}

pub enum Outcome {
    Open,
    Close,
}

impl Search {
    pub fn new(apps: Arc<Vec<App>>) -> Self {
        let results = desktop::search(&apps, "");
        Self {
            apps,
            query: String::new(),
            results,
            selected: 0,
        }
    }

    fn refresh(&mut self) {
        self.results = desktop::search(&self.apps, &self.query);
        self.selected = 0;
    }

    pub fn visible(&self) -> impl Iterator<Item = (usize, &App)> {
        let start = self.selected.saturating_sub(ROWS - 1);
        self.results
            .iter()
            .enumerate()
            .skip(start)
            .take(ROWS)
            .filter_map(|(position, index)| Some((position, self.apps.get(*index)?)))
    }

    pub fn launch(&self, position: usize) {
        let app = self.results.get(position).and_then(|index| self.apps.get(*index));
        match app.map(|app| (app, desktop::launch(app))) {
            Some((_, Some(launch))) => launch::spawn(launch),
            Some((app, None)) => tracing::error!(app = app.name, exec = app.exec, "cannot launch: unusable Exec line"),
            None => {}
        }
    }

    pub fn key(&mut self, keystroke: &Keystroke) -> Outcome {
        match keystroke.key.as_str() {
            "escape" => return Outcome::Close,
            "enter" => {
                self.launch(self.selected);
                return Outcome::Close;
            }
            "up" => self.selected = self.selected.saturating_sub(1),
            "down" | "tab" => self.selected = (self.selected + 1).min(self.results.len().saturating_sub(1)),
            "backspace" => {
                self.query.pop();
                self.refresh();
            }
            "u" if keystroke.modifiers.control => {
                self.query.clear();
                self.refresh();
            }
            _ => {
                let typed = keystroke
                    .key_char
                    .as_deref()
                    .filter(|_| !keystroke.modifiers.control && !keystroke.modifiers.alt)
                    .filter(|text| text.chars().all(|char| !char.is_control()));
                if let Some(text) = typed {
                    self.query.push_str(text);
                    self.refresh();
                }
            }
        }
        Outcome::Open
    }
}

pub struct KeyGrab {
    shell: WeakEntity<Shell>,
    focus: FocusHandle,
    focused: bool,
}

impl KeyGrab {
    pub fn new(shell: WeakEntity<Shell>, cx: &mut Context<Self>) -> Self {
        Self {
            shell,
            focus: cx.focus_handle(),
            focused: false,
        }
    }
}

impl Render for KeyGrab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focused {
            self.focus.focus(window, cx);
            window.set_input_region(Some(&[Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(0.0), px(0.0)),
            }]));
            self.focused = true;
        }
        div()
            .size_full()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let keystroke = event.keystroke.clone();
                let _ = this.shell.update(cx, |shell, cx| shell.launcher_key(&keystroke, cx));
            }))
    }
}
