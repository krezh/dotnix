//! What a click reaches. Features and blocks hold a [`Handler`] and know only [`Ui`]; the frontend
//! says what each thing does.

use futures::channel::oneshot;

use super::model::Handler;
use crate::{features::AnyLocal, menu::MenuNode, stage::Panel};

pub trait Ui {
    fn run(&mut self, handler: &Handler);
    fn open(&mut self, panel: Panel);
    fn toggle(&mut self, panel: Panel);
    fn open_named(&mut self, name: &str, view: Option<&str>, toggle: bool);
    fn collapse(&mut self);
    /// Keeps the event from reaching what lies under it.
    fn stop_propagation(&mut self);
    /// Where a menu at `x` opens, as the screen sees it.
    fn below_bar(&self, x: f32) -> Option<(i32, i32)>;
    fn menu_open_for(&self, source: &str) -> bool;
    fn close_menu(&mut self);
    /// Hangs the menu `response` brings from the bar at `anchor`.
    fn menu_when_ready(
        &mut self,
        source: String,
        response: oneshot::Receiver<Option<MenuNode>>,
        anchor: f32,
        pick: Box<dyn Fn(i32)>,
    );
    /// Changes this island's own part of the feature.
    fn update_local(&mut self, name: &'static str, action: &mut dyn FnMut(&mut dyn AnyLocal));
}
