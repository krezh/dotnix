use std::{
    sync::{LazyLock, OnceLock},
    time::Instant,
};

use crate::{
    config, features,
    spring::Spring,
    stage::{Panel, SHAPE_SPRING},
    ui::{HEADER_H, STRIP_PAD},
};

const EDGE: f32 = 14.0;
pub const WIDGET_H: f32 = 28.0;
const WIDGET_PAD: f32 = 8.0;
pub const GLYPH: f32 = 15.0;
pub const ICON_GAP: f32 = 6.0;
pub const METER_W: f32 = 4.0;
pub const METER_H: f32 = 16.0;
/// Each tab shows its glyph, `tab-<panel>`, and names itself in a tooltip.
const TAB_W: f32 = 48.0;
pub const TAB_GLYPH: f32 = 18.0;
pub const TAB_GAP: f32 = 4.0;
const CLOSE_GAP: f32 = 16.0;
/// The Dashboard tab's place among the tabs.
const DASHBOARD_ORDER: u8 = 4;
/// Between a dashboard view's glyph and the name it shows while picked.
pub const LABEL_GAP: f32 = 8.0;
pub const LABEL_SIZE: f32 = 13.0;
/// The glyph's place from a tab's start, where it stays as the tab widens to show its name.
pub const GLYPH_X: f32 = (TAB_W - TAB_GLYPH) / 2.0;
/// Room for the line between the Dashboard tab and its views, while they show.
pub const DIVIDER_GAP: f32 = 7.0;
pub const DIVIDER_H: f32 = 16.0;
/// The picked tab's tint, outside the dashboard's views.
pub const PICKED_ALPHA: u8 = 0x2e;
pub const VIEWS_ALPHA: u8 = 0x1a;
pub const VIEWS_BORDER_ALPHA: u8 = 0x38;
pub const DIVIDER_ALPHA: u8 = 0x4d;

/// Island's own tabs; each feature's tab glyph comes with the feature's glyphs.
pub const GLYPHS: &[features::GlyphSpec] = &[
    ("tab-system", &["cpu"]),
    ("tab-dashboard", &["layout-grid"]),
    ("dashboard-warning", &["triangle-alert"]),
    ("dashboard-battery", &["battery-full"]),
];

pub struct Tab {
    pub panel: Panel,
    pub title: &'static str,
    /// One of the dashboard's views, which show only while one of them is open.
    pub view: bool,
}

/// The dashboard's views follow one another where its tab goes, the overview first.
pub fn tab_list() -> &'static [Tab] {
    static TABS: LazyLock<Vec<Tab>> = LazyLock::new(|| {
        let mut views = vec![
            (Panel::Dashboard, "Dashboard", 0),
            (Panel::System, "System", 2),
        ];
        views.extend(
            features::dashboard_tabs()
                .into_iter()
                .map(|(name, title, order)| (Panel::Feature(name), title, order)),
        );
        views.sort_by_key(|(_, _, order)| *order);
        let mut tabs: Vec<(u8, u8, Tab)> = features::tabs()
            .into_iter()
            .map(|(name, title, order)| {
                let panel = Panel::Feature(name);
                (
                    order,
                    0,
                    Tab {
                        panel,
                        title,
                        view: false,
                    },
                )
            })
            .collect();
        tabs.extend(views.into_iter().map(|(panel, title, order)| {
            (
                DASHBOARD_ORDER,
                order,
                Tab {
                    panel,
                    title,
                    view: true,
                },
            )
        }));
        tabs.sort_by_key(|(order, within, _)| (*order, *within));
        tabs.into_iter().map(|(_, _, tab)| tab).collect()
    });
    &TABS
}

static LABELS: OnceLock<Vec<f32>> = OnceLock::new();

/// Measures the names the dashboard's views show while picked, once, as text is first drawn.
pub fn measure_labels(width: &mut dyn FnMut(&str, f32) -> f32) {
    LABELS.get_or_init(|| {
        tab_list()
            .iter()
            .map(|tab| {
                if tab.view {
                    width(tab.title, LABEL_SIZE)
                } else {
                    0.0
                }
            })
            .collect()
    });
}

fn label_w(index: usize) -> f32 {
    LABELS
        .get()
        .and_then(|labels| labels.get(index))
        .map_or(0.0, |width| LABEL_GAP + width)
}

pub fn tab_index(kind: Panel) -> Option<usize> {
    tab_list().iter().position(|tab| tab.panel == kind)
}

/// The strip with room for the close button on both sides, at its widest, with every view showing
/// and the longest name.
pub fn tabs_width() -> f32 {
    let tabs = tab_list();
    let widest = (0..tabs.len()).map(label_w).fold(0.0, f32::max);
    tabs.len() as f32 * (TAB_W + TAB_GAP) - TAB_GAP
        + widest
        + DIVIDER_GAP
        + 2.0 * STRIP_PAD
        + 2.0 * (CLOSE_GAP + HEADER_H)
}

/// The tabs' motion: the pill sliding to the picked one, the dashboard's views fanning out of its
/// tab while one of them is open, and the picked view widening to show its name.
pub struct TabStrip {
    pill_x: Spring,
    pill_w: Spring,
    fan: Spring,
    labels: Vec<Spring>,
    picked: Panel,
    /// The dashboard's view last open, where its tab goes back to.
    last_view: Panel,
}

impl Default for TabStrip {
    fn default() -> Self {
        let spring = |value| Spring::new(value, SHAPE_SPRING, 0.1);
        let shown = |value| Spring::new(value, SHAPE_SPRING, 0.002);
        Self {
            pill_x: spring(0.0),
            pill_w: spring(TAB_W),
            fan: shown(0.0),
            labels: tab_list().iter().map(|_| shown(0.0)).collect(),
            picked: Panel::Dashboard,
            last_view: Panel::Dashboard,
        }
    }
}

impl TabStrip {
    /// Where each tab starts and how wide it is, and the strip's width, with the springs where
    /// they are now or, with `settled`, where they are going.
    fn layout(&self, settled: bool) -> (Vec<(f32, f32)>, f32) {
        let at = |spring: &Spring| {
            if settled {
                spring.target()
            } else {
                spring.value()
            }
        };
        let fan = at(&self.fan).clamp(0.0, 1.0);
        let mut x = 0.0;
        let mut places = Vec::new();
        for (index, tab) in tab_list().iter().enumerate() {
            let label = at(&self.labels[index]).clamp(0.0, 1.0) * label_w(index);
            let shown = if tab.view && tab.panel != Panel::Dashboard {
                fan
            } else {
                1.0
            };
            let width = shown * (TAB_W + label);
            places.push((x, width));
            x += width + shown * TAB_GAP;
            if tab.panel == Panel::Dashboard {
                x += fan * DIVIDER_GAP;
            }
        }
        (places, (x - TAB_GAP).max(0.0))
    }

    pub fn width(&self) -> f32 {
        self.layout(false).1
    }

    /// Where each tab is now, and the strip's width.
    pub fn places(&self) -> (Vec<(f32, f32)>, f32) {
        self.layout(false)
    }

    pub fn fan(&self) -> f32 {
        self.fan.value().clamp(0.0, 1.0)
    }

    pub fn pill(&self) -> (f32, f32) {
        (self.pill_x.value(), self.pill_w.value())
    }

    pub fn label(&self, index: usize) -> f32 {
        self.labels[index].value().clamp(0.0, 1.0)
    }

    pub fn picked(&self) -> Panel {
        self.picked
    }

    pub fn last_view(&self) -> Panel {
        self.last_view
    }

    /// `at_once` puts everything in place without moving, such as while the tabs are hidden.
    pub fn pick(&mut self, panel: Panel, at_once: bool) {
        let Some(index) = tab_index(panel) else {
            return;
        };
        let view = tab_list()[index].view;
        self.picked = panel;
        if view {
            self.last_view = panel;
        }
        let fan = if view { 1.0 } else { 0.0 };
        self.fan.retarget(fan, SHAPE_SPRING);
        for (at, label) in self.labels.iter_mut().enumerate() {
            let shown = if view && at == index { 1.0 } else { 0.0 };
            label.retarget(shown, SHAPE_SPRING);
        }
        let (places, _) = self.layout(true);
        let (x, w) = places[index];
        self.pill_x.retarget(x, SHAPE_SPRING);
        self.pill_w.retarget(w, SHAPE_SPRING);
        if at_once {
            for spring in [&mut self.pill_x, &mut self.pill_w, &mut self.fan]
                .into_iter()
                .chain(self.labels.iter_mut())
            {
                spring.reset(spring.target());
            }
        }
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        for spring in [&mut self.pill_x, &mut self.pill_w, &mut self.fan]
            .into_iter()
            .chain(self.labels.iter_mut())
        {
            spring.advance(dt, now);
        }
    }

    pub fn is_settled(&self) -> bool {
        [&self.pill_x, &self.pill_w, &self.fan]
            .into_iter()
            .chain(self.labels.iter())
            .all(Spring::is_settled)
    }
}

pub fn half(spacing: f32) -> f32 {
    WIDGET_PAD + spacing / 2.0
}

pub fn edge(layout: &config::Bar) -> f32 {
    (EDGE - half(layout.spacing)).max(0.0)
}
