//! What a click reaches. Features and blocks hold a [`Handler`] and know only [`Ui`]; the frontend
//! says what each thing does.

use std::rc::Rc;

use futures::channel::oneshot;
use gpui::{Context, px};
use tracing::{debug, warn};

use super::model::Handler;
use crate::{
    features::AnyLocal,
    island::{Island, Mode, Panel},
    menu::MenuNode,
};

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
    /// Goes into the entry of the open menu at `index`.
    fn menu_enter(&mut self, index: usize);
    fn menu_pick(&mut self, id: i32);
    /// Changes this island's own part of the feature.
    fn update_local(&mut self, name: &'static str, action: &mut dyn FnMut(&mut dyn AnyLocal));
}

pub struct IslandUi<'a, 'b> {
    island: &'a mut Island,
    cx: &'a mut Context<'b, Island>,
}

impl<'a, 'b> IslandUi<'a, 'b> {
    pub fn new(island: &'a mut Island, cx: &'a mut Context<'b, Island>) -> Self {
        Self { island, cx }
    }
}

impl Ui for IslandUi<'_, '_> {
    fn run(&mut self, handler: &Handler) {
        handler(self);
    }

    fn open(&mut self, panel: Panel) {
        self.island.set_mode(Mode::Open(panel), self.cx);
    }

    fn toggle(&mut self, panel: Panel) {
        self.island.toggle(panel, self.cx);
    }

    fn open_named(&mut self, name: &str, view: Option<&str>, toggle: bool) {
        self.island.open_named(name, view, toggle, self.cx);
    }

    fn collapse(&mut self) {
        self.island.set_mode(Mode::Collapsed, self.cx);
    }

    fn stop_propagation(&mut self) {
        self.cx.stop_propagation();
    }

    fn below_bar(&self, x: f32) -> Option<(i32, i32)> {
        self.island.below_bar(px(x))
    }

    fn menu_open_for(&self, source: &str) -> bool {
        self.island.menu_open_for(source)
    }

    fn close_menu(&mut self) {
        self.island.close_menu(self.cx);
    }

    fn menu_when_ready(
        &mut self,
        source: String,
        response: oneshot::Receiver<Option<MenuNode>>,
        anchor: f32,
        pick: Box<dyn Fn(i32)>,
    ) {
        self.cx
            .spawn(async move |island, cx| {
                let root = match response.await {
                    Ok(Some(root)) if !root.children.is_empty() => root,
                    Ok(_) => {
                        debug!(source, "the menu is empty");
                        let _ = island.update(cx, |island, cx| island.close_menu(cx));
                        return;
                    }
                    Err(_) => {
                        warn!(source, "the menu request was dropped");
                        let _ = island.update(cx, |island, cx| island.close_menu(cx));
                        return;
                    }
                };
                debug!(source, entries = root.children.len(), "showing the menu");
                let pick = Rc::new(move |id, _: &mut Island, _: &mut Context<Island>| pick(id));
                let _ = island.update(cx, |island, cx| {
                    island.show_menu(source, root, pick, px(anchor), cx);
                });
            })
            .detach();
    }

    fn menu_enter(&mut self, index: usize) {
        self.island.menu_enter(index, self.cx);
    }

    fn menu_pick(&mut self, id: i32) {
        self.island.menu_pick(id, self.cx);
    }

    fn update_local(&mut self, name: &'static str, action: &mut dyn FnMut(&mut dyn AnyLocal)) {
        self.island.update_local(name, action);
        self.island.local_changed(name, self.cx);
    }
}
