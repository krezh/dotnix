use std::{rc::Rc, time::Duration};

use futures::{StreamExt, channel::mpsc::UnboundedReceiver};
use gpui::{
    AnyWindowHandle, App, AppContext, Bounds, Context, DisplayEvent, DisplayId, Entity, Global,
    Keystroke, Size, Subscription, WeakEntity, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    point, px,
};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    config,
    control::{Asked, Control, Reply, Report, Row},
    cover::{ClickCatcher, KeyGrab},
    features::{self, Features},
    glyphs::Glyphs,
    island::{COLLAPSED_H, Grab, Island, IslandEvent, Placement, SURFACE_H, SURFACE_W, Startup},
    sys::{self, Samples},
    wm::Snapshot,
};

/// The island's layer-shell namespace, which compositors list its surfaces under.
pub const NAMESPACE: &str = "island";

const UNBOUND_AFTER: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct Config {
    pub layout: Rc<config::Bar>,
    pub features: Rc<toml::Table>,
    pub top: f32,
    pub corner: (f32, f32),
    pub opacity: f32,
    pub shadow: f32,
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
    /// Over the holder's output.
    cover: Option<AnyWindowHandle>,
    /// Over every other output.
    catchers: Vec<AnyWindowHandle>,
    holder: Option<(WeakEntity<Island>, Grab)>,
    subscriptions: Vec<Subscription>,
    slots: Vec<Slot>,
    workspaces: Option<Snapshot>,
    opened_any: bool,
}

struct Running(#[allow(dead_code)] Entity<Shell>);

fn cover(
    display: Option<DisplayId>,
    namespace: &str,
    keyboard: KeyboardInteractivity,
) -> WindowOptions {
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
        mut controls: UnboundedReceiver<Asked>,
        workspaces: Option<UnboundedReceiver<Snapshot>>,
        cx: &mut App,
    ) {
        let shell = cx.new(|cx: &mut Context<Self>| {
            let this = cx.weak_entity();
            let displays = cx.observe_displays(move |event, cx| {
                if matches!(event, DisplayEvent::Added(_) | DisplayEvent::Removed(_)) {
                    let _ = this.update(cx, |shell, cx| shell.reconcile(false, cx));
                }
            });
            // The compositor closes an island's surface when its output goes, and may on its own.
            // Deferred, as the shell closing its own covers lands here mid-update.
            let this = cx.weak_entity();
            let closed = cx.on_window_closed(move |cx, closed| {
                let this = this.clone();
                cx.defer(move |cx| {
                    let _ = this.update(cx, |shell, cx| {
                        if shell
                            .slots
                            .iter()
                            .any(|slot| slot.window.window_id() == closed)
                        {
                            shell.reconcile(false, cx);
                        }
                    });
                });
            });

            // With no output announced shortly after start, open one island unbound to any.
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(UNBOUND_AFTER).await;
                let _ = this.update(cx, |shell, cx| {
                    if !shell.opened_any {
                        shell.reconcile(true, cx);
                    }
                });
            })
            .detach();

            cx.spawn(async move |this, cx| {
                while let Some(Asked { control, answer }) = controls.next().await {
                    let Ok(answered) = this.update(cx, |shell, cx| shell.control(control, cx))
                    else {
                        break;
                    };
                    match answered {
                        Ok(Reply::Later(report)) => cx
                            .background_executor()
                            .spawn(async move {
                                let report = report.await.unwrap_or_else(|_| {
                                    Err("island stopped before answering".to_owned())
                                });
                                let _ = answer.send(report);
                            })
                            .detach(),
                        Ok(Reply::Now(report)) => {
                            let _ = answer.send(Ok(report));
                        }
                        Err(err) => {
                            let _ = answer.send(Err(err));
                        }
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
                cover: None,
                catchers: Vec::new(),
                holder: None,
                subscriptions: vec![displays, closed],
                slots: Vec::new(),
                workspaces: None,
                opened_any: false,
            }
        });
        shell.update(cx, |shell, cx| shell.reconcile(false, cx));
        cx.set_global(Running(shell));
    }

    /// In the background, so startup does not wait on the icon theme.
    fn look_up_glyphs(cx: &mut Context<Self>) {
        cx.set_global(Glyphs::default());
        cx.spawn(async move |_, cx| {
            let glyphs = cx
                .background_spawn(async {
                    Glyphs::resolve(&[features::glyphs(), crate::bar::GLYPHS.to_vec()].concat())
                })
                .await;
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
                || slot
                    .display
                    .is_some_and(|display| !present.contains(&display))
                || (slot.display.is_none() && !present.is_empty());
            if gone {
                info!(output = ?slot.display, "closing the island of a removed output");
                let _ = slot
                    .window
                    .update(cx, |_, window, _| window.remove_window());
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
            shadow: self.config.shadow,
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
            app_id: Some(NAMESPACE.to_string()),
            window_background: WindowBackgroundAppearance::Transparent,
            inactive_frame_interval: None,
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: NAMESPACE.to_string(),
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
                self.subscriptions.push(cx.subscribe(
                    &entity,
                    |shell, island, event, cx| match event {
                        IslandEvent::Grab(Grab::None) => shell.released(&island, cx),
                        IslandEvent::Grab(grab) => shell.grab(island, *grab, cx),
                    },
                ));
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

    /// The island with something open, so `island ctl` acts on what is showing, or else the one on
    /// the focused monitor.
    fn target(&self, cx: &App) -> Option<Entity<Island>> {
        if let Some(holder) = self.holding() {
            return Some(holder);
        }
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

    fn holding(&self) -> Option<Entity<Island>> {
        self.holder
            .as_ref()
            .and_then(|(island, _)| island.upgrade())
    }

    /// An island already replaced as the holder changes nothing.
    fn released(&mut self, island: &Entity<Island>, cx: &mut Context<Self>) {
        if self
            .holding()
            .is_none_or(|holder| holder.entity_id() == island.entity_id())
        {
            self.release(cx);
        }
    }

    fn release(&mut self, cx: &mut Context<Self>) {
        self.holder = None;
        self.remove_covers(cx);
    }

    fn remove_covers(&mut self, cx: &mut Context<Self>) {
        for handle in self.cover.take().into_iter().chain(self.catchers.drain(..)) {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }
    }

    pub fn key(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) {
        match self.holding() {
            Some(island) => island.update(cx, |island, cx| island.key(keystroke, cx)),
            None => self.release(cx),
        }
    }

    pub fn close_grabbed(&mut self, cx: &mut Context<Self>) {
        match self.holding() {
            Some(island) => island.update(cx, |island, cx| island.dismiss(cx)),
            None => self.release(cx),
        }
    }

    /// Closes what any other island had open.
    fn grab(&mut self, island: Entity<Island>, grab: Grab, cx: &mut Context<Self>) {
        match self.holder.replace((island.downgrade(), grab)) {
            Some((previous, held)) => match previous.upgrade() {
                Some(previous) if previous.entity_id() != island.entity_id() => {
                    previous.update(cx, |previous, cx| previous.dismiss(cx));
                    self.remove_covers(cx);
                }
                Some(_) if held == grab => {}
                _ => self.remove_covers(cx),
            },
            None => self.remove_covers(cx),
        }
        if self.cover.is_some() {
            return;
        }
        let display = self
            .slots
            .iter()
            .find(|slot| slot.island.entity_id() == island.entity_id())
            .and_then(|slot| slot.display);
        let shell = cx.entity().downgrade();
        let own = shell.clone();
        let opened = if grab == Grab::Keyboard {
            cx.open_window(
                cover(display, "island-keyboard", KeyboardInteractivity::OnDemand),
                move |_, cx| cx.new(|cx| KeyGrab::new(own, &island, cx)),
            )
            .map(AnyWindowHandle::from)
        } else {
            cx.open_window(
                cover(display, "island-catcher", KeyboardInteractivity::None),
                move |_, cx| cx.new(|cx| ClickCatcher::around(own, &island, cx)),
            )
            .map(AnyWindowHandle::from)
        };
        match opened {
            Ok(handle) => self.cover = Some(handle),
            Err(error) => error!(?grab, "cannot cover the island's output: {error:#}"),
        }
        let others: Vec<DisplayId> = self
            .slots
            .iter()
            .filter_map(|slot| slot.display)
            .filter(|other| Some(*other) != display)
            .collect();
        for other in others {
            let shell = shell.clone();
            match cx.open_window(
                cover(Some(other), "island-catcher", KeyboardInteractivity::None),
                move |_, cx| cx.new(|_| ClickCatcher::new(shell)),
            ) {
                Ok(handle) => self.catchers.push(handle.into()),
                Err(error) => warn!(output = ?other, "cannot cover the output: {error:#}"),
            }
        }
    }

    fn open_panel(
        &mut self,
        name: &str,
        view: Option<&str>,
        toggle: bool,
        cx: &mut Context<Self>,
    ) -> Result<Report, String> {
        let island = self.target(cx).ok_or("No island is showing")?;
        let open = island.update(cx, |island, cx| island.open_named(name, view, toggle, cx));
        let output = island
            .read(cx)
            .output()
            .map(|output| format!(" on {output}"))
            .unwrap_or_default();
        let shown = view.map(|view| format!(" on {view}")).unwrap_or_default();
        Ok(Report::did(Row::said(if open {
            format!("Opened {name}{shown}{output}")
        } else {
            format!("Closed {name}{output}")
        })))
    }

    fn status(&self, cx: &App) -> Report {
        let outputs: Vec<String> = self
            .workspaces
            .iter()
            .flat_map(|snapshot| snapshot.monitors.iter().map(|monitor| monitor.name.clone()))
            .collect();
        let on = if outputs.is_empty() {
            String::new()
        } else {
            format!(" · running on {}", outputs.join(", "))
        };
        Report {
            title: Some(format!("island {}{on}", env!("CARGO_PKG_VERSION"))),
            changed: false,
            rows: self.features.status(cx),
        }
    }

    fn control(&mut self, control: Control, cx: &mut Context<Self>) -> Result<Reply, String> {
        match control {
            Control::Open(name, view) => self
                .open_panel(&name, view.as_deref(), false, cx)
                .map(Reply::from),
            Control::Toggle(name, view) => self
                .open_panel(&name, view.as_deref(), true, cx)
                .map(Reply::from),
            Control::Close => {
                for island in self.islands().collect::<Vec<_>>() {
                    island.update(cx, |island, cx| island.dismiss(cx));
                }
                Ok(Report::did(Row::said("Closed what was open")).into())
            }
            Control::Feature(name, args) => {
                let feature = self
                    .features
                    .get(name)
                    .ok_or_else(|| format!("The {name} feature is not running"))?;
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                Ok(feature.command(&args, cx))
            }
            Control::Status => Ok(self.status(cx).into()),
            Control::Log(_) => Ok(Report::default().into()),
        }
    }
}
