use gpui::SharedString;

use crate::{
    components::{
        card::{self, stat},
        chart::Series,
        tile::{Drive, Meter as TileMeter, Part, Segment, Tile, Volume},
    },
    config,
    island::{Island, Panel},
    panel::{Block, ButtonRow, Header, IconButton, PanelView},
    sys::SysStats,
    theme::{BLUE, GREEN, LAVENDER, MAUVE, PEACH, PINK, RED, SUBTEXT1, TEAL, YELLOW},
};

/// Nothing, for a panel whose content went away while it closes.
fn empty<'a>() -> PanelView {
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

fn row(
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
) -> (SharedString, SharedString) {
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

fn series(
    values: &[f32],
    max: f32,
    color: u32,
    label: impl Into<SharedString>,
    format: fn(f32) -> String,
) -> Series {
    Series {
        values: values.to_vec(),
        max,
        color,
        label: label.into(),
        format,
    }
}

fn percent(value: f32) -> String {
    format!("{:.0}%", value * 100.0)
}

fn temperature(value: f32) -> String {
    format!("{value:.0}°C")
}

fn watts(value: f32) -> String {
    format!("{value:.0} W")
}

fn tile_meter(
    label: impl Into<SharedString>,
    fraction: f32,
    value: impl Into<SharedString>,
    color: u32,
) -> TileMeter {
    TileMeter {
        label: label.into(),
        fraction,
        value: value.into(),
        color,
        mark: None,
    }
}

fn titled(id: &str, span: usize, title: &str, value: Option<card::Stat>, parts: Vec<Part>) -> Tile {
    Tile {
        id: id.to_owned().into(),
        span,
        title: Some((title.to_owned().into(), value)),
        on_click: None,
        parts,
    }
}

fn cpu_tile(stats: &SysStats) -> Tile {
    let legend = [
        stats
            .cpu_temp
            .map(|temp| (Some(MAUVE), format!("{temp:.0}°C"))),
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
                series: vec![
                    series(&stats.cpu_history, 1.0, PINK, "Usage", percent),
                    series(
                        &stats.cpu_temp_history,
                        100.0,
                        MAUVE,
                        "Temperature",
                        temperature,
                    ),
                ],
                grow: true,
            },
            Part::Cores {
                loads: stats.cores.clone(),
                color: PINK,
            },
            Part::Legend(
                legend
                    .into_iter()
                    .flatten()
                    .map(|(dot, text)| (dot, text.into()))
                    .collect(),
            ),
        ],
    )
}

fn gpu_tile(stats: &SysStats) -> Tile {
    let Some(gpu) = stats.gpu else {
        return titled(
            "tile-gpu",
            1,
            "GPU",
            None,
            vec![
                Part::Spacer,
                Part::Legend(vec![(None, "No GPU found".into())]),
            ],
        );
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
                series: vec![
                    series(&stats.gpu_history, 1.0, PINK, "Usage", percent),
                    series(
                        &stats.gpu_temp_history,
                        100.0,
                        PEACH,
                        "Temperature",
                        temperature,
                    ),
                ],
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
            Part::Legend(
                legend
                    .into_iter()
                    .flatten()
                    .map(|(dot, text)| (dot, text.into()))
                    .collect(),
            ),
        ],
    )
}

fn memory_tile(stats: &SysStats) -> Tile {
    let kib = |value: u64| value * 1024;
    let share = |used: u64, total: u64| used as f32 / total.max(1) as f32;
    let mut rows = vec![tile_meter(
        "RAM",
        share(stats.mem_used_kib, stats.mem_total_kib),
        format!(
            "{} / {}",
            size(kib(stats.mem_used_kib)),
            size(kib(stats.mem_total_kib))
        ),
        PEACH,
    )];
    if stats.swap_total_kib > 0 {
        rows.push(tile_meter(
            "Swap",
            share(stats.swap_used_kib, stats.swap_total_kib),
            format!(
                "{} / {}",
                size(kib(stats.swap_used_kib)),
                size(kib(stats.swap_total_kib))
            ),
            YELLOW,
        ));
    }
    titled(
        "tile-memory",
        1,
        "Memory",
        Some(stat(
            format!(
                "{:.0}%",
                share(stats.mem_used_kib, stats.mem_total_kib) * 100.0
            ),
            PEACH,
        )),
        vec![
            Part::Chart {
                series: vec![series(&stats.mem_history, 1.0, PEACH, "RAM", percent)],
                grow: true,
            },
            Part::Meters {
                rows,
                scroll: false,
            },
        ],
    )
}

fn processes_tile(stats: &SysStats) -> Tile {
    let busiest = stats
        .processes
        .iter()
        .map(|process| process.cpu)
        .fold(f32::EPSILON, f32::max);
    let rows = stats
        .processes
        .iter()
        .map(|process| {
            tile_meter(
                process.name.clone(),
                process.cpu / busiest,
                format!("{:.1}%", process.cpu * 100.0),
                PINK,
            )
        })
        .collect();
    titled(
        "tile-processes",
        1,
        "Processes",
        Some(stat(stats.process_count.to_string(), SUBTEXT1)),
        vec![
            Part::Spacer,
            Part::Meters {
                rows,
                scroll: false,
            },
        ],
    )
}

fn power_tile(stats: &SysStats) -> Tile {
    const COLORS: [u32; 3] = [YELLOW, TEAL, MAUVE];
    if stats.power.is_empty() {
        return titled(
            "tile-power",
            1,
            "Power",
            None,
            vec![
                Part::Spacer,
                Part::Legend(vec![(None, "No power sensors".into())]),
            ],
        );
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
                    .map(|(source, color)| {
                        series(&source.history, peak, color, source.label.clone(), watts)
                    })
                    .collect(),
                grow: true,
            },
            Part::Legend(
                stats
                    .power
                    .iter()
                    .zip(COLORS)
                    .map(|(source, color)| {
                        (
                            Some(color),
                            format!("{:.0} W {}", source.watts, source.label).into(),
                        )
                    })
                    .collect(),
            ),
        ],
    )
}

fn network_tile(stats: &SysStats) -> Tile {
    let peak = nice_peak(
        stats.rx_history.iter().chain(&stats.tx_history),
        64.0 * 1024.0,
    );
    titled(
        "tile-network",
        1,
        "Network",
        Some(stat(format!("↓ {}", rate(stats.rx_rate)), TEAL)),
        vec![
            Part::Chart {
                series: vec![
                    series(&stats.rx_history, peak, TEAL, "Download", |value| {
                        rate(value as f64)
                    }),
                    series(&stats.tx_history, peak, PEACH, "Upload", |value| {
                        rate(value as f64)
                    }),
                ],
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
    let peak = nice_peak(
        stats.read_history.iter().chain(&stats.write_history),
        1024.0 * 1024.0,
    );
    titled(
        "tile-disk",
        1,
        "Disk activity",
        Some(stat(format!("R {}", rate(stats.read_rate)), TEAL)),
        vec![
            Part::Chart {
                series: vec![
                    series(&stats.read_history, peak, TEAL, "Read", |value| {
                        rate(value as f64)
                    }),
                    series(&stats.write_history, peak, PEACH, "Write", |value| {
                        rate(value as f64)
                    }),
                ],
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
            && digits
                .chars()
                .all(|char| char.is_ascii_digit() || char == '.')
            && ["TB", "GB", "T", "G"].contains(&unit.to_uppercase().as_str())
    };
    model
        .split_whitespace()
        .filter(|word| !matches!(word.to_uppercase().as_str(), "SSD" | "NVME") && !capacity(word))
        .collect::<Vec<_>>()
        .join(" ")
}

fn storage_tile(stats: &SysStats, settings: &config::System) -> Tile {
    const COLORS: [u32; 5] = [BLUE, LAVENDER, TEAL, MAUVE, PEACH];
    let volumes: Vec<Volume> = settings
        .storage
        .shown_partitions(&stats.partitions, |partition| partition.mount.as_str())
        .into_iter()
        .map(|partition| {
            let fraction = partition.used as f32 / partition.total.max(1) as f32;
            Volume {
                id: partition.mount.clone().into(),
                mount: partition.mount.clone().into(),
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
                let free = drive
                    .size
                    .saturating_sub(drive.parts.iter().map(|part| part.size).sum());
                if free > drive.size / 100 {
                    segments.push(Segment {
                        weight: free as f32,
                        color: None,
                    });
                }
                let model = drive_model(&drive.model);
                Drive {
                    name: if model.is_empty() {
                        drive.name.clone().into()
                    } else {
                        model.into()
                    },
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
        on_click: None,
        parts: vec![
            Part::Hero {
                name: if info.hostname.is_empty() {
                    "This computer".into()
                } else {
                    info.hostname.clone().into()
                },
                lines: vec![
                    format!("{} · Linux {}", info.os, info.kernel).into(),
                    hardware.into(),
                ],
            },
            Part::Spacer,
            Part::Facts(facts),
        ],
    }
}

fn temperatures_tile(stats: &SysStats) -> Tile {
    let hottest = stats
        .sensors
        .iter()
        .map(|sensor| sensor.celsius)
        .fold(f32::MIN, f32::max);
    let histories = [
        (&stats.cpu_temp_history, MAUVE, "CPU"),
        (&stats.gpu_temp_history, PEACH, "GPU"),
        (&stats.storage_temp_history, TEAL, "NVMe"),
    ];
    let shown: Vec<_> = histories
        .into_iter()
        .filter(|(history, _, _)| !history.is_empty())
        .collect();
    titled(
        "tile-temperatures",
        2,
        "Temperatures",
        (hottest > f32::MIN).then(|| stat(format!("{hottest:.0}°C"), heat(hottest))),
        vec![
            Part::Chart {
                series: shown
                    .iter()
                    .map(|(history, color, label)| {
                        series(history, 100.0, *color, *label, temperature)
                    })
                    .collect(),
                grow: false,
            },
            Part::Legend(
                shown
                    .iter()
                    .map(|(_, color, label)| (Some(*color), (*label).into()))
                    .collect(),
            ),
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

pub fn system(
    stats: &SysStats,
    settings: &config::System,
    displays: usize,
    buttons: Vec<IconButton>,
) -> PanelView {
    PanelView {
        header: Header::Tabs,
        blocks: vec![
            Block::Buttons(ButtonRow {
                buttons,
                compact: false,
                indent: 0.0,
            }),
            Block::Board(vec![
                vec![
                    cpu_tile(stats),
                    memory_tile(stats),
                    processes_tile(stats),
                    power_tile(stats),
                ],
                vec![
                    gpu_tile(stats),
                    network_tile(stats),
                    storage_tile(stats, settings),
                ],
                vec![
                    host_tile(stats, displays),
                    temperatures_tile(stats),
                    disk_tile(stats),
                ],
            ]),
        ],
    }
}

pub fn panel<'a>(island: &'a Island, panel: Panel, cx: &gpui::App) -> PanelView {
    match panel {
        Panel::System => system(
            &island.stats,
            cx.global(),
            island.displays(),
            island.feature_buttons(cx),
        ),
        Panel::Feature(name) => island.feature_panel(name, cx).unwrap_or_else(empty),
        Panel::Popups => island.popups(cx).unwrap_or_else(empty),
    }
}
