use std::{path::Path, rc::Rc};

use gpui::SharedString;

use crate::{
    components::{
        card::{self, stat},
        chart::Series,
        tile::{Drive, Meter as TileMeter, Part, Segment, Tile, Volume},
    },
    glyphs::Glyphs,
    island::{Island, Mode, Panel},
    launcher::Search,
    panel::{
        AppRow, Block, Button, Entry, Header, IconButton, LauncherView, Meter, PanelView,
        on,
    },
    sys::SysStats,
    theme::{BLUE, GREEN, LAVENDER, MAUVE, PEACH, PINK, RED, SUBTEXT1, TEAL, YELLOW},
};

/// Nothing, for a panel whose content went away while it closes.
fn empty<'a>() -> PanelView<'a> {
    PanelView {
        header: Header::None,
        blocks: Vec::new(),
    }
}

fn gib(bytes: f64) -> f64 {
    bytes / (1024.0 * 1024.0 * 1024.0)
}

fn rate(bytes_per_second: f64) -> String {
    match bytes_per_second {
        rate if rate >= 1024.0 * 1024.0 => format!("{:.1} MB/s", rate / (1024.0 * 1024.0)),
        rate if rate >= 1024.0 => format!("{:.1} kB/s", rate / 1024.0),
        rate => format!("{rate:.0} B/s"),
    }
}

fn row(label: impl Into<SharedString>, value: impl Into<SharedString>) -> (SharedString, SharedString) {
    (label.into(), value.into())
}

/// Binary size with one unit letter: "79 G", "1.4 T", "512 M".
fn size(bytes: u64) -> String {
    let gib = gib(bytes as f64);
    match gib {
        gib if gib >= 1024.0 => format!("{:.1} T", gib / 1024.0),
        gib if gib >= 10.0 => format!("{gib:.0} G"),
        gib if gib >= 1.0 => format!("{gib:.1} G"),
        _ => format!("{:.0} M", gib * 1024.0),
    }
}

fn short_duration(seconds: u64) -> String {
    let (days, hours, minutes) = (seconds / 86_400, (seconds / 3600) % 24, (seconds / 60) % 60);
    match days {
        0 => format!("{hours}h {minutes}m"),
        _ => format!("{days}d {hours}h"),
    }
}

/// Green when cool, through yellow and peach, to red when hot.
fn heat(celsius: f32) -> u32 {
    match celsius {
        celsius if celsius >= 85.0 => RED,
        celsius if celsius >= 70.0 => PEACH,
        celsius if celsius >= 55.0 => YELLOW,
        _ => GREEN,
    }
}

/// A round chart ceiling (1, 2 or 5 × 10ⁿ) above every value, never below `floor`.
fn nice_peak<'a>(values: impl IntoIterator<Item = &'a f32>, floor: f32) -> f32 {
    let peak = values.into_iter().copied().fold(floor, f32::max);
    let magnitude = 10f32.powf(peak.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|step| step * magnitude)
        .find(|nice| *nice >= peak)
        .unwrap_or(peak)
}

fn series(values: &[f32], max: f32, color: u32) -> Series {
    Series {
        values: values.to_vec(),
        max,
        color,
    }
}

fn tile_meter(label: impl Into<SharedString>, fraction: f32, value: impl Into<SharedString>, color: u32) -> TileMeter {
    TileMeter {
        label: label.into(),
        fraction,
        value: value.into(),
        color,
    }
}

fn titled(id: &str, span: usize, title: &str, value: Option<card::Stat>, parts: Vec<Part>) -> Tile {
    Tile {
        id: id.to_owned().into(),
        span,
        title: Some((title.to_owned().into(), value)),
        parts,
    }
}

fn cpu_tile(stats: &SysStats) -> Tile {
    let legend = [
        stats.cpu_temp.map(|temp| (Some(MAUVE), format!("{temp:.0}°C"))),
        stats.cpu_ghz.map(|ghz| (None, format!("{ghz:.2} GHz"))),
        Some((None, format!("{} threads", stats.threads))),
    ];
    titled(
        "tile-cpu",
        1,
        "CPU",
        Some(stat(format!("{:.0}%", stats.cpu * 100.0), PINK)),
        vec![
            Part::Chart {
                series: vec![series(&stats.cpu_history, 1.0, PINK), series(&stats.cpu_temp_history, 100.0, MAUVE)],
                grow: true,
            },
            Part::Cores {
                loads: stats.cores.clone(),
                color: PINK,
            },
            Part::Legend(legend.into_iter().flatten().map(|(dot, text)| (dot, text.into())).collect()),
        ],
    )
}

fn gpu_tile(stats: &SysStats) -> Tile {
    let Some(gpu) = stats.gpu else {
        return titled("tile-gpu", 1, "GPU", None, vec![Part::Spacer, Part::Legend(vec![(None, "No GPU found".into())])]);
    };
    let legend = [
        gpu.temp.map(|temp| (Some(PEACH), format!("{temp:.0}°C"))),
        gpu.power.map(|watts| (None, format!("{watts:.0} W"))),
        gpu.fan_rpm.map(|rpm| (None, format!("{rpm} RPM"))),
    ];
    titled(
        "tile-gpu",
        1,
        "GPU",
        Some(stat(format!("{:.0}%", gpu.busy * 100.0), PINK)),
        vec![
            Part::Chart {
                series: vec![series(&stats.gpu_history, 1.0, PINK), series(&stats.gpu_temp_history, 100.0, PEACH)],
                grow: true,
            },
            Part::Meters {
                rows: vec![tile_meter(
                    "VRAM",
                    gpu.vram_used as f32 / gpu.vram_total.max(1) as f32,
                    format!("{} / {}", size(gpu.vram_used), size(gpu.vram_total)),
                    PEACH,
                )],
                scroll: false,
            },
            Part::Legend(legend.into_iter().flatten().map(|(dot, text)| (dot, text.into())).collect()),
        ],
    )
}

fn memory_tile(stats: &SysStats) -> Tile {
    let kib = |value: u64| value * 1024;
    let share = |used: u64, total: u64| used as f32 / total.max(1) as f32;
    let mut rows = vec![tile_meter(
        "RAM",
        share(stats.mem_used_kib, stats.mem_total_kib),
        format!("{} / {}", size(kib(stats.mem_used_kib)), size(kib(stats.mem_total_kib))),
        PEACH,
    )];
    if stats.swap_total_kib > 0 {
        rows.push(tile_meter(
            "Swap",
            share(stats.swap_used_kib, stats.swap_total_kib),
            format!("{} / {}", size(kib(stats.swap_used_kib)), size(kib(stats.swap_total_kib))),
            YELLOW,
        ));
    }
    titled(
        "tile-memory",
        1,
        "Memory",
        Some(stat(format!("{:.0}%", share(stats.mem_used_kib, stats.mem_total_kib) * 100.0), PEACH)),
        vec![
            Part::Chart {
                series: vec![series(&stats.mem_history, 1.0, PEACH)],
                grow: true,
            },
            Part::Meters { rows, scroll: false },
        ],
    )
}

fn processes_tile(stats: &SysStats) -> Tile {
    let busiest = stats.processes.iter().map(|process| process.cpu).fold(f32::EPSILON, f32::max);
    let rows = stats
        .processes
        .iter()
        .map(|process| {
            tile_meter(process.name.clone(), process.cpu / busiest, format!("{:.1}%", process.cpu * 100.0), PINK)
        })
        .collect();
    titled(
        "tile-processes",
        1,
        "Processes",
        Some(stat(stats.process_count.to_string(), SUBTEXT1)),
        vec![Part::Spacer, Part::Meters { rows, scroll: false }],
    )
}

fn power_tile(stats: &SysStats) -> Tile {
    const COLORS: [u32; 3] = [YELLOW, TEAL, MAUVE];
    if stats.power.is_empty() {
        return titled("tile-power", 1, "Power", None, vec![Part::Spacer, Part::Legend(vec![(None, "No power sensors".into())])]);
    }
    let peak = nice_peak(stats.power.iter().flat_map(|source| &source.history), 10.0);
    let total: f32 = stats.power.iter().map(|source| source.watts).sum();
    titled(
        "tile-power",
        1,
        "Power",
        Some(stat(format!("{total:.0} W"), YELLOW)),
        vec![
            Part::Chart {
                series: stats
                    .power
                    .iter()
                    .zip(COLORS)
                    .map(|(source, color)| series(&source.history, peak, color))
                    .collect(),
                grow: true,
            },
            Part::Legend(
                stats
                    .power
                    .iter()
                    .zip(COLORS)
                    .map(|(source, color)| (Some(color), format!("{:.0} W {}", source.watts, source.label).into()))
                    .collect(),
            ),
        ],
    )
}

fn network_tile(stats: &SysStats) -> Tile {
    let peak = nice_peak(stats.rx_history.iter().chain(&stats.tx_history), 64.0 * 1024.0);
    titled(
        "tile-network",
        1,
        "Network",
        Some(stat(format!("↓ {}", rate(stats.rx_rate)), TEAL)),
        vec![
            Part::Chart {
                series: vec![series(&stats.rx_history, peak, TEAL), series(&stats.tx_history, peak, PEACH)],
                grow: true,
            },
            Part::Legend(vec![
                (Some(TEAL), format!("↓ {}", rate(stats.rx_rate)).into()),
                (Some(PEACH), format!("↑ {}", rate(stats.tx_rate)).into()),
            ]),
        ],
    )
}

fn disk_tile(stats: &SysStats) -> Tile {
    let peak = nice_peak(stats.read_history.iter().chain(&stats.write_history), 1024.0 * 1024.0);
    titled(
        "tile-disk",
        1,
        "Disk activity",
        Some(stat(format!("R {}", rate(stats.read_rate)), TEAL)),
        vec![
            Part::Chart {
                series: vec![series(&stats.read_history, peak, TEAL), series(&stats.write_history, peak, PEACH)],
                grow: true,
            },
            Part::Legend(vec![
                (Some(TEAL), format!("R {}", rate(stats.read_rate)).into()),
                (Some(PEACH), format!("W {}", rate(stats.write_rate)).into()),
            ]),
        ],
    )
}

/// "Samsung SSD 980 PRO 1TB" → "Samsung 980 PRO": the kind of device and its capacity are shown elsewhere.
fn drive_model(model: &str) -> String {
    let capacity = |word: &str| {
        let digits = word.trim_end_matches(|char: char| char.is_ascii_alphabetic());
        let unit = &word[digits.len()..];
        !digits.is_empty()
            && digits.chars().all(|char| char.is_ascii_digit() || char == '.')
            && ["TB", "GB", "T", "G"].contains(&unit.to_uppercase().as_str())
    };
    model
        .split_whitespace()
        .filter(|word| !matches!(word.to_uppercase().as_str(), "SSD" | "NVME") && !capacity(word))
        .collect::<Vec<_>>()
        .join(" ")
}

fn storage_tile(stats: &SysStats) -> Tile {
    const COLORS: [u32; 5] = [BLUE, LAVENDER, TEAL, MAUVE, PEACH];
    let volumes: Vec<Volume> = stats
        .partitions
        .iter()
        .map(|partition| {
            let fraction = partition.used as f32 / partition.total.max(1) as f32;
            Volume {
                mount: partition.mount.clone().into(),
                detail: format!("{} · {}", partition.device, partition.fs).into(),
                usage: format!("{} / {}", size(partition.used), size(partition.total)).into(),
                fraction,
                color: if fraction >= 0.9 { RED } else { BLUE },
            }
        })
        .collect();
    let count = volumes.len();
    let mut parts = vec![Part::Volumes(volumes)];
    if count <= 4 && !stats.drives.is_empty() {
        let mut palette = COLORS.into_iter().cycle();
        let drives = stats
            .drives
            .iter()
            .map(|drive| {
                let mut segments: Vec<Segment> = drive
                    .parts
                    .iter()
                    .map(|part| Segment {
                        weight: part.size as f32,
                        color: part.mount.as_ref().and_then(|_| palette.next()),
                    })
                    .collect();
                let free = drive.size.saturating_sub(drive.parts.iter().map(|part| part.size).sum());
                if free > drive.size / 100 {
                    segments.push(Segment {
                        weight: free as f32,
                        color: None,
                    });
                }
                let model = drive_model(&drive.model);
                Drive {
                    name: if model.is_empty() { drive.name.clone().into() } else { model.into() },
                    size: size(drive.size).into(),
                    segments,
                }
            })
            .collect();
        parts.push(Part::Drives(drives));
    }
    titled(
        "tile-storage",
        2,
        "Storage",
        Some(stat(
            format!("{count} partition{}", if count == 1 { "" } else { "s" }),
            BLUE,
        )),
        parts,
    )
}

fn host_tile(stats: &SysStats, displays: usize) -> Tile {
    let info = &stats.info;
    let hardware = [info.cpu.as_str(), info.gpu.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let mut facts = vec![
        row("Uptime", short_duration(stats.uptime_secs)),
        row("Load", format!("{:.2}", stats.load[0])),
        row("Session", info.session.clone()),
    ];
    if let Some(generation) = info.generation {
        facts.push(row("Generation", generation.to_string()));
    }
    facts.push(row("Displays", displays.to_string()));
    facts.push(row("Threads", stats.threads.to_string()));
    Tile {
        id: "tile-host".into(),
        span: 1,
        title: None,
        parts: vec![
            Part::Hero {
                name: if info.hostname.is_empty() { "This computer".into() } else { info.hostname.clone().into() },
                lines: vec![format!("{} · Linux {}", info.os, info.kernel).into(), hardware.into()],
            },
            Part::Spacer,
            Part::Facts(facts),
        ],
    }
}

fn temperatures_tile(stats: &SysStats) -> Tile {
    let hottest = stats.sensors.iter().map(|sensor| sensor.celsius).fold(f32::MIN, f32::max);
    let histories = [
        (&stats.cpu_temp_history, MAUVE, "CPU"),
        (&stats.gpu_temp_history, PEACH, "GPU"),
        (&stats.storage_temp_history, TEAL, "NVMe"),
    ];
    let shown: Vec<_> = histories.into_iter().filter(|(history, _, _)| !history.is_empty()).collect();
    titled(
        "tile-temperatures",
        2,
        "Temperatures",
        (hottest > f32::MIN).then(|| stat(format!("{hottest:.0}°C"), heat(hottest))),
        vec![
            Part::Chart {
                series: shown.iter().map(|(history, color, _)| series(history, 100.0, *color)).collect(),
                grow: false,
            },
            Part::Legend(shown.iter().map(|(_, color, label)| (Some(*color), (*label).into())).collect()),
            Part::Meters {
                rows: stats
                    .sensors
                    .iter()
                    .map(|sensor| {
                        tile_meter(
                            sensor.label.clone(),
                            sensor.celsius / 100.0,
                            format!("{:.0}°C", sensor.celsius),
                            heat(sensor.celsius),
                        )
                    })
                    .collect(),
                scroll: true,
            },
        ],
    )
}

/// The System panel; `buttons` are the features' quick buttons.
pub fn system(stats: &SysStats, displays: usize, buttons: Vec<IconButton>) -> PanelView<'static> {
    PanelView {
        header: Header::Tabs,
        blocks: vec![
            Block::Icons(buttons),
            Block::Board(vec![
                vec![cpu_tile(stats), memory_tile(stats), processes_tile(stats), power_tile(stats)],
                vec![gpu_tile(stats), network_tile(stats), storage_tile(stats)],
                vec![host_tile(stats, displays), temperatures_tile(stats), disk_tile(stats)],
            ]),
        ],
    }
}

/// One entry of a menu, with its submenu.
#[derive(Clone, Debug)]
pub struct MenuNode {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub separator: bool,
    pub toggle: Option<bool>,
    pub children: Vec<MenuNode>,
}

/// What happens when a menu entry with this id is picked.
pub type Pick = Rc<dyn Fn(i32, &mut Island, &mut gpui::Context<Island>)>;

/// A menu shown as the island's Menu panel, such as a tray item's.
#[derive(Clone)]
pub struct MenuView {
    pub title: String,
    pub root: MenuNode,
    /// The submenus entered, from the root.
    pub path: Vec<usize>,
    pub pick: Pick,
}

impl MenuView {
    fn current(&self) -> &MenuNode {
        self.path
            .iter()
            .fold(&self.root, |node, index| node.children.get(*index).unwrap_or(node))
    }
}

pub fn menu(view: &MenuView) -> PanelView<'static> {
    let current = view.current();
    let entries = current
        .children
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let pick = view.pick.clone();
            let id = node.id;
            let submenu = !node.children.is_empty();
            Entry {
                id: format!("menu-{}-{index}", node.id).into(),
                label: node.label.clone().into(),
                enabled: node.enabled,
                separator: node.separator,
                check: node.toggle,
                submenu,
                on_click: Some(if submenu {
                    on(move |island, cx| island.menu_enter(index, cx))
                } else {
                    on(move |island, cx| {
                        pick(id, island, cx);
                        island.set_mode(Mode::Collapsed, cx);
                    })
                }),
            }
        })
        .collect();
    let title = if view.path.is_empty() {
        view.title.clone()
    } else {
        current.label.clone()
    };
    let buttons = if view.path.is_empty() {
        Vec::new()
    } else {
        vec![Button {
            id: "menu-back".into(),
            label: "Back".into(),
            active: false,
            toggle: false,
            on_click: on(|island, cx| island.menu_back(cx)),
        }]
    };
    PanelView {
        header: Header::Title(title.into(), buttons),
        blocks: vec![Block::Menu(entries)],
    }
}

/// A brief popup for a level that just changed, such as volume or brightness.
#[derive(Clone, Debug, PartialEq)]
pub struct Osd {
    /// The glyph key of its icon.
    pub glyph: &'static str,
    pub color: u32,
    /// What the level is of, e.g. "Output"; may be empty.
    pub caption: &'static str,
    pub label: String,
    /// Whether the device itself changed, rather than its level.
    pub switched: bool,
    pub value: f32,
    pub muted: bool,
}

pub fn osd(osd: Osd, glyphs: &Glyphs) -> PanelView<'static> {
    PanelView {
        header: Header::None,
        blocks: vec![Block::Meter(Meter {
            glyph: glyphs.get(osd.glyph).map(Path::to_path_buf),
            caption: osd.caption.into(),
            label: osd.label.into(),
            switched: osd.switched,
            value: osd.value,
            muted: osd.muted,
            color: osd.color,
        })],
    }
}

pub fn panel<'a>(island: &'a Island, panel: Panel, cx: &gpui::App) -> PanelView<'a> {
    match panel {
        Panel::System => system(&island.stats, island.displays(), island.feature_buttons(cx)),
        Panel::Osd => island.osd().map_or_else(empty, |current| osd(current, cx.global())),
        Panel::Menu => island.menu_view().map_or_else(empty, menu),
        Panel::Launcher => island.search().map_or_else(empty, launcher),
        Panel::Feature(name) => island.feature_panel(name, cx).unwrap_or_else(empty),
        Panel::Popup(name) => island.feature_popup(name, cx).unwrap_or_else(empty),
    }
}

pub fn launcher(search: &Search) -> PanelView<'static> {
    let rows = search
        .visible()
        .map(|(position, app)| AppRow {
            id: format!("app-{}", app.id).into(),
            position,
            name: app.name.clone().into(),
            detail: if app.generic.is_empty() { app.comment.clone() } else { app.generic.clone() }.into(),
            icon: app.icon.clone(),
            selected: position == search.selected,
        })
        .collect();
    PanelView {
        header: Header::None,
        blocks: vec![Block::Launcher(LauncherView {
            query: search.query.clone().into(),
            rows,
        })],
    }
}
