//! One island drawn with wgpu: a [`Stage`], its layer surface, and its bar.

use std::{
    any::Any,
    cell::Cell,
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{Future, channel::oneshot};
use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    shell::{WaylandSurface, wlr_layer::LayerSurface},
};
use tracing::debug;
use uuid::Uuid;
use wayland_client::{Proxy, QueueHandle};
use wayland_protocols::{
    ext::background_effect::v1::client::ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
    wp::idle_inhibit::zv1::client::{
        zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
        zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
    },
    wp::{
        fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1,
        viewporter::client::wp_viewport::WpViewport,
    },
};

use super::{
    app::EffectState,
    blocks::{Blocks, chip_label, chip_look},
    chrome::{self, Action, Chrome},
    menu::{self, Menu},
    node::Painting,
    node::div,
    paint::Painter,
    panel::{self, Built, DROP, MorphText, Shown},
    records,
    renderer::{Gpu, MERGED_FILL, MERGED_OUTLINE, MERGED_RIM, Presented, Sprite, inside},
    turn::Turn,
    widget::{Drawn, Hit, Layout, Tint, color, sets, tinted},
};
use crate::{
    bar::{self, TabStrip, WIDGET_H},
    bar_widget::{Pointer, Press},
    components::measure::Measure,
    components::{
        aura::{Aura, Part as AuraPart},
        badge::{PULSE, pulsed},
        card,
        chart::{self, Phase},
        overscroll::{LET_GO, REACH, settle},
        slider::{HANDLE_W, HELD_H, HOVER_H, STRETCH_REACH},
    },
    components::{
        marquee::{BRISK, TICK, smoothstep},
        slider::{HIT_H, OnChange},
    },
    compositor,
    config::{self, Widget},
    features::{self, AnyLocal, Outcome, WidgetCx},
    glyphs::Glyphs,
    key::Key,
    menu::MenuNode,
    merge::Merge,
    panel::{Block, Handler, SharedString, Ui},
    panels,
    spring::{Spring, rubber, unrubber},
    stage::{
        COLLAPSED_H, COLLAPSED_LAYER, CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT, Grab, Mode, Panel,
        SHAPE_SPRING, SURFACE_H, SURFACE_W, Shape, Stage, collapsed_width,
    },
    sys::{self, Samples, SysStats, Watch},
    theme::{ACCENT, HOVER_ALPHA, SURFACE, SURFACE_BORDER_ALPHA, SURFACE1, TEXT},
    ui::{CONTROL_RADIUS, INSET, SMALL_SIZE},
    wm::Snapshot,
};

const MAX_STEP: Duration = Duration::from_millis(50);
const TIP_DELAY: Duration = Duration::from_millis(500);
const TIP_GAP: (f32, f32) = (6.0, 24.0);
const TIP_LEADING: f32 = 1.618;
const TIP_PAD: (f32, f32) = (12.0, 7.0);
const TIP_RADIUS: f32 = 10.0;
const MENU_GAP: f32 = 6.0;
const LEAVE_GRACE: Duration = Duration::from_millis(500);
pub const PIXELS_PER_NOTCH: f32 = 50.0;
const LINES_PER_NOTCH: f32 = 3.0;
/// How far a notch of the wheel scrolls a panel.
const WHEEL_PIXELS: f32 = 69.0;
const ROW_PIXELS: f32 = 48.0;
const SHADOW_SIDE: f32 = 45.0;
const POPOVER_MARGIN: f32 = 8.0;
const AURA_EVERY: Duration = Duration::from_millis(33);
const HELD_DIM: f32 = 0.85;
const FILL_EPSILON: f32 = 0.001;
const BODY: &str = "panel-body";
const FOLLOW_SLACK: f32 = 24.0;
const GLIDE: Duration = Duration::from_millis(16);
const SHADOW_BELOW: f32 = 55.0;

pub struct Look {
    pub top: f32,
    pub corner: (f32, f32),
    pub opacity: f32,
    pub shadow: f32,
    pub glow: f32,
    pub samples: Samples,
    pub system: std::rc::Rc<config::System>,
}

struct Cached {
    drawn: Drawn,
    /// Drawn afresh at this time, or `None` until something changes.
    redraw_at: Option<Instant>,
}

/// How the pointer looks.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    Default,
    Pointer,
    Grab,
    Grabbing,
}

struct Over {
    /// How far past an end it is pulled, above 0 past the top.
    pull: f32,
    last: Option<Instant>,
    stretch: Spring,
}

enum Act {
    Chrome(Action),
    Panel(Handler),
    Bar(Press, f32),
    Menu(menu::Action),
}

/// Taken as a button went down; acts if the button comes up still on `rect`.
struct Pressed {
    act: Act,
    rect: [f32; 4],
    /// `rect` is in the panel's own coordinates.
    panel: bool,
    sprites: (usize, usize),
}

struct Growing {
    grown: Spring,
    revealed: Spring,
    open: bool,
    /// Where what opens it is, followed smoothly, as it can change size when picked from.
    left: Spring,
    width: Spring,
}

impl Growing {
    fn new() -> Self {
        Self {
            grown: Spring::new(0.0, SHAPE_SPRING, 0.001),
            revealed: Spring::new(0.0, CONTENT_IN, 0.002),
            open: false,
            left: Spring::new(0.0, SHAPE_SPRING, 0.1),
            width: Spring::new(0.0, SHAPE_SPRING, 0.1),
        }
    }

    /// Whether its rows have grown far enough to answer the pointer.
    fn is_live(&self) -> bool {
        self.open && self.grown.value().clamp(0.0, 1.0) >= crate::components::popover::ROWS_AT
    }

    fn is_settled(&self) -> bool {
        self.grown.is_settled()
            && self.revealed.is_settled()
            && self.left.is_settled()
            && self.width.is_settled()
    }
}

/// A slider held by the pointer.
#[derive(Clone)]
struct Grip {
    key: u64,
    rect: [f32; 4],
    on_change: OnChange,
    /// Where on its end it was taken.
    offset: f32,
}

struct Awaited {
    source: String,
    response: oneshot::Receiver<Option<MenuNode>>,
    anchor: f32,
    pick: Box<dyn Fn(i32)>,
}

struct Tip {
    text: crate::panel::SharedString,
    since: Instant,
    at: (f32, f32),
    /// Reads off a chart, so changes as the pointer and the chart move.
    charted: bool,
}

/// A hit placed in the frame just composed.
struct Placed {
    widget: Widget,
    index: usize,
    x: f32,
    y: f32,
    w: f32,
}

pub struct Island {
    /// Goes before the layer it draws on.
    pub surface: Option<wgpu::Surface<'static>>,
    pub stage: Stage,
    pub layer: LayerSurface,
    pub output: wayland_client::protocol::wl_output::WlOutput,
    pub blur: Option<ExtBackgroundEffectSurfaceV1>,
    pub scale: f32,
    /// Keeps the compositor's fractional scale coming, while there is one.
    pub fractional: Option<(WpViewport, WpFractionalScaleV1)>,
    pub configured: bool,
    pub turn: Turn,
    pub dirty: bool,
    look: Look,
    cache: HashMap<Widget, Cached>,
    last_frame: Option<Instant>,
    started: Instant,
    pointer: Option<(f32, f32)>,
    tip: Option<Tip>,
    placed: Vec<Placed>,
    body: [f32; 4],
    region: Option<Vec<[i32; 4]>>,
    inhibitor: Option<ZwpIdleInhibitorV1>,
    pace: Option<Duration>,
    drew_at: Instant,
    strip: TabStrip,
    /// Each panel on show, as it was last laid out.
    built: HashMap<usize, Built>,
    /// Where everything easing in each built panel has got to.
    eases: HashMap<usize, HashMap<u64, Spring>>,
    /// Each busy widget's beam.
    bar_beams: HashMap<Widget, panel::Beam>,
    /// Each busy thing's beam in a panel.
    panel_beams: HashMap<u64, panel::Beam>,
    /// A beam showed in the last frame.
    beaming: bool,
    aura: Aura,
    aura_room: Option<usize>,
    /// What to draw of the glow as last made, and when.
    aura_shown: Option<([f32; 4], Instant)>,
    menu_aura_room: Option<usize>,
    /// The light on the menu: where it is in the atlas, when it was made and for which rect.
    menu_aura: Option<([f32; 4], Instant, [f32; 4])>,
    menu_glow: Option<Sprite>,
    /// The menu hanging from the bar as last drawn, blended into it.
    merge: Option<Merge>,
    /// How far each dropdown or menu has grown from what opens it.
    growing: HashMap<SharedString, Growing>,
    /// What was chosen on each dropdown when it last stood open.
    chosen: HashMap<SharedString, Option<usize>>,
    /// Rows scrolled that have not added up to a whole one.
    rows_scrolled: f32,
    /// A handler asked that nothing under it hears of the press.
    stopped: bool,
    /// How far each thing that scrolls is pulled past an end.
    overs: HashMap<SharedString, Over>,
    /// Since when each name the pointer is on has been gliding.
    glides: HashMap<u64, Instant>,
    /// What a button going down took, to act on when it comes up.
    pressed: Option<Pressed>,
    /// How tall each slider's handle is.
    grips: HashMap<u64, Spring>,
    /// How far the slider `pulled` is pulled past an end.
    stretch: Spring,
    pulled: Option<u64>,
    /// How far each thing coming into view or going has got.
    rises: HashMap<usize, HashMap<u64, Spring>>,
    /// What each bar widget's text shows.
    widget_texts: HashMap<Widget, HashMap<u64, Shown>>,
    /// What each built panel's text shows.
    texts: HashMap<usize, HashMap<u64, Shown>>,
    /// Lay the panel out again before drawing it.
    stale: bool,
    offsets: HashMap<SharedString, f32>,
    chrome: Vec<chrome::Hit>,
    panel_at: (f32, f32),
    /// Held while a panel is open, so what it shows keeps being read.
    panel_watch: Option<Box<dyn Any>>,
    /// Lay the open panel out again this often, for what moves without new state.
    refresh: Option<Duration>,
    refreshed: Instant,
    /// The slider the pointer holds.
    held: Option<Grip>,
    /// The row last told the pointer came onto it.
    entered: Option<[f32; 4]>,
    stats: SysStats,
    revision: u64,
    sampled_at: Instant,
    watch: Option<Watch>,
    leave_at: Option<Instant>,
    /// The parts that moved last frame, which show where they came to rest once more.
    moving: Vec<&'static str>,
    /// When each countdown last changed.
    countdowns: HashMap<SharedString, Instant>,
    redraw: Cell<Option<Duration>>,
    redraw_at: Option<Instant>,
    menu: Option<Menu>,
    /// A menu on its way from the app.
    awaited: Option<Awaited>,
    /// Since the island started moving, and when it did.
    frames: Option<(u32, Instant)>,
}

impl Island {
    pub fn new(
        stage: Stage,
        layer: LayerSurface,
        output: wayland_client::protocol::wl_output::WlOutput,
        scale: f32,
        look: Look,
    ) -> Self {
        Self {
            stage,
            layer,
            output,
            surface: None,
            blur: None,
            scale,
            fractional: None,
            configured: false,
            turn: Turn::new(),
            dirty: true,
            look,
            cache: HashMap::new(),
            last_frame: None,
            started: Instant::now(),
            pointer: None,
            tip: None,
            placed: Vec::new(),
            body: [0.0; 4],
            region: None,
            inhibitor: None,
            pace: None,
            drew_at: Instant::now(),
            strip: TabStrip::default(),
            built: HashMap::new(),
            eases: HashMap::new(),
            bar_beams: HashMap::new(),
            panel_beams: HashMap::new(),
            beaming: false,
            aura: Aura::default(),
            aura_room: None,
            aura_shown: None,
            menu_aura_room: None,
            menu_aura: None,
            menu_glow: None,
            merge: None,
            growing: HashMap::new(),
            chosen: HashMap::new(),
            rows_scrolled: 0.0,
            stopped: false,
            overs: HashMap::new(),
            glides: HashMap::new(),
            pressed: None,
            grips: HashMap::new(),
            stretch: Spring::new(0.0, SHAPE_SPRING, 0.1),
            pulled: None,
            rises: HashMap::new(),
            widget_texts: HashMap::new(),
            texts: HashMap::new(),
            stale: false,
            offsets: HashMap::new(),
            chrome: Vec::new(),
            panel_at: (0.0, 0.0),
            panel_watch: None,
            refresh: None,
            refreshed: Instant::now(),
            held: None,
            entered: None,
            stats: SysStats::default(),
            revision: 0,
            sampled_at: Instant::now(),
            watch: None,
            leave_at: None,
            moving: Vec::new(),
            countdowns: HashMap::new(),
            redraw: Cell::new(None),
            redraw_at: None,
            menu: None,
            awaited: None,
            frames: None,
        }
    }

    pub fn set_output_name(&mut self, name: &str) {
        self.stage.display = Some(Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes()));
        self.stage.monitor_stale = true;
    }

    fn forget_widgets_of(&mut self, name: &str) {
        let Some(feature) = self.stage.features.get(name) else {
            return;
        };
        for widget in feature.widgets() {
            self.cache.remove(&Widget(widget));
        }
    }

    /// Draws at `scale` from now on.
    pub fn set_scale(&mut self, scale: f32, gpu: Option<&Gpu>) {
        if (scale - self.scale).abs() < f32::EPSILON {
            return;
        }
        self.scale = scale;
        if let (Some(surface), Some(gpu)) = (&self.surface, gpu) {
            let (w, h) = self.buffer_size();
            gpu.configure(surface, w, h);
        }
        self.stale = true;
        self.forget_widgets();
    }

    pub fn buffer_size(&self) -> (u32, u32) {
        (
            (SURFACE_W * self.scale).round() as u32,
            (SURFACE_H * self.scale).round() as u32,
        )
    }

    /// The atlas was emptied: whatever was laid out from it has to be made again.
    pub fn forget_atlas(&mut self) {
        let active = self.stage.active_layer();
        self.built.retain(|layer, _| Some(*layer) == active);
        self.aura_room = None;
        self.aura_shown = None;
        self.menu_aura_room = None;
        self.menu_aura = None;
        self.forget_widgets();
        self.stale = true;
        self.dirty = true;
    }

    pub fn forget_widgets(&mut self) {
        self.cache.clear();
        self.dirty = true;
    }

    pub fn feature_changed(&mut self, name: &'static str) {
        self.forget_widgets_of(name);
        self.stage.sync_local(name);
        self.stage.sync_held();
        self.stale = self.stage.mode != Mode::Collapsed;
        self.dirty = true;
    }

    pub fn feature_live(&mut self, name: &'static str) {
        if self.stage.fullscreen {
            return;
        }
        let live = self
            .stage
            .features
            .get(name)
            .map_or(&[][..], |feature| feature.live_widgets());
        for widget in live {
            self.cache.remove(&Widget(widget));
        }
        self.dirty = true;
    }

    /// Shows the pop-ups once any has something, and takes them away once none has.
    pub fn local_changed(&mut self, name: &'static str) {
        self.forget_widgets_of(name);
        let waiting = self.stage.popup_waiting();
        match self.stage.mode {
            Mode::Collapsed if waiting && self.menu.is_none() => {
                self.set_mode(Mode::Open(Panel::Popups));
            }
            Mode::Open(Panel::Popups) if !waiting => self.set_mode(Mode::Collapsed),
            Mode::Open(_) => self.stale = true,
            _ => {}
        }
        self.dirty = true;
    }

    pub fn set_workspaces(&mut self, snapshot: Snapshot) {
        self.stage.workspaces = Some(snapshot);
        self.stage.monitor_stale = true;
        self.dirty = true;
    }

    fn sync_monitor(&mut self) {
        if !self.stage.monitor_stale {
            return;
        }
        self.stage.monitor_stale = false;
        let monitor = self.stage.monitor().cloned();
        let was_fullscreen = self.stage.fullscreen;
        self.stage.fullscreen = monitor.as_ref().is_some_and(|monitor| monitor.fullscreen);
        if self.stage.fullscreen {
            self.close_menu();
            if !was_fullscreen && self.stage.mode == Mode::Collapsed {
                self.stage.morph.layers[COLLAPSED_LAYER].retarget(0.0, CONTENT_OUT);
            }
        }
        self.stage.sync_locals(monitor.as_ref());
        if self.stage.mode == Mode::Collapsed {
            self.retarget();
        }
        self.last_frame.get_or_insert_with(Instant::now);
        self.cache.clear();
    }

    fn draw_widget(
        &mut self,
        kind: Widget,
        glyphs: &Glyphs,
        painter: &mut Painter,
        gpu: &Gpu,
        now: Instant,
    ) {
        let Widget(name) = kind;
        let Some(feature) = self.stage.features.owning(name) else {
            return;
        };
        let state = self
            .stage
            .held
            .iter()
            .find(|slot| slot.key == name)
            .map_or_else(|| feature.snapshot(), |slot| slot.item.clone());
        let local = self
            .stage
            .locals
            .iter()
            .find(|(owner, _)| *owner == feature.name())
            .map(|(_, local)| &**local);
        let spacing = self.stage.layout.spacing;
        let grouped = self.stage.layout.group.contains(&kind);
        let half = bar::half(spacing);
        let mut w = WidgetCx::new(half, grouped, glyphs, local);
        let widget = feature.widget(name, &*state, &mut w);
        let nothing = HashMap::new();
        let mut layout = Layout {
            painter,
            gpu,
            scale: self.scale,
            hover_inset: spacing / 2.0,
            half,
            grouped,
            started: self.started,
            before: self.widget_texts.get(&kind).unwrap_or(&nothing),
            owner: format!("{}-{}", kind.0, self.output.id().protocol_id()),
        };
        let mut drawn = widget
            .map(|widget| layout.draw(&widget))
            .unwrap_or_default();
        self.widget_texts
            .insert(kind, std::mem::take(&mut drawn.texts));
        let redraw_at = w.next_redraw().map(|after| now + after);
        let redraw_at = if drawn.moving {
            Some(redraw_at.map_or(now + TICK, |at| at.min(now + TICK)))
        } else {
            redraw_at
        };
        self.cache.insert(kind, Cached { drawn, redraw_at });
    }

    /// What each section holds and how wide it is at each widget's fold.
    fn sections(&self, target: bool) -> [Vec<(Widget, f32, f32, f32)>; 3] {
        let layout = &self.stage.layout;
        let spacing = layout.spacing;
        [&layout.left, &layout.middle, &layout.right].map(|widgets| {
            widgets
                .iter()
                .filter_map(|kind| {
                    let Widget(name) = *kind;
                    let rest = self
                        .stage
                        .features
                        .owning(name)
                        .map_or(0.0, |feature| feature.rest_width(name, spacing));
                    let presence = match self.stage.held.iter().find(|slot| slot.key == name) {
                        Some(slot) if target => slot.presence.target(),
                        Some(slot) => slot.presence.value().clamp(0.0, 1.0),
                        None if rest > 0.0 => 0.0,
                        None => return None,
                    };
                    let natural = self
                        .cache
                        .get(kind)
                        .map_or(0.0, |cached| cached.drawn.width);
                    let width = rest + (natural - rest).max(0.0) * presence;
                    Some((*kind, width, presence, rest))
                })
                .collect()
        })
    }

    fn bar_width(&self, target: bool) -> f32 {
        let widths = self
            .sections(target)
            .map(|section| section.iter().map(|(_, width, _, _)| width).sum::<f32>());
        let [left, middle, right] = widths;
        2.0 * bar::edge(&self.stage.layout) + middle + 2.0 * left.max(right)
    }

    fn retarget(&mut self) {
        let shape = self.stage.desired_shape(Some(self.bar_width(true)));
        if self.stage.morph.fit(shape) {
            self.dirty = true;
        }
        let active = self.stage.active_layer();
        self.stage.morph.fade(active);
        match self.stage.mode {
            Mode::Open(kind) if bar::tab_index(kind).is_some() => {
                let at_once = self.stage.morph.chrome.value() < 0.05;
                self.strip.pick(kind, at_once);
                self.stage.morph.show_chrome();
            }
            _ => self.stage.morph.hide_chrome(),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        let mode = self.stage.resolved(mode);
        if self.stage.mode == mode {
            return;
        }
        self.panel_watch = None;
        self.tip = None;
        self.leave_at = None;
        self.held = None;
        self.pressed = None;
        if let Some(previous) = self.stage.closing.take()
            && mode != Mode::Open(Panel::Feature(previous))
        {
            self.stage
                .update_local(previous, |local| local.panel_closed());
        }
        if let Mode::Open(Panel::Feature(name)) = self.stage.mode {
            self.stage.closing = Some(name);
        }
        if let (Mode::Open(from), Mode::Open(to)) = (self.stage.mode, mode)
            && let (Some(from_tab), Some(to_tab)) = (bar::tab_index(from), bar::tab_index(to))
        {
            self.stage
                .morph
                .slide(from.layer(), to.layer(), to_tab > from_tab);
        }
        if mode != Mode::Collapsed {
            self.menu = None;
            self.awaited = None;
        }
        self.stage.mode = mode;
        if let Mode::Open(Panel::Feature(name)) = mode
            && let Some(feature) = self.stage.features.get(name)
        {
            self.panel_watch = feature.panel_opened();
        }
        if mode == Mode::Open(Panel::Dashboard) {
            let watches: Vec<Box<dyn Any>> = features::dashboard_tabs()
                .into_iter()
                .filter_map(|(name, _, _)| self.stage.features.get(name)?.panel_opened())
                .collect();
            self.panel_watch = Some(Box::new(watches));
        }
        let stats = matches!(mode, Mode::Open(panel) if panel.shows_stats());
        self.watch = stats.then(|| self.look.samples.watch());
        self.offsets.clear();
        self.stale = mode != Mode::Collapsed;
        self.refresh = match mode {
            Mode::Open(Panel::Feature(name)) => features::panel_refresh(name),
            Mode::Open(Panel::Popups) => self
                .stage
                .locals
                .iter()
                .filter(|(_, local)| local.popup())
                .filter_map(|(name, _)| features::panel_refresh(name))
                .min(),
            _ => None,
        };
        let shown = mode == Mode::Open(Panel::Popups);
        for (_, local) in &mut self.stage.locals {
            local.popup_shown(shown);
        }
        self.last_frame.get_or_insert_with(Instant::now);
        self.dirty = true;
    }

    pub fn toggle(&mut self, panel: Panel) -> bool {
        let open = self.stage.mode != Mode::Open(panel);
        self.set_mode(if open {
            Mode::Open(panel)
        } else {
            Mode::Collapsed
        });
        open
    }

    fn refresh_panel(&mut self, glyphs: &Glyphs, painter: &mut Painter, gpu: &Gpu) {
        let Mode::Open(panel) = self.stage.mode else {
            return;
        };
        if panel.shows_stats()
            && let Some((stats, at, revision)) = self.look.samples.since(self.revision)
        {
            self.stats = stats;
            self.sampled_at = at;
            self.revision = revision;
            self.stale = true;
        }
        if !self.stale {
            return;
        }
        self.stale = false;
        self.stage.lent.borrow_mut().clear();
        self.refreshed = Instant::now();
        self.redraw.set(None);
        let view = match panel {
            Panel::System => Some(panels::system(
                &self.stats,
                &self.look.system,
                self.stage.displays(),
            )),
            Panel::Dashboard => Some(panels::dashboard(
                self.stage.dashboard(glyphs, &self.redraw),
                &self.stats,
                glyphs,
            )),
            Panel::Feature(name) => self.stage.feature_panel(name, glyphs, &self.redraw),
            Panel::Popups => self.stage.popups(glyphs, &self.redraw),
        };
        self.redraw_at = self.redraw.get().map(|after| Instant::now() + after);
        let view = view.unwrap_or_else(panels::empty);
        let key = view.key();
        let frozen = self.stage.morph.frozen[panel.layer()]
            .filter(|(frozen, _)| *frozen == key)
            .map(|(_, shape)| shape);
        let at_end = self.active_built().is_none_or(|built| {
            built.scrolling.iter().all(|area| {
                area.key.as_ref() != BODY
                    || area.reach - self.offsets.get(&area.key).copied().unwrap_or(0.0)
                        < FOLLOW_SLACK
            })
        });
        if at_end && view.blocks.iter().any(Block::follows_end) {
            self.offsets.insert(BODY.into(), f32::MAX);
        }
        let glass = color(SURFACE).map(|channel| channel * self.look.opacity);
        let nothing = HashMap::new();
        let before = self
            .texts
            .get(&panel.layer())
            .filter(|_| self.built.contains_key(&panel.layer()))
            .unwrap_or(&nothing);
        let mut painting = Painting {
            painter,
            gpu,
            scale: self.scale,
            built: Built::default(),
            offsets: &self.offsets,
            glass,
            holder_rect: [0.0; 4],
            lighting: None,
            before,
            path: 0,
            align: 0.0,
            paints: 0,
        };
        let max = (
            SURFACE_W - 2.0 * SHADOW_SIDE,
            SURFACE_H - self.look.top - SHADOW_BELOW,
        );
        let mut blocks = Blocks {
            painting: &mut painting,
            popovers: Vec::new(),
            prompt: None,
            island_w: self.stage.morph.w.value(),
            sliding: false,
            chosen: std::mem::take(&mut self.chosen),
        };
        let node = match frozen {
            Some(shape) => div()
                .w(shape.w)
                .h(shape.h)
                .child(blocks.panel(view, shape.w, (shape.w, shape.h)).named(BODY)),
            None => blocks.panel(view, 0.0, max).named(BODY),
        };
        let popovers = blocks.popovers;
        let prompt = blocks.prompt;
        let sliding = blocks.sliding;
        self.chosen = blocks.chosen;
        let (width, height) = painting.paint(&node, None, (0.0, 0.0));
        if let Some(prompt) = prompt {
            painting.paint(&records::prompt(prompt, (width, height)), None, (0.0, 0.0));
        }
        painting.built.above = Some(painting.built.sprites.len());
        for popover in popovers {
            let Some(anchor) = painting.built.anchors.get(&popover.anchor).copied() else {
                continue;
            };
            let hits = painting.built.hits.len();
            if popover.open {
                painting.built.hits.push(panel::Hit {
                    rect: [0.0, 0.0, width, height],
                    on_click: Some(popover.on_toggle),
                    immediate: true,
                    blocks: true,
                    ..panel::Hit::default()
                });
            }
            let list = if popover.drops {
                popover.list
            } else {
                popover.list.min_w(anchor[2])
            };
            let (list_w, list_h) = painting.measure(&list, None);
            let (wanted, top) = if popover.drops {
                (anchor[0] + anchor[2] - list_w, anchor[1] + anchor[3] + DROP)
            } else {
                (anchor[0] + (anchor[2] - list_w) / 2.0, anchor[1])
            };
            let left = wanted
                .clamp(
                    POPOVER_MARGIN,
                    (width - list_w - POPOVER_MARGIN).max(POPOVER_MARGIN),
                )
                .round();
            let from = painting.built.sprites.len();
            let row_hits = painting.built.hits.len();
            painting.paint(&list, None, (left, top.round()));
            let rows = (from, painting.built.sprites.len());
            let rect = [left, top.round(), list_w, list_h];
            let row_hits = if popover.open {
                painting.built.reaches.push(rect);
                (row_hits, painting.built.hits.len())
            } else {
                painting.built.hits.truncate(hits);
                (0, 0)
            };
            let strip = popover.label.as_ref().is_some_and(|(_, strip)| *strip);
            if let Some((words, strip)) = popover.label {
                let label = chip_label(words, strip, (anchor[2], anchor[3]));
                let hits = painting.built.hits.len();
                painting.paint(&label, None, (anchor[0], anchor[1]));
                painting.built.hits.truncate(hits);
            }
            let (rest_fill, rest_border, rest_radius) = chip_look(strip);
            let key = popover.anchor;
            painting.built.surfaces.push(panel::Surface {
                key: key.clone(),
                open: popover.open,
                drops: popover.drops,
                anchor,
                rect,
                rest_fill,
                rest_border,
                rest_radius,
                rows,
                label: (rows.1, painting.built.sprites.len()),
                chip: painting.built.anchored.get(&key).copied(),
                hits: row_hits,
                live: false,
            });
        }
        let mut built = painting.built;
        for area in &built.scrolling {
            if let Some(offset) = self.offsets.get_mut(&area.key) {
                *offset = offset.min(area.reach);
            }
        }
        self.stale = sliding;
        built.width = width.ceil();
        built.height = height.ceil();
        let now = Instant::now();
        for countdown in &built.countdowns {
            self.countdowns.entry(countdown.key.clone()).or_insert(now);
        }
        let shape = Shape {
            w: built.width,
            h: built.height,
        };
        self.stage.morph.frozen[panel.layer()] = Some((key, shape));
        self.texts
            .insert(panel.layer(), std::mem::take(&mut built.texts));
        for surface in &mut built.surfaces {
            let growing = self
                .growing
                .entry(surface.key.clone())
                .or_insert_with(Growing::new);
            let [left, _, width, _] = surface.anchor;
            if growing.open || growing.grown.value().abs() > 0.01 {
                growing.left.retarget(left, SHAPE_SPRING);
                growing.width.retarget(width, SHAPE_SPRING);
            } else {
                growing.left.reset(left);
                growing.width.reset(width);
            }
            if growing.open != surface.open {
                growing.open = surface.open;
                let (to, config) = if surface.open {
                    (1.0, CONTENT_IN)
                } else {
                    (0.0, CONTENT_OUT)
                };
                growing.grown.retarget(to, SHAPE_SPRING);
                growing.revealed.retarget(to, config);
            }
            surface.live = growing.is_live();
        }
        let risen = self.rises.entry(panel.layer()).or_default();
        risen.retain(|key, _| built.rises.iter().any(|rise| rise.key == *key));
        for rise in &built.rises {
            risen.entry(rise.key).or_insert_with(|| {
                let (from, to, config) = if rise.leaving {
                    (1.0, 0.0, CONTENT_OUT)
                } else {
                    (0.0, 1.0, CONTENT_IN)
                };
                let mut spring = Spring::new(from, config, FILL_EPSILON);
                spring.retarget(to, config);
                spring
            });
        }
        let shown = self.built.contains_key(&panel.layer());
        let held = self.held.as_ref().map(|grip| grip.rect);
        let springs = self.eases.entry(panel.layer()).or_default();
        springs.retain(|key, _| built.eases.iter().any(|ease| ease.key == *key));
        for ease in &built.eases {
            match springs.get_mut(&ease.key) {
                Some(spring) if shown && (held.is_none() || ease.follows != held) => {
                    spring.retarget(ease.value, ease.config);
                }
                Some(spring) => spring.reset(ease.value),
                None => {
                    springs.insert(ease.key, Spring::new(ease.value, ease.config, FILL_EPSILON));
                }
            }
        }
        self.built.insert(panel.layer(), built);
    }

    /// Also shows any pop-up that waited for the menu.
    pub fn close_menu(&mut self) {
        self.awaited = None;
        let Some(menu) = self.menu.as_mut().filter(|menu| menu.is_open()) else {
            return;
        };
        menu.close();
        self.last_frame.get_or_insert_with(Instant::now);
        self.dirty = true;
        if self.stage.popup_waiting() {
            self.set_mode(Mode::Collapsed);
        }
    }

    pub fn poll_menu(&mut self, cx: &mut Context<'_>) {
        let Some(awaited) = &mut self.awaited else {
            return;
        };
        let Poll::Ready(root) = Pin::new(&mut awaited.response).poll(cx) else {
            return;
        };
        if let Some(awaited) = self.awaited.take()
            && let Ok(Some(root)) = root
            && !root.children.is_empty()
            && self.stage.mode == Mode::Collapsed
            && !self.stage.fullscreen
        {
            match &mut self.menu {
                Some(menu) => menu.replace(awaited.source, root, awaited.pick, awaited.anchor),
                None => {
                    self.menu = Some(Menu::new(
                        awaited.source,
                        root,
                        awaited.pick,
                        awaited.anchor,
                    ));
                }
            }
            self.last_frame.get_or_insert_with(Instant::now);
            self.dirty = true;
        }
    }

    fn measure_locals(&mut self, painter: &mut Painter) {
        let painter = std::cell::RefCell::new(painter);
        let lines = |text: &str, size: f32, width: f32| {
            painter
                .borrow_mut()
                .wrapped_count(crate::theme::FONT, text, size, width)
        };
        let changed: Vec<&'static str> = self
            .stage
            .locals
            .iter_mut()
            .filter_map(|(name, local)| local.measure(&Measure::new(&lines)).then_some(*name))
            .collect();
        for name in changed {
            self.local_changed(name);
        }
    }

    fn still(&self, local_still: impl Fn(&dyn AnyLocal) -> bool) -> bool {
        self.stage.still(local_still)
            && self.strip.is_settled()
            && self.menu.as_ref().is_none_or(Menu::is_settled)
            && self
                .eases
                .values()
                .flat_map(HashMap::values)
                .all(Spring::is_settled)
            && self.grips.values().all(Spring::is_settled)
            && self
                .overs
                .values()
                .all(|over| over.pull == 0.0 && over.stretch.is_settled())
            && self.glides.is_empty()
            && self.growing.values().all(Growing::is_settled)
            && self.stretch.is_settled()
            && self
                .rises
                .values()
                .flat_map(HashMap::values)
                .all(Spring::is_settled)
            && !self
                .built
                .values()
                .any(|built| built.morphs.iter().any(MorphText::is_running))
            && !self
                .cache
                .values()
                .any(|cached| cached.drawn.morphs.iter().any(MorphText::is_running))
    }

    fn feature_pace(&self) -> Option<Duration> {
        if self.still(|local| local.is_settled() || local.is_drifting()) {
            Some(TICK)
        } else {
            self.still(|local| local.is_settled() || local.is_drifting() || local.is_brisk())
                .then_some(BRISK)
        }
    }

    /// Redraws the widgets of features still moving, whose locals only they know the look of.
    fn forget_unsettled(&mut self) {
        let moving: Vec<&'static str> = self
            .stage
            .locals
            .iter()
            .filter(|(_, local)| !local.is_settled())
            .map(|(name, _)| *name)
            .collect();
        let settling = std::mem::replace(&mut self.moving, moving.clone());
        let names = moving.into_iter().chain(settling);
        for name in names {
            if self.stage.shows_local(name) {
                self.stale = true;
            }
            self.forget_widgets_of(name);
        }
    }

    pub fn next_wake(&self) -> Option<Instant> {
        let hidden = self.stage.hidden();
        let widgets = self
            .cache
            .values()
            .filter(|_| !hidden)
            .filter_map(|cached| cached.redraw_at);
        let tip = self
            .tip
            .as_ref()
            .filter(|tip| tip.since.elapsed() < TIP_DELAY)
            .map(|tip| tip.since + TIP_DELAY);
        let waiting = !self.turn.ready();
        let pace = self
            .pace
            .filter(|_| !waiting)
            .map(|pace| self.drew_at + pace);
        let refresh = self
            .refresh
            .filter(|_| !waiting)
            .map(|every| self.refreshed + every);
        widgets
            .chain(tip)
            .chain(pace)
            .chain(refresh)
            .chain(self.leave_at)
            .chain(self.redraw_at)
            .chain(self.turn.again())
            .chain(self.stage.next_due())
            .min()
    }

    /// Whatever has come due: a feature's time, or a widget that moves.
    pub fn tick(&mut self, now: Instant) {
        if self.turn.elapse(now) {
            self.dirty = true;
        }
        if self.leave_at.is_some_and(|at| at <= now) {
            self.leave_at = None;
            self.set_mode(Mode::Collapsed);
        }
        if self.redraw_at.is_some_and(|at| at <= now) {
            self.redraw_at = None;
            self.stale = true;
            self.dirty = true;
        }
        for name in self.stage.elapse(now) {
            self.local_changed(name);
        }
        let hidden = self.stage.hidden();
        let due: Vec<Widget> = self
            .cache
            .iter()
            .filter(|_| !hidden)
            .filter(|(_, cached)| cached.redraw_at.is_some_and(|at| at <= now))
            .map(|(kind, _)| *kind)
            .collect();
        for kind in due {
            self.cache.remove(&kind);
            self.dirty = true;
        }
        if self.pace.is_some_and(|pace| self.drew_at + pace <= now) {
            self.dirty = true;
        }
        if self
            .refresh
            .is_some_and(|every| self.refreshed + every <= now)
        {
            self.stale = true;
            self.dirty = true;
        }
        if self
            .tip
            .as_ref()
            .is_some_and(|tip| tip.since + TIP_DELAY <= now)
        {
            self.dirty = true;
        }
    }

    fn panel_sprites(&mut self, clip: [f32; 4], out: &mut Vec<Sprite>) {
        let scale = self.scale;
        let collapsed = self.stage.mode == Mode::Collapsed;
        let active = self.stage.active_layer();
        let phase = self.sampled_at.elapsed().as_secs_f32() / sys::INTERVAL.as_secs_f32();
        let width = self.stage.morph.slide_width;
        for (layer, built) in &self.built {
            let layer = *layer;
            let faded = self.stage.morph.layers[layer].value().clamp(0.0, 1.0);
            let slide = &self.stage.morph.slides[layer];
            let opacity = if slide.is_settled() || collapsed {
                faded
            } else {
                1.0
            };
            if opacity <= 0.001 {
                continue;
            }
            let across = (slide.value() * width).round();
            let x = ((SURFACE_W - built.width) / 2.0 * scale).round() / scale + across;
            let y = self.look.top - (1.0 - opacity) * CONTENT_DRIFT;
            if Some(layer) == active {
                self.panel_at = (x, y);
            }
            let first = out.len();
            let above = built.above.unwrap_or(built.sprites.len());
            out.extend(built.sprites.iter().copied());
            let own = &mut out[first..];
            let eases = self.eases.get(&layer);
            if let Some(springs) = eases {
                for ease in &built.eases {
                    let Some(spring) = springs.get(&ease.key) else {
                        continue;
                    };
                    panel::ease_sprites(ease, spring.value(), &mut own[ease.from..ease.to]);
                }
            }
            let local = self
                .pointer
                .filter(|_| Some(layer) == active)
                .map(|pointer| (pointer.0 - x, pointer.1 - y));
            let hovered = local.and_then(|local| built.answering(local, |hit| hit.hover.is_some()));
            if let Some(hit) = hovered {
                if let Some((tint, sprites)) = &hit.lit {
                    for &(from, to) in sprites.iter().filter(|(_, to)| *to <= above) {
                        for sprite in &mut own[from..to] {
                            sprite.color = *tint;
                        }
                    }
                }
                if let Some(fill) = hit.hover {
                    own[fill].color = tinted(ACCENT, HOVER_ALPHA);
                }
            }
            for surface in built.surfaces.iter().filter(|surface| !surface.drops) {
                let out_now = self
                    .growing
                    .get(&surface.key)
                    .is_some_and(|growing| growing.open || growing.grown.value().abs() > 0.01);
                if let Some((from, to)) = surface.chip.filter(|_| out_now) {
                    for sprite in &mut own[from..to] {
                        sprite.color = [0.0; 4];
                    }
                }
            }
            for area in &built.scrolling {
                let stretch = self.stretch_of(&area.key);
                if stretch != 0.0 {
                    for sprite in &mut own[area.from..area.to] {
                        sprite.rect[1] += stretch;
                    }
                }
            }
            if let Some(held) = &self.pressed
                && held.panel
                && Some(layer) == active
                && held.sprites.1 <= above
            {
                for sprite in &mut own[held.sprites.0..held.sprites.1] {
                    *sprite = sprite.faded(HELD_DIM);
                }
            }
            for reveal in built.reveals.iter().filter(|reveal| reveal.to <= above) {
                let shown = built
                    .anchors
                    .get(&reveal.group)
                    .is_some_and(|rect| local.is_some_and(|local| inside(*rect, local)));
                if !shown {
                    for sprite in &mut own[reveal.from..reveal.to] {
                        *sprite = sprite.faded(0.0);
                    }
                }
            }
            let shown =
                pulsed((self.started.elapsed().as_secs_f32() / PULSE.as_secs_f32()).fract());
            for &(from, to) in built.pulsing.iter().filter(|(_, to)| *to <= above) {
                for sprite in &mut own[from..to] {
                    *sprite = sprite.faded(shown);
                }
            }
            for track in &built.sliding {
                if self.pulled == Some(track.key) {
                    panel::pulled_sprites(
                        track.rect,
                        self.stretch.value(),
                        &mut own[track.from..track.to],
                    );
                }
            }
            if let Some(risen) = self.rises.get(&layer) {
                for rise in built.rises.iter().filter(|rise| rise.to <= above) {
                    let Some(spring) = risen.get(&rise.key) else {
                        continue;
                    };
                    let shown = spring.value().clamp(0.0, 1.0);
                    for sprite in &mut own[rise.from..rise.to] {
                        *sprite = sprite
                            .moved(0.0, (1.0 - shown) * CONTENT_DRIFT)
                            .faded(shown);
                    }
                }
            }
            let over = out.split_off(first + above);
            for chart in &built.charts {
                let phase = chart.phase.map_or(phase, Phase::now);
                panel::chart_sprites(chart, phase, 1.0 / scale, out);
            }
            for countdown in &built.countdowns {
                let gone = self
                    .countdowns
                    .get(&countdown.key)
                    .filter(|_| !countdown.running.is_zero())
                    .map_or(0.0, |since| {
                        since.elapsed().as_secs_f32() / countdown.running.as_secs_f32()
                    });
                panel::countdown_sprites(countdown, gone, out);
            }
            let seconds = self.started.elapsed().as_secs_f32();
            for marquee in &built.marquees {
                match marquee.hover {
                    Some(_) => {
                        let glided = self
                            .glides
                            .get(&marquee.key)
                            .map_or(0.0, |since| since.elapsed().as_secs_f32());
                        panel::marquee_sprites(marquee, glided, panel::glided(glided), out);
                    }
                    None => panel::marquee_sprites(marquee, seconds, 1.0, out),
                }
            }
            for (rect, clip) in &built.carets {
                out.push(panel::caret_sprite(*rect, *clip, seconds));
            }
            for morph in &built.morphs {
                panel::morph_sprites(morph, out);
            }
            for pulse in &built.pulses {
                out.push(panel::pulse_sprite(pulse, seconds));
            }
            for (key, busy, rect) in &built.beams {
                let shone = self.panel_beams.entry(*key).or_default().shine(*busy);
                if let Some((phase, strength)) = shone {
                    self.beaming = true;
                    panel::beam_sprites(*rect, phase, strength, out);
                }
            }
            for track in &built.sliding {
                let (Some(grip), Some((fill, _))) = (self.grips.get(&track.key), track.fill) else {
                    continue;
                };
                let tall = grip.value().max(0.0);
                if tall <= 0.5 {
                    continue;
                }
                let level = eases
                    .and_then(|springs| springs.get(&fill))
                    .map_or(0.0, |spring| spring.value().clamp(0.0, 1.0));
                let wide = HANDLE_W + ((tall - HOVER_H) / (HELD_H - HOVER_H)).clamp(0.0, 1.0);
                let pull = if self.pulled == Some(track.key) {
                    self.stretch.value()
                } else {
                    0.0
                };
                let x = track.rect[0] + pull.min(0.0) + (track.rect[2] + pull.abs()) * level
                    - wide / 2.0;
                let y = track.rect[1] + (track.rect[3] - tall) / 2.0;
                out.push(Sprite::shadow(
                    [x - 3.0, y - 2.0, wide + 6.0, tall + 6.0],
                    [0.0, 0.0, 0.0, 0.25],
                    wide / 2.0,
                    3.0,
                    1.0,
                ));
                out.push(Sprite::shape(
                    [x, y, wide, tall],
                    color(TEXT),
                    wide / 2.0,
                    2.0,
                ));
            }
            let inner = out.len();
            for area in &built.scrolling {
                let stretch = self.stretch_of(&area.key);
                panel::pulled_along(&mut out[first + above..inner], area.rect, stretch);
            }
            for surface in &built.surfaces {
                if let Some(growing) = self.growing.get(&surface.key) {
                    panel::surface_sprites(
                        surface,
                        &over,
                        above,
                        (growing.left.value(), growing.width.value()),
                        (growing.grown.value(), growing.revealed.value()),
                        out,
                    );
                }
            }
            for sprite in &mut out[first..inner] {
                *sprite = sprite.moved(x, y).faded(opacity).within(clip);
            }
            for sprite in &mut out[inner..] {
                *sprite = sprite.moved(x, y).faded(opacity);
            }
        }
    }

    /// How far what scrolls in `key` is pulled past its end.
    fn stretch_of(&self, key: &SharedString) -> f32 {
        self.overs.get(key).map_or(0.0, |over| over.stretch.value())
    }

    fn chrome_sprites(
        &mut self,
        glyphs: &Glyphs,
        painter: &mut Painter,
        gpu: &Gpu,
        clip: [f32; 4],
        out: &mut Vec<Sprite>,
    ) {
        let opacity = self.stage.morph.chrome.value().clamp(0.0, 1.0);
        self.chrome.clear();
        if opacity <= 0.001 {
            return;
        }
        let scale = self.scale;
        let x = ((SURFACE_W - self.strip.width()) / 2.0 * scale).round() / scale;
        let y = self.look.top + INSET - (1.0 - opacity) * CONTENT_DRIFT;
        let right = self.body[0] + self.body[2] - INSET;
        let mut chrome = Chrome {
            strip: &self.strip,
            glyphs,
            painter,
            gpu,
            scale,
            pointer: self.pointer,
        };
        let (sprites, hits) = chrome.draw(x, y, right);
        out.extend(
            sprites
                .iter()
                .map(|sprite| sprite.within(clip).faded(opacity)),
        );
        self.chrome = hits;
    }

    fn compose(&mut self, glyphs: &Glyphs, painter: &mut Painter, gpu: &Gpu) -> Vec<Sprite> {
        let viewport = SURFACE_W;
        let w = self.stage.morph.w.value();
        let h = self.stage.morph.h.value();
        let top = self.look.top;
        let scale = self.scale;
        let x = ((viewport - w) / 2.0 * scale).round() / scale;
        self.body = [x, top, w, h];
        let (radius, power) = self.look.corner;
        let radius = radius.min(h / 2.0);
        let mut sprites = Vec::with_capacity(2048);
        self.beaming = false;
        if w < 1.0 || h < 1.0 {
            self.placed.clear();
            return sprites;
        }

        self.place_menu(painter);
        let body = self.body;
        let strength = self.look.shadow;
        let merge = self.merge(radius);
        self.merge = merge;
        let corner = (radius, power);
        if strength > 0.0 {
            let shade = [0.0, 0.0, 0.0, 0.35 * strength];
            let line = [0.0, 0.0, 0.0, 0.3 * strength];
            match &merge {
                Some(merge) => {
                    sprites.push(Sprite::merged_shadow(
                        body,
                        shade,
                        corner,
                        merge,
                        (15.0, 10.0),
                    ));
                    sprites.push(Sprite::merged(body, line, corner, merge, MERGED_OUTLINE));
                }
                None => {
                    sprites.push(Sprite::drop_shadow(body, shade, radius, 15.0, 10.0));
                    sprites.push(Sprite::ring(
                        [body[0] - 1.0, body[1] - 1.0, body[2] + 2.0, body[3] + 2.0],
                        line,
                        radius + 1.0,
                        power,
                    ));
                }
            }
        }
        let glass = color(SURFACE).map(|channel| channel * self.look.opacity);
        let edge = tinted(SURFACE1, SURFACE_BORDER_ALPHA);
        match &merge {
            Some(merge) => {
                sprites.push(Sprite::merged(body, glass, corner, merge, MERGED_FILL));
                sprites.push(Sprite::merged(body, edge, corner, merge, MERGED_RIM));
            }
            None => {
                sprites.push(Sprite::shape(body, glass, radius, power));
                sprites.push(Sprite::ring(body, edge, radius, power));
            }
        }

        let fade = self.stage.morph.layers[COLLAPSED_LAYER]
            .value()
            .clamp(0.0, 1.0);
        if self.look.glow > 0.0 && fade > 0.001 && self.aura.is_lit() && !self.stage.hidden() {
            let due = self
                .aura_shown
                .is_none_or(|(_, made)| made.elapsed() >= AURA_EVERY);
            if due {
                if self.aura_room.is_none() {
                    self.aura_room = painter.glow_room();
                }
                let light = self.aura.pixels(body[2], body[3], self.look.glow);
                self.aura_shown = self
                    .aura_room
                    .and_then(|room| painter.glow(gpu, room, light))
                    .map(|uv| (uv, Instant::now()));
            }
            if let Some((uv, _)) = self.aura_shown {
                sprites.push(Sprite::soft(body, uv, [fade; 4], radius, power));
            }
        }
        self.menu_glow = self.menu_light(painter, gpu, fade);
        self.placed.clear();
        let clip = [body[0], body[1], body[0] + body[2], body[1] + body[3]];
        if fade > 0.001 {
            let sections = self.sections(false);
            let widths = sections
                .each_ref()
                .map(|section| section.iter().map(|(_, width, _, _)| width).sum::<f32>());
            let side = widths[0].max(widths[2]);
            let edge = bar::edge(&self.stage.layout);
            let content = self.bar_width(false);
            let origin = ((viewport - content) / 2.0 * scale).round() / scale;
            let starts = [
                origin + edge,
                origin + edge + side,
                origin + edge + side + widths[1] + (side - widths[2]),
            ];
            let row = top + ((h - COLLAPSED_H) / 2.0).max(0.0) + (COLLAPSED_H - WIDGET_H) / 2.0;
            let collapsed = self.stage.mode == Mode::Collapsed;
            for (section, start) in sections.iter().zip(starts) {
                sprites.extend(self.set_tint(section, start, row));
                let mut at = start;
                for (kind, width, presence, rest) in section {
                    let Some(cached) = self.cache.get(kind) else {
                        continue;
                    };
                    let fold = [at, row, at + width, row + WIDGET_H];
                    let opacity = fade
                        * if *rest <= 0.0 {
                            smoothstep(*presence)
                        } else {
                            1.0
                        };
                    for (index, hit) in cached.drawn.hits.iter().enumerate() {
                        if !collapsed {
                            break;
                        }
                        self.placed.push(Placed {
                            widget: *kind,
                            index,
                            x: at + hit.rect[0],
                            y: row,
                            w: hit.rect[2],
                        });
                    }
                    let tint = self.hover_tint(*kind, at, row);
                    sprites.extend(tint);
                    sprites.extend(cached.drawn.sprites.iter().map(|sprite| {
                        sprite
                            .moved(at, row)
                            .within(fold)
                            .within(clip)
                            .faded(opacity)
                    }));
                    let turning = sprites.len();
                    for morph in &cached.drawn.morphs {
                        panel::morph_sprites(morph, &mut sprites);
                    }
                    if let Some((busy, rect)) = cached.drawn.busy {
                        let shone = self.bar_beams.entry(*kind).or_default().shine(busy);
                        if let Some((phase, strength)) = shone {
                            self.beaming = true;
                            panel::beam_sprites(rect, phase, strength, &mut sprites);
                        }
                    }
                    for sprite in &mut sprites[turning..] {
                        *sprite = sprite
                            .moved(at, row)
                            .within(fold)
                            .within(clip)
                            .faded(opacity);
                    }
                    at += width;
                }
            }
        }
        self.panel_sprites(clip, &mut sprites);
        self.chrome_sprites(glyphs, painter, gpu, clip, &mut sprites);
        self.menu_sprites(painter, gpu, &mut sprites);
        self.tip_sprites(painter, gpu, &mut sprites);
        sprites
    }

    /// The bar's light running on down into the menu hanging from it.
    fn menu_light(&mut self, painter: &mut Painter, gpu: &Gpu, fade: f32) -> Option<Sprite> {
        let lit = self.look.glow > 0.0 && fade > 0.001 && self.aura.is_lit();
        let [x, bottom, w, h] = self.menu.as_ref().map(|menu| menu.rect)?;
        if !lit || self.stage.hidden() || h < 0.5 || w < 1.0 {
            self.menu_aura = None;
            return None;
        }
        let body = self.body;
        let corner = menu::corner(w, h);
        let rect = [x, bottom - corner, w, h + corner];
        let due = self
            .menu_aura
            .is_none_or(|(_, made, at)| made.elapsed() >= AURA_EVERY || at != rect);
        if due {
            if self.menu_aura_room.is_none() {
                self.menu_aura_room = painter.glow_room();
            }
            let part = AuraPart {
                field: (body[2], body[3]),
                offset: (rect[0] - body[0], rect[1] - body[1]),
            };
            let light = self.aura.pixels_of(rect[2], rect[3], self.look.glow, part);
            self.menu_aura = self
                .menu_aura_room
                .and_then(|room| painter.glow(gpu, room, light))
                .map(|uv| (uv, Instant::now(), rect));
        }
        let (uv, _, _) = self.menu_aura?;
        let (far, wide) = (-1.0e5, 1.0e5);
        Some(Sprite::soft(rect, uv, [fade; 4], corner, 2.0).within([far, bottom, wide, wide]))
    }

    /// The menu hanging from the bar this frame, and how it blends into it: never so softly that the
    /// blend reaches the bar's rounded ends.
    fn merge(&self, radius: f32) -> Option<Merge> {
        let menu = self.menu.as_ref()?;
        let hang = menu.rect;
        if hang[3] < 0.5 || hang[2] < 1.0 {
            return None;
        }
        let (rounding, blend) = menu.blend();
        let body = self.body;
        let room = (hang[0] - body[0] - radius).min(body[0] + body[2] - radius - hang[0] - hang[2]);
        Some(Merge {
            hang,
            radius: rounding,
            blend: blend.min((2.0 * room).max(menu::SOFT_OPEN)),
        })
    }

    /// Where the menu hangs this frame, so what is cut round it fits it.
    fn place_menu(&mut self, painter: &mut Painter) {
        let body = self.body;
        let radius = self.look.corner.0.min(body[3] / 2.0);
        let scale = self.scale;
        let rest_w = self.bar_width(true).min(body[2]);
        let rest_x = ((SURFACE_W - rest_w) / 2.0 * scale).round() / scale;
        if let Some(menu) = &mut self.menu {
            let edges = (
                rest_x + radius + crate::menu::FILLET,
                rest_x + rest_w - radius - crate::menu::FILLET,
                body[1] + body[3],
            );
            menu.place(edges, painter, scale);
        }
    }

    fn menu_sprites(&mut self, painter: &mut Painter, gpu: &Gpu, out: &mut Vec<Sprite>) {
        let (pointer, scale) = (self.pointer, self.scale);
        if let Some(menu) = &mut self.menu {
            menu.draw(pointer, self.menu_glow, painter, gpu, scale, out);
        }
    }

    /// The hover on a set of widgets side by side, which show it together.
    fn set_tint(
        &self,
        section: &[(Widget, f32, f32, f32)],
        start: f32,
        row: f32,
    ) -> Option<Sprite> {
        let pointer = self.pointer?;
        let group = &self.stage.layout.group;
        let widths = section
            .iter()
            .map(|(kind, width, ..)| (group.contains(kind), *width));
        let (x, w) = sets(widths, start)
            .into_iter()
            .find(|(x, w)| inside([*x, row, *w, WIDGET_H], pointer))?;
        let inset = self.stage.layout.spacing / 2.0;
        Some(Sprite::shape(
            [x + inset, row, w - 2.0 * inset, WIDGET_H],
            tinted(ACCENT, HOVER_ALPHA),
            CONTROL_RADIUS,
            2.0,
        ))
    }

    fn hover_tint(&self, kind: Widget, at: f32, row: f32) -> Option<Sprite> {
        let pointer = self.pointer?;
        let cached = self.cache.get(&kind)?;
        let hit = cached.drawn.hits.iter().rev().find(|hit| {
            inside([at + hit.rect[0], row, hit.rect[2], hit.rect[3]], pointer)
                && !matches!(hit.tint, Tint::None)
        })?;
        let hover = tinted(ACCENT, HOVER_ALPHA);
        match hit.tint {
            Tint::None => None,
            Tint::Control(inset) => Some(Sprite::shape(
                [
                    at + hit.rect[0] + inset,
                    row,
                    hit.rect[2] - 2.0 * inset,
                    WIDGET_H,
                ],
                hover,
                CONTROL_RADIUS,
                2.0,
            )),
            Tint::Pill(height) => Some(Sprite::shape(
                [
                    at + hit.rect[0],
                    row + (WIDGET_H - height) / 2.0,
                    hit.rect[2],
                    height,
                ],
                hover,
                height,
                2.0,
            )),
        }
    }

    fn tip_sprites(&self, painter: &mut Painter, gpu: &Gpu, out: &mut Vec<Sprite>) {
        let Some(tip) = &self.tip else {
            return;
        };
        if tip.since.elapsed() < TIP_DELAY || tip.text.is_empty() {
            return;
        }
        let scale = self.scale;
        let lines: Vec<_> = tip
            .text
            .lines()
            .map(|text| painter.line(text, SMALL_SIZE, 500, scale))
            .collect();
        let row = (SMALL_SIZE * TIP_LEADING).round();
        let widest = lines.iter().map(|line| line.width).fold(0.0, f32::max);
        let (w, h) = (
            widest + 2.0 * TIP_PAD.0,
            lines.len() as f32 * row + 2.0 * TIP_PAD.1,
        );
        let x = (tip.at.0 + TIP_GAP.0).min(SURFACE_W - w - 4.0).max(4.0);
        let y = tip.at.1 + TIP_GAP.1;
        let rect = [x.round(), y.round(), w, h];
        out.push(Sprite::shape(
            rect,
            color(SURFACE).map(|channel| channel * 0.97),
            TIP_RADIUS,
            2.0,
        ));
        out.push(Sprite::ring(
            rect,
            tinted(SURFACE1, SURFACE_BORDER_ALPHA),
            TIP_RADIUS,
            2.0,
        ));
        for (index, line) in lines.iter().enumerate() {
            let top = rect[1] + TIP_PAD.1 + index as f32 * row + (row - SMALL_SIZE * 1.2) / 2.0;
            painter.text(
                gpu,
                line,
                (rect[0] + TIP_PAD.0, top),
                color(TEXT),
                scale,
                out,
            );
        }
    }

    /// Something needs drawing, and the compositor is ready for it.
    pub fn wants_frame(&self) -> bool {
        self.dirty && self.configured && self.turn.ready() && self.surface.is_some()
    }

    pub fn frame(
        &mut self,
        glyphs: &Glyphs,
        painter: &mut Painter,
        gpu: &Gpu,
        compositor: &CompositorState,
    ) {
        self.dirty = false;
        self.sync_monitor();
        let now = Instant::now();
        let dt = self
            .last_frame
            .map_or(Duration::ZERO, |last| (now - last).min(MAX_STEP))
            .as_secs_f32();
        self.stage.advance(dt, now);
        if self.look.glow > 0.0 {
            let levels = self
                .stage
                .features
                .iter()
                .find_map(|feature| feature.glow());
            self.aura
                .update(levels.as_deref().unwrap_or_default(), levels.is_some());
        }
        self.aim_grips();
        self.aim_glides();
        for over in self.overs.values_mut() {
            if over.pull != 0.0 {
                if over.last.is_some_and(|last| now - last < LET_GO) {
                    over.stretch.reset(rubber(over.pull, REACH));
                } else {
                    over.pull = 0.0;
                    over.stretch.retarget(0.0, SHAPE_SPRING);
                }
            }
            over.stretch.advance(dt, now);
        }
        let risen = self.rises.values_mut().flat_map(HashMap::values_mut);
        let eased = self.eases.values_mut().flat_map(HashMap::values_mut);
        let grown = self.growing.values_mut().flat_map(|growing| {
            [
                &mut growing.grown,
                &mut growing.revealed,
                &mut growing.left,
                &mut growing.width,
            ]
        });
        for spring in eased
            .chain(risen)
            .chain(self.grips.values_mut())
            .chain(grown)
        {
            spring.advance(dt, now);
        }
        for surface in self
            .built
            .values_mut()
            .flat_map(|built| &mut built.surfaces)
        {
            surface.live = self.growing.get(&surface.key).is_some_and(Growing::is_live);
        }
        self.retip();
        self.stretch.advance(dt, now);
        self.strip.advance(dt, now);
        if let Some(menu) = &mut self.menu {
            menu.advance(dt, now);
        }
        if self.menu.as_ref().is_some_and(Menu::is_gone) {
            self.menu = None;
        }
        self.measure_locals(painter);
        self.forget_unsettled();
        self.refresh_panel(glyphs, painter, gpu);
        let wanted: Vec<Widget> = self
            .stage
            .layout
            .widgets()
            .filter(|kind| !self.cache.contains_key(kind))
            .collect();
        for kind in wanted {
            self.draw_widget(kind, glyphs, painter, gpu, now);
        }
        self.retarget();
        if self.stage.mode == Mode::Collapsed
            && !self.stage.fullscreen
            && !self.stage.held.is_settled()
        {
            let width = collapsed_width(self.bar_width(false), self.stage.hovered);
            self.stage.morph.w.reset(width);
        }
        let sprites = self.compose(glyphs, painter, gpu);
        self.sync_regions(compositor);
        self.show(&sprites, gpu, now);
        self.drew_at = now;
        let open = self.stage.mode != Mode::Collapsed;
        let morph = &self.stage.morph;
        let active = self.stage.active_layer();
        self.built.retain(|layer, _| {
            Some(*layer) == active
                || morph.layers[*layer].value() > 0.001
                || !morph.slides[*layer].is_settled()
        });
        let built = &self.built;
        self.eases.retain(|layer, _| built.contains_key(layer));
        self.growing.retain(|key, growing| {
            !growing.is_settled()
                || built
                    .values()
                    .any(|built| built.surfaces.iter().any(|surface| surface.key == *key))
        });
        self.rises.retain(|layer, _| built.contains_key(layer));
        self.texts.retain(|layer, _| built.contains_key(layer));
        let settled = self.still(|local| local.is_settled());
        self.pace = None;
        if settled {
            self.stage.finish_closing();
            if let Some((frames, started)) = self.frames.take() {
                debug!(
                    target: "island::frames",
                    mode = ?self.stage.mode,
                    frames,
                    took = ?started.elapsed(),
                    "animation settled"
                );
            }
            self.last_frame = None;
            // Charts slide with the clock, and countdowns run out.
            let moving = self
                .built
                .values()
                .any(|built| !built.charts.is_empty() || !built.countdowns.is_empty());
            let gliding = self.built.values().any(|built| {
                built.marquees.iter().any(|marquee| marquee.hover.is_none())
                    || !built.carets.is_empty()
                    || !built.pulses.is_empty()
                    || !built.pulsing.is_empty()
            });
            self.pace = open
                .then(|| {
                    if gliding {
                        Some(GLIDE)
                    } else {
                        moving.then_some(TICK)
                    }
                })
                .flatten();
        } else {
            let (frames, _) = self.frames.get_or_insert((0, now));
            *frames += 1;
            self.last_frame = Some(now);
            self.pace = self.feature_pace();
            if self.pace.is_none() {
                self.dirty = true;
            }
        }
        let soonest = [
            self.beaming.then_some(GLIDE),
            (self.aura.is_lit() && !self.stage.hidden()).then_some(AURA_EVERY),
        ]
        .into_iter()
        .flatten()
        .min();
        if !self.dirty
            && let Some(soonest) = soonest
        {
            self.pace = Some(self.pace.map_or(soonest, |pace| pace.min(soonest)));
        }
    }

    /// Puts `sprites` on the surface, or sets up trying again.
    fn show(&mut self, sprites: &[Sprite], gpu: &Gpu, now: Instant) {
        let Some(surface) = &self.surface else {
            return;
        };
        match gpu.draw(surface, sprites, self.scale) {
            Presented::Shown => return,
            Presented::Skipped => {}
            Presented::Outdated => {
                let (w, h) = self.buffer_size();
                gpu.configure(surface, w, h);
            }
            Presented::Lost => self.surface = None,
        }
        self.turn.missed(now);
    }

    fn sync_regions(&mut self, compositor: &CompositorState) {
        let holes = self.holes();
        if self.region.as_ref() == Some(&holes) {
            return;
        }
        let Ok(region) = Region::new(compositor) else {
            return;
        };
        for [x, y, w, h] in &holes {
            region.add(*x, *y, *w, *h);
        }
        self.region = Some(holes);
        self.layer
            .wl_surface()
            .set_input_region(Some(region.wl_region()));
        if let Some(effect) = &self.blur
            && let Ok(blur) = Region::new(compositor)
        {
            let whole = |rect: [f32; 4]| rect.map(|value| value.round() as i32);
            let (radius, power) = self.look.corner;
            let corner = (radius.min(self.body[3] / 2.0), power);
            let rows = match self.merge {
                Some(merge) => crate::merge::rows(self.body, corner, merge),
                None => crate::shape::region_of(whole(self.body), self.look.corner, None),
            };
            for [x, y, w, h] in rows {
                blur.add(x, y, w, h);
            }
            effect.set_blur_region(Some(blur.wl_region()));
        }
    }

    fn in_panel(&self, at: (f32, f32)) -> (f32, f32) {
        (at.0 - self.panel_at.0, at.1 - self.panel_at.1)
    }

    /// The innermost area under `pointer` that `answer` finds something in.
    fn answer_at<T>(&self, pointer: (f32, f32), answer: impl Fn(&Hit) -> Option<T>) -> Option<T> {
        self.placed.iter().rev().find_map(|placed| {
            let hit = self
                .cache
                .get(&placed.widget)?
                .drawn
                .hits
                .get(placed.index)?;
            inside([placed.x, placed.y, placed.w, WIDGET_H], pointer)
                .then(|| answer(hit))
                .flatten()
        })
    }

    pub fn pointer_moved(&mut self, at: (f32, f32)) {
        self.pointer = Some(at);
        if !self.stage.hovered {
            self.stage.set_pointer(true);
        }
        self.leave_at = None;
        if let Some(grip) = self.held.clone() {
            let x = at.0 - self.panel_at.0 - grip.rect[0] - grip.offset;
            let width = grip.rect[2].max(1.0);
            let over = if x < 0.0 { x } else { (x - width).max(0.0) };
            self.stretch.reset(rubber(over, STRETCH_REACH));
            self.pulled = Some(grip.key);
            (grip.on_change)((x / width).clamp(0.0, 1.0), &mut IslandUi { island: self });
            self.dirty = true;
        }
        let chrome = self
            .chrome
            .iter()
            .find(|hit| inside(hit.rect, at) && !hit.tip.is_empty())
            .map(|hit| SharedString::from(hit.tip));
        let entered = self.active_built().and_then(|built| {
            let local = self.in_panel(at);
            built
                .answering(local, |hit| hit.on_hover.is_some())
                .map(|hit| (hit.rect, hit.on_hover.clone()))
        });
        match entered {
            Some((rect, Some(handler))) if self.entered != Some(rect) => {
                self.entered = Some(rect);
                handler(&mut IslandUi { island: self });
            }
            Some(_) => {}
            None => self.entered = None,
        }
        let panel = self.active_built().and_then(|built| {
            let local = self.in_panel(at);
            built
                .answering(local, |hit| hit.tip.is_some())
                .and_then(|hit| hit.tip.clone())
        });
        let read = self.chart_reading(at);
        let charted = chrome.is_none() && panel.is_none() && read.is_some();
        let tip = chrome
            .or(panel)
            .or(read)
            .or_else(|| self.answer_at(at, |hit| hit.tip.clone()));
        match (&mut self.tip, tip) {
            (Some(shown), Some(text)) if shown.text == text => shown.at = at,
            (Some(shown), Some(text)) if shown.charted && charted => {
                shown.text = text;
                shown.at = at;
            }
            (_, Some(text)) => {
                self.tip = Some(Tip {
                    text,
                    since: Instant::now(),
                    at,
                    charted,
                });
            }
            (_, None) => self.tip = None,
        }
        self.dirty = true;
    }

    /// What the chart under `at` says there.
    fn chart_reading(&self, at: (f32, f32)) -> Option<SharedString> {
        let local = self.in_panel(at);
        let phase = self.sampled_at.elapsed().as_secs_f32() / sys::INTERVAL.as_secs_f32();
        let chart = self
            .active_built()?
            .charts
            .iter()
            .find(|chart| inside(chart.rect, local))?;
        let across = (local.0 - chart.rect[0]) / chart.rect[2].max(1.0);
        let phase = chart.phase.map_or(phase, Phase::now).clamp(0.0, 1.0);
        let words = chart::text(&chart.series, card::SLOTS, phase, across);
        (!words.is_empty()).then(|| words.into())
    }

    /// A chart's tip keeps up with the chart under it.
    fn retip(&mut self) {
        let shown = self
            .tip
            .as_ref()
            .filter(|tip| tip.charted)
            .map(|tip| tip.at);
        if let Some(at) = shown
            && let Some(words) = self.chart_reading(at)
            && let Some(tip) = &mut self.tip
        {
            tip.text = words;
        }
    }

    fn active_built(&self) -> Option<&Built> {
        self.built.get(&self.stage.active_layer()?)
    }

    /// Shows `view` of it if it stays open. Whether it is open after.
    pub fn open_panel(&mut self, panel: Panel, view: Option<&str>, toggle: bool) -> bool {
        let closing = toggle && self.stage.mode == Mode::Open(panel);
        if let (Some(view), Panel::Feature(feature), false) = (view, panel, closing) {
            self.stage.update_local(feature, |local| local.show(view));
            self.local_changed(feature);
        }
        if toggle {
            self.toggle(panel)
        } else {
            self.set_mode(Mode::Open(panel));
            true
        }
    }

    /// What it needs covers over the outputs for.
    pub fn grab(&self) -> Grab {
        self.stage
            .grab(self.menu.as_ref().is_some_and(Menu::is_open))
    }

    /// Where the pointer reaches it, for covers to leave open.
    pub fn holes(&self) -> Vec<[i32; 4]> {
        let body = self.body.map(|value| value.round() as i32);
        let (x, y) = self.panel_at;
        let reaches = self.active_built().into_iter().flat_map(|built| {
            built.reaches.iter().map(move |[left, top, w, h]| {
                [left + x, top + y, *w, *h].map(|value| value.round() as i32)
            })
        });
        std::iter::once(body)
            .chain(
                self.menu
                    .as_ref()
                    .map(|menu| menu.rect.map(|value| value.round() as i32)),
            )
            .chain(reaches)
            .collect()
    }

    /// Holds off the screen going idle while `awake`.
    pub fn keep_awake(
        &mut self,
        awake: bool,
        manager: &ZwpIdleInhibitManagerV1,
        handle: &QueueHandle<EffectState>,
    ) {
        if awake == self.inhibitor.is_some() {
            return;
        }
        match self.inhibitor.take() {
            Some(inhibitor) => inhibitor.destroy(),
            None => {
                self.inhibitor =
                    Some(manager.create_inhibitor(self.layer.wl_surface(), handle, ()));
            }
        }
    }

    pub fn dismiss(&mut self) {
        self.close_menu();
        self.set_mode(Mode::Collapsed);
    }

    pub fn key(&mut self, key: &Key) {
        let Mode::Open(Panel::Feature(name)) = self.stage.mode else {
            return;
        };
        let features = self.stage.features.clone();
        let Some(feature) = features.get(name) else {
            return;
        };
        let mut outcome = Outcome::Close;
        self.stage
            .update_local(name, |local| outcome = feature.key(local, key));
        match outcome {
            Outcome::Open => self.local_changed(name),
            Outcome::Close => self.set_mode(Mode::Collapsed),
        }
    }

    /// What the pointer should look like over the island.
    pub fn cursor(&self) -> Cursor {
        if self.held.is_some() {
            return Cursor::Grabbing;
        }
        let Some(at) = self.pointer else {
            return Cursor::Default;
        };
        if self
            .menu
            .as_ref()
            .is_some_and(|menu| menu.action_at(at).is_some())
            || self.chrome.iter().any(|hit| inside(hit.rect, at))
        {
            return Cursor::Pointer;
        }
        let local = self.in_panel(at);
        if let Some(built) = self.active_built() {
            if built.sliding.iter().any(|track| inside(track.seen, local)) {
                return Cursor::Grab;
            }
            if let Some(hit) = built
                .under(local)
                .find(|hit| hit.on_click.is_some() || hit.hover.is_some() || hit.blocks)
            {
                return if hit.on_click.is_some() && !hit.blocks || hit.hover.is_some() {
                    Cursor::Pointer
                } else {
                    Cursor::Default
                };
            }
        }
        if self.stage.mode == Mode::Collapsed
            && self
                .answer_at(at, |hit| {
                    (hit.pressable || hit.on_click.is_some()).then_some(())
                })
                .is_some()
        {
            return Cursor::Pointer;
        }
        Cursor::Default
    }

    /// `button` as Linux numbers them.
    pub fn release(&mut self, button: u32) {
        const RIGHT: u32 = 0x111;
        if button == RIGHT {
            let local = self.pointer.map(|at| self.in_panel(at));
            let handler = self
                .active_built()
                .zip(local)
                .and_then(|(built, local)| built.under(local).find_map(|hit| hit.on_right.clone()));
            if let Some(handler) = handler {
                handler(&mut IslandUi { island: self });
                self.dirty = true;
            }
            return;
        }
        self.held = None;
        self.stretch.retarget(0.0, SHAPE_SPRING);
        self.dirty = true;
        let Some(pressed) = self.pressed.take() else {
            return;
        };
        let Some(at) = self.pointer else {
            return;
        };
        let spot = if pressed.panel { self.in_panel(at) } else { at };
        if !inside(pressed.rect, spot) {
            return;
        }
        match pressed.act {
            Act::Chrome(Action::Open(panel)) => self.set_mode(Mode::Open(panel)),
            Act::Chrome(Action::Close) => self.set_mode(Mode::Collapsed),
            Act::Panel(handler) => handler(&mut IslandUi { island: self }),
            Act::Menu(action) => match action {
                menu::Action::Enter(index) => {
                    if let Some(menu) = &mut self.menu {
                        menu.enter(index);
                    }
                }
                menu::Action::Back => {
                    if let Some(menu) = &mut self.menu {
                        menu.back();
                    }
                }
                menu::Action::Pick(id) => {
                    if let Some(menu) = &self.menu {
                        (menu.pick)(id);
                    }
                    self.close_menu();
                }
            },
            Act::Bar(press, x) => press(Pointer { x }, &mut IslandUi { island: self }),
        }
    }

    /// Names the pointer is on glide; off them they go back to their start.
    fn aim_glides(&mut self) {
        let local = self.pointer.map(|at| self.in_panel(at));
        let on: Vec<u64> = self
            .active_built()
            .into_iter()
            .flat_map(|built| &built.marquees)
            .filter(|marquee| {
                marquee
                    .hover
                    .is_some_and(|rect| local.is_some_and(|local| inside(rect, local)))
            })
            .map(|marquee| marquee.key)
            .collect();
        self.glides.retain(|key, _| on.contains(key));
        for key in on {
            self.glides.entry(key).or_insert_with(Instant::now);
        }
    }

    /// Sliders the pointer is on show a handle, a little bigger while held.
    fn aim_grips(&mut self) {
        let held = self.held.as_ref().map(|grip| grip.key);
        let local = self.pointer.map(|at| self.in_panel(at));
        let Some(built) = self.active_built() else {
            self.grips.clear();
            return;
        };
        let aims: Vec<(u64, f32)> = built
            .sliding
            .iter()
            .filter(|track| track.rect[3] <= HIT_H + 0.5)
            .map(|track| {
                let over = local.is_some_and(|local| inside(track.seen, local));
                let tall = if held == Some(track.key) {
                    HELD_H
                } else if over {
                    HOVER_H
                } else {
                    0.0
                };
                (track.key, tall)
            })
            .collect();
        self.grips
            .retain(|key, _| aims.iter().any(|(aimed, _)| aimed == key));
        for (key, tall) in aims {
            self.grips
                .entry(key)
                .or_insert_with(|| Spring::new(0.0, SHAPE_SPRING, 0.1))
                .retarget(tall, SHAPE_SPRING);
        }
    }

    pub fn pointer_left(&mut self) {
        self.pointer = None;
        self.pressed = None;
        self.tip = None;
        self.stage.set_pointer(false);
        if !matches!(self.stage.mode, Mode::Collapsed | Mode::Open(Panel::Popups)) {
            self.leave_at = Some(Instant::now() + LEAVE_GRACE);
        }
        self.dirty = true;
    }

    /// `button` as Linux numbers them.
    pub fn press(&mut self, button: u32) {
        const LEFT: u32 = 0x110;
        const RIGHT: u32 = 0x111;
        const MIDDLE: u32 = 0x112;
        let Some(at) = self.pointer else {
            return;
        };
        let menu_open = self.menu.as_ref().is_some_and(Menu::is_open);
        self.stopped = false;
        if menu_open {
            let row = self.menu.as_ref().and_then(|menu| menu.action_at(at));
            if let Some((rect, action)) = row.filter(|_| button == LEFT) {
                self.pressed = Some(Pressed {
                    act: Act::Menu(action),
                    rect,
                    panel: false,
                    sprites: (0, 0),
                });
                self.dirty = true;
                return;
            }
            if self.menu.as_ref().is_some_and(|menu| inside(menu.rect, at)) {
                return;
            }
            let holds = button == LEFT
                && self
                    .answer_at(at, |hit| hit.holds_press.then_some(()))
                    .is_some();
            if button == LEFT && !holds {
                self.close_menu();
            }
        }
        if button == LEFT
            && let Some(hit) = self.chrome.iter().find(|hit| inside(hit.rect, at))
        {
            self.pressed = Some(Pressed {
                act: Act::Chrome(hit.action),
                rect: hit.rect,
                panel: false,
                sprites: (0, 0),
            });
            return;
        }
        if let Some(built) = self.active_built() {
            let local = self.in_panel(at);
            let track = built
                .sliding
                .iter()
                .rev()
                .find(|track| button == LEFT && inside(track.seen, local))
                .map(|track| {
                    let level = track.fill.map_or(0.0, |(_, value)| value);
                    (track.key, track.rect, track.on_change.clone(), level)
                });
            if let Some((key, rect, on_change, level)) = track {
                let width = rect[2].max(1.0);
                let from_end = local.0 - (rect[0] + width * level);
                let on_end = from_end.abs() <= HANDLE_W;
                self.held = Some(Grip {
                    key,
                    rect,
                    on_change: on_change.clone(),
                    offset: if on_end { from_end } else { 0.0 },
                });
                if !on_end {
                    let value = ((local.0 - rect[0]) / width).clamp(0.0, 1.0);
                    on_change(value, &mut IslandUi { island: self });
                }
                self.dirty = true;
                return;
            }
            let found = built.under(local).find_map(|hit| {
                let handler = match button {
                    LEFT => hit.on_click.clone(),
                    RIGHT => hit.on_right.clone(),
                    _ => None,
                };
                handler.map(|handler| (handler, hit.rect, hit.immediate, (hit.from, hit.to)))
            });
            if let Some((handler, rect, immediate, sprites)) = found {
                if button == RIGHT {
                    return;
                }
                if immediate {
                    handler(&mut IslandUi { island: self });
                } else {
                    self.pressed = Some(Pressed {
                        act: Act::Panel(handler),
                        rect,
                        panel: true,
                        sprites,
                    });
                }
                self.dirty = true;
                return;
            }
        }
        let press = self.answer_at(at, |hit| match button {
            LEFT => hit.on_click.clone(),
            RIGHT => hit.on_right.clone(),
            MIDDLE => hit.on_middle.clone(),
            _ => None,
        });
        if let Some(press) = press {
            let place = self
                .placed
                .iter()
                .rev()
                .find(|placed| inside([placed.x, placed.y, placed.w, WIDGET_H], at))
                .map(|placed| [placed.x, placed.y, placed.w, WIDGET_H]);
            match place.filter(|_| button == LEFT) {
                Some(rect) => {
                    self.pressed = Some(Pressed {
                        act: Act::Bar(press, at.0),
                        rect,
                        panel: false,
                        sprites: (0, 0),
                    });
                }
                None => press(Pointer { x: at.0 }, &mut IslandUi { island: self }),
            }
        }
        if menu_open && button != LEFT && !self.stopped {
            self.close_menu();
        }
        self.dirty = true;
    }

    /// `touch` is a touchpad, which pulls past the ends; a wheel stops at them.
    pub fn scroll(&mut self, delta: f32, touch: bool) {
        let Some(at) = self.pointer else {
            return;
        };
        let local = self.in_panel(at);
        let rows = self
            .active_built()
            .and_then(|built| built.under(local).find_map(|hit| hit.on_rows.clone()));
        if let Some(rows) = rows {
            let per_notch = if touch {
                PIXELS_PER_NOTCH / ROW_PIXELS
            } else {
                LINES_PER_NOTCH
            };
            let total = self.rows_scrolled - delta * per_notch;
            let moved = total.trunc();
            self.rows_scrolled = total - moved;
            if moved != 0.0 {
                rows(moved as i32, &mut IslandUi { island: self });
            }
            return;
        }
        let area = self.active_built().and_then(|built| {
            built
                .scrolling
                .iter()
                .rev()
                .find(|area| inside(area.rect, local))
                .map(|area| (area.key.clone(), area.reach))
        });
        if let Some((key, reach)) = area {
            if touch && reach > 0.0 {
                let moved = delta * PIXELS_PER_NOTCH;
                let offset = self.offsets.entry(key.clone()).or_default();
                let over = self.overs.entry(key).or_insert_with(|| Over {
                    pull: 0.0,
                    last: None,
                    stretch: Spring::new(0.0, SHAPE_SPRING, 0.1),
                });
                let (mut pull, y) = settle(over.pull, moved - *offset, moved, reach);
                let showing = over.stretch.value();
                if over.pull == 0.0 && pull * showing > 0.0 {
                    pull += unrubber(showing, REACH);
                }
                over.pull = pull;
                if pull != 0.0 {
                    over.last = Some(Instant::now());
                }
                *offset = -y;
                self.stale = true;
                self.dirty = true;
                return;
            }
            let offset = self.offsets.entry(key).or_default();
            *offset = (*offset - delta * WHEEL_PIXELS).clamp(0.0, reach);
            self.stale = true;
            self.dirty = true;
            return;
        }
        let scroll = self.answer_at(at, |hit| hit.on_scroll.clone());
        if let Some(scroll) = scroll {
            scroll(delta, &mut IslandUi { island: self });
        }
    }
}

struct IslandUi<'a> {
    island: &'a mut Island,
}

impl Ui for IslandUi<'_> {
    fn run(&mut self, handler: &Handler) {
        handler(self);
    }

    fn open(&mut self, panel: Panel) {
        self.island.set_mode(Mode::Open(panel));
    }

    fn toggle(&mut self, panel: Panel) {
        self.island.toggle(panel);
    }

    fn open_named(&mut self, name: &str, view: Option<&str>, toggle: bool) {
        if let Some(panel) = Panel::parse(name) {
            self.island.open_panel(panel, view, toggle);
        }
    }

    fn collapse(&mut self) {
        self.island.set_mode(Mode::Collapsed);
    }

    fn stop_propagation(&mut self) {
        self.island.stopped = true;
    }

    fn below_bar(&self, x: f32) -> Option<(i32, i32)> {
        let monitor = self.island.stage.monitor()?;
        let (left, top) = compositor::backend()
            .and_then(|backend| backend.surface_origin(&monitor.name))
            .unwrap_or((
                monitor.x + (monitor.width - SURFACE_W as i32) / 2,
                monitor.y,
            ));
        let y = self.island.look.top + COLLAPSED_H + MENU_GAP;
        Some((left + x.round() as i32, top + y.round() as i32))
    }

    fn menu_open_for(&self, source: &str) -> bool {
        self.island
            .menu
            .as_ref()
            .is_some_and(|menu| menu.is_open() && menu.source == source)
    }

    fn close_menu(&mut self) {
        self.island.close_menu();
    }

    fn menu_when_ready(
        &mut self,
        source: String,
        response: oneshot::Receiver<Option<MenuNode>>,
        anchor: f32,
        pick: Box<dyn Fn(i32)>,
    ) {
        self.island.awaited = Some(Awaited {
            source,
            response,
            anchor,
            pick,
        });
    }

    fn menu_enter(&mut self, _index: usize) {}

    fn menu_pick(&mut self, _id: i32) {}

    fn update_local(&mut self, name: &'static str, action: &mut dyn FnMut(&mut dyn AnyLocal)) {
        self.island.stage.update_local(name, action);
        self.island.local_changed(name);
    }
}
