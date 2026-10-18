use gpui::Keystroke;
use tracing::error;

use super::{
    desktop::{self, App},
    history,
    runtime::Launcher,
    state::Local,
};
use crate::{
    components::list::{self, ROWS},
    features::Outcome,
    launch,
};

impl Launcher {
    pub(super) fn launch(&self, app: &App) {
        match desktop::launch(app, &self.settings) {
            Some(launch) => {
                history::record(&app.id);
                launch::spawn(launch);
            }
            None => error!(
                app = app.name,
                exec = app.exec,
                "cannot launch: unusable Exec line or terminal command"
            ),
        }
    }

    pub(super) fn handle_key(&self, local: &mut Local, keystroke: &Keystroke) -> Outcome {
        match keystroke.key.as_str() {
            "escape" => return Outcome::Close,
            "enter" => {
                if let Some(app) = local.selected() {
                    self.launch(app);
                }
                return Outcome::Close;
            }
            "up" => local.step(-1),
            "down" | "tab" => local.step(1),
            "pageup" => local.step(-(ROWS as i32)),
            "pagedown" => local.step(ROWS as i32),
            "backspace" => local.search(|query| {
                query.pop();
            }),
            "u" if keystroke.modifiers.control => local.search(String::clear),
            _ => {
                if let Some(text) = list::typed(keystroke) {
                    local.search(|query| query.push_str(text));
                }
            }
        }
        Outcome::Open
    }
}
