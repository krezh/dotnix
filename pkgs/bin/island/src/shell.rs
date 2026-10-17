use std::{rc::Rc, sync::Arc, time::Duration};

use futures::{StreamExt, channel::mpsc::UnboundedReceiver};
use gpui::{
    AnyWindowHandle, App, AppContext, Bounds, Context, DisplayId, Entity, Global, Keystroke, Size, Subscription,
    WeakEntity, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    point, px,
};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    config,
    control::Control,
    desktop,
    features::{self, Features},
    glyphs::Glyphs,
    island::{COLLAPSED_H, Island, IslandEvent, Mode, Panel, Placement, SURFACE_H, SURFACE_W, Startup},
    launcher::{ClickCatcher, KeyGrab},
    launches,
    sys::{self, Samples},
    wm::Snapshot,
};

const DISPLAY_POLL: Duration = Duration::from_secs(2);
const STARTUP_POLL: Duration = Duration::from_millis(100);
const STARTUP_ATTEMPTS: u32 = 10;

#[derive(Clone)]
pub struct Config {
    pub layout: Rc<config::Bar>,
    /// The features' sections of the config file.
    pub features: Rc<toml::Table>,
    pub top: f32,
    pub corner: (f32, f32),
    pub opacity: f32,
    pub glow: f32,
}

struct Slot {
    display: Option<DisplayId>,
    window: AnyWindowHandle,
    island: WeakEntity<Island>,
}

pub struct Shell {
    config: Config,
    features: Rc<Features>,
    samples: Samples,
    apps: Arc<Vec<desktop::App>>,
    /// The surface that holds the keyboard while a panel that takes it is open.
    keyboard: Option<AnyWindowHandle>,
    /// Surfaces over every other output meanwhile, so a click on any of them closes the panel.
    catchers: Vec<AnyWindowHandle>,
    /// The island whose panel has the keyboard.
    keyboard_island: Option<WeakEntity<Island>>,
    subscriptions: Vec<Subscription>,
    slots: Vec<Slot>,
    workspaces: Option<Snapshot>,
    opened_any: bool,
}

struct Running(#[allow(dead_code)] Entity<Shell>);

/// A transparent surface over the whole of `display`, above everything, while a panel takes the
/// keyboard.
fn cover(display: Option<DisplayId>, namespace: &str, keyboard: KeyboardInteractivity) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        // A layer surface anchored to every edge fills the output only when it asks for no size;
        // given one, the compositor centres it at that size instead.
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.0), px(0.0)),
            size: Size::new(px(0.0), px(0.0)),
        })),
        display_id: display,
        app_id: Some(namespace.to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: namespace.to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: Some(px(-1.0)),
            keyboard_interactivity: keyboard,
            ..Default::default()
        }),
        ..Default::default()
    }
}

impl Global for Running {}

impl Shell {
    pub fn start(
        config: Config,
        mut controls: UnboundedReceiver<Control>,
        workspaces: Option<UnboundedReceiver<Snapshot>>,
        cx: &mut App,
    ) {
        let shell = cx.new(|cx: &mut Context<Self>| {
            cx.spawn(async move |this, cx| {
                let mut attempts = 0;
                loop {
                    let Ok(opened) = this.update(cx, |shell, cx| {
                        shell.reconcile(attempts >= STARTUP_ATTEMPTS, cx);
                        shell.opened_any
                    }) else {
                        break;
                    };
                    attempts += 1;
                    let wait = if opened { DISPLAY_POLL } else { STARTUP_POLL };
                    cx.background_executor().timer(wait).await;
                }
            })
            .detach();

            cx.spawn(async move |this, cx| {
                while let Some(control) = controls.next().await {
                    if this.update(cx, |shell, cx| shell.control(control, cx)).is_err() {
                        break;
                    }
                }
            })
            .detach();

            if let Some(mut snapshots) = workspaces {
                cx.spawn(async move |this, cx| {
                    while let Some(snapshot) = snapshots.next().await {
                        let alive = this.update(cx, |shell, cx| {
                            shell.set_workspaces(snapshot, cx);
                            shell.reconcile(false, cx);
                        });
                        if alive.is_err() {
                            break;
                        }
                    }
                })
                .detach();
            }

            cx.spawn(async move |this, cx| {
                let apps = cx
                    .background_spawn(async {
                        launches::load();
                        desktop::load()
                    })
                    .await;
                let _ = this.update(cx, |shell, _| shell.apps = Arc::new(apps));
            })
            .detach();

            Self::look_up_glyphs(cx);
            cx.set_global(Placement::default());

            let layout = config.layout.clone();
            let features = Rc::new(features::start(
                |name| layout.widgets().any(|widget| widget.0 == name),
                config.glow > 0.0,
                &config.features,
                cx,
            ));

            Self {
                config,
                features,
                samples: sys::watch(),
                apps: Arc::new(Vec::new()),
                keyboard: None,
                catchers: Vec::new(),
                keyboard_island: None,
                subscriptions: Vec::new(),
                slots: Vec::new(),
                workspaces: None,
                opened_any: false,
            }
        });
        cx.set_global(Running(shell));
    }

    /// Starts with no icons and looks them up in the background, so startup does not wait on the
    /// icon theme.
    fn look_up_glyphs(cx: &mut Context<Self>) {
        cx.set_global(Glyphs::default());
        cx.spawn(async move |_, cx| {
            let glyphs = cx.background_spawn(async { Glyphs::resolve(&features::glyphs()) }).await;
            cx.update(|cx| cx.set_global(glyphs));
        })
        .detach();
    }

    fn islands(&self) -> impl Iterator<Item = Entity<Island>> + '_ {
        self.slots.iter().filter_map(|slot| slot.island.upgrade())
    }

    fn set_workspaces(&mut self, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.workspaces = Some(snapshot.clone());
        for island in self.islands().collect::<Vec<_>>() {
            let snapshot = snapshot.clone();
            island.update(cx, |island, cx| island.set_workspaces(snapshot, cx));
        }
        self.place(cx);
    }

    /// Tells the features which islands are open.
    fn place(&self, cx: &mut Context<Self>) {
        let placement = Placement {
            islands: self.slots.iter().map(|slot| slot.island.clone()).collect(),
        };
        cx.set_global(placement);
    }

    fn reconcile(&mut self, allow_unbound: bool, cx: &mut Context<Self>) {
        let displays = cx.displays();
        let present: Vec<DisplayId> = displays.iter().map(|display| display.id()).collect();

        let mut kept = Vec::with_capacity(self.slots.len());
        for slot in self.slots.drain(..) {
            let gone = slot.island.upgrade().is_none()
                || slot.display.is_some_and(|display| !present.contains(&display))
                || (slot.display.is_none() && !present.is_empty());
            if gone {
                info!(output = ?slot.display, "closing the island of a removed output");
                let _ = slot.window.update(cx, |_, window, _| window.remove_window());
            } else {
                kept.push(slot);
            }
        }
        self.slots = kept;

        if present.is_empty() {
            if allow_unbound && self.slots.is_empty() {
                self.open(None, cx);
            }
        } else {
            for display in present {
                if !self.slots.iter().any(|slot| slot.display == Some(display)) {
                    self.open(Some(display), cx);
                }
            }
        }
        self.place(cx);
    }

    fn open(&mut self, display: Option<DisplayId>, cx: &mut Context<Self>) {
        let startup = Startup {
            layout: self.config.layout.clone(),
            top: self.config.top,
            corner: self.config.corner,
            opacity: self.config.opacity,
            glow: self.config.glow,
            workspaces: self.workspaces.clone(),
            features: self.features.clone(),
            samples: self.samples.clone(),
        };
        let options = WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: Size::new(px(SURFACE_W), px(SURFACE_H)),
            })),
            display_id: display,
            app_id: Some("island".to_string()),
            window_background: WindowBackgroundAppearance::Transparent,
            inactive_frame_interval: None,
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "island".to_string(),
                layer: Layer::Top,
                anchor: Anchor::TOP,
                exclusive_zone: Some(px(self.config.top + COLLAPSED_H)),
                keyboard_interactivity: KeyboardInteractivity::None,
                ..Default::default()
            }),
            ..Default::default()
        };
        let output = display;
        match cx.open_window(options, |_, cx| cx.new(|cx| Island::new(startup, cx))) {
            Ok(handle) => {
                let Ok(entity) = handle.update(cx, |_, _, cx| cx.entity()) else {
                    return;
                };
                let island = entity.downgrade();
                self.subscriptions.push(cx.subscribe(&entity, |shell, island, event, cx| match event {
                    IslandEvent::KeyboardWanted => shell.grab_keyboard(island, cx),
                    IslandEvent::KeyboardReleased => shell.keyboard_released(&island, cx),
                }));
                self.slots.push(Slot {
                    display,
                    window: handle.into(),
                    island,
                });
                self.opened_any = true;
                info!(?output, "opened an island");
            }
            Err(error) => error!(?output, "cannot open the island's layer surface: {error:#}"),
        }
    }

    fn target(&self, cx: &App) -> Option<Entity<Island>> {
        let focused = self
            .workspaces
            .as_ref()
            .and_then(Snapshot::focused)
            .map(|monitor| Uuid::new_v5(&Uuid::NAMESPACE_DNS, monitor.name.as_bytes()));
        focused
            .and_then(|focused| {
                self.islands()
                    .find(|island| island.read(cx).display() == Some(focused))
            })
            .or_else(|| self.islands().next())
    }

    /// `island` closed its panel that took the keyboard; one that has since handed the keyboard to
    /// another island changes nothing.
    fn keyboard_released(&mut self, island: &Entity<Island>, cx: &mut Context<Self>) {
        let holder = self.keyboard_island.as_ref().and_then(WeakEntity::upgrade);
        if holder.is_none_or(|holder| holder.entity_id() == island.entity_id()) {
            self.release_keyboard(cx);
        }
    }

    fn release_keyboard(&mut self, cx: &mut Context<Self>) {
        self.keyboard_island = None;
        self.remove_covers(cx);
    }

    /// Removes the surface holding the keyboard and the click catchers on the other outputs.
    fn remove_covers(&mut self, cx: &mut Context<Self>) {
        for handle in self.keyboard.take().into_iter().chain(self.catchers.drain(..)) {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }
    }

    pub fn key(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) {
        match self.keyboard_island.as_ref().and_then(WeakEntity::upgrade) {
            Some(island) => island.update(cx, |island, cx| island.key(keystroke, cx)),
            None => self.release_keyboard(cx),
        }
    }

    fn launcher(&mut self, toggle: bool, cx: &mut Context<Self>) {
        let island = match self.keyboard_island.as_ref().and_then(WeakEntity::upgrade) {
            Some(island) if island.read(cx).mode() == Mode::Open(Panel::Launcher) => {
                if toggle {
                    island.update(cx, |island, cx| island.set_mode(Mode::Collapsed, cx));
                }
                return;
            }
            Some(island) => island,
            None => match self.target(cx) {
                Some(island) => island,
                None => return,
            },
        };
        let apps = if self.apps.is_empty() { Arc::new(desktop::load()) } else { self.apps.clone() };
        island.update(cx, |island, cx| island.open_launcher(apps, cx));
    }

    /// Closes the panel that has the keyboard, for a click outside the island.
    pub fn close_keyboard_panel(&mut self, cx: &mut Context<Self>) {
        match self.keyboard_island.as_ref().and_then(WeakEntity::upgrade) {
            Some(island) => island.update(cx, |island, cx| island.set_mode(Mode::Collapsed, cx)),
            None => self.release_keyboard(cx),
        }
    }

    /// Hands the keyboard to `island`, whose panel takes it, closing the panel of any other island
    /// that had it. The surface holding the keyboard covers `island`'s output, and click catchers
    /// cover every other output.
    fn grab_keyboard(&mut self, island: Entity<Island>, cx: &mut Context<Self>) {
        let previous = self.keyboard_island.replace(island.downgrade()).and_then(|previous| previous.upgrade());
        if let Some(previous) = previous.filter(|previous| previous.entity_id() != island.entity_id()) {
            previous.update(cx, |previous, cx| previous.set_mode(Mode::Collapsed, cx));
            self.remove_covers(cx);
        }
        if self.keyboard.is_some() {
            return;
        }
        let display = self
            .slots
            .iter()
            .find(|slot| slot.island.entity_id() == island.entity_id())
            .and_then(|slot| slot.display);
        let shell = cx.entity().downgrade();
        let grab = shell.clone();
        match cx.open_window(cover(display, "island-keyboard", KeyboardInteractivity::OnDemand), move |_, cx| {
            cx.new(|cx| KeyGrab::new(grab, &island, cx))
        }) {
            Ok(handle) => self.keyboard = Some(handle.into()),
            Err(error) => error!("cannot grab the keyboard: {error:#}"),
        }
        let others: Vec<DisplayId> = self.slots.iter().filter_map(|slot| slot.display).filter(|other| Some(*other) != display).collect();
        for other in others {
            let shell = shell.clone();
            match cx.open_window(cover(Some(other), "island-catcher", KeyboardInteractivity::None), move |_, cx| {
                cx.new(|_| ClickCatcher::new(shell))
            }) {
                Ok(handle) => self.catchers.push(handle.into()),
                Err(error) => warn!(output = ?other, "cannot cover the output: {error:#}"),
            }
        }
    }

    fn control(&mut self, control: Control, cx: &mut Context<Self>) {
        match control {
            Control::Open(name) if name == "launcher" => self.launcher(false, cx),
            Control::Toggle(name) if name == "launcher" => self.launcher(true, cx),
            Control::Open(name) => {
                if let Some(island) = self.target(cx) {
                    island.update(cx, |island, cx| island.open_named(&name, false, cx));
                }
            }
            Control::Toggle(name) => {
                if let Some(island) = self.target(cx) {
                    island.update(cx, |island, cx| island.open_named(&name, true, cx));
                }
            }
            Control::Close => {
                for island in self.islands().collect::<Vec<_>>() {
                    island.update(cx, |island, cx| island.set_mode(Mode::Collapsed, cx));
                }
            }
            Control::Feature(name, args) => {
                if let Some(feature) = self.features.get(name) {
                    let args: Vec<&str> = args.iter().map(String::as_str).collect();
                    feature.command(&args, cx);
                }
            }
            Control::Log(_) => {}
        }
    }
}
