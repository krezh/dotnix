use std::{collections::HashMap, env, fs, path::PathBuf, time::Instant};

use super::{
    sources::{
        HISTORY, Mount, Probe, busy, cpu_ghz, cpu_model, discrete_gpu, drives, first_dir,
        generation, gpu_name, hwmon_temp, mounts, os_name, parse_cpu_line, power_sources,
        process_ticks, push, rate, read_disks, read_net, read_temp, read_text, read_u64, sensors,
        statvfs, temp_input,
    },
    stats::{Gpu, Info, Partition, PowerSource, Process, Sensor, SysStats},
};

/// Clock ticks per second for /proc/<pid>/stat times; 100 on every Linux target island runs on.
const TICKS: f64 = 100.0;

const TOP_PROCESSES: usize = 5;

pub struct Sampler {
    pub(super) stats: SysStats,
    prev_cpu: Option<(u64, u64)>,
    prev_cores: Vec<(u64, u64)>,
    prev_net: Option<(u64, u64, Instant)>,
    prev_disk: Option<(u64, u64, Instant)>,
    prev_processes: HashMap<u32, u64>,
    prev_process_at: Option<Instant>,
    /// Program names by pid and start time, so each process's `exe` is read once.
    programs: HashMap<(u32, u64), String>,
    cpu_temp: Option<PathBuf>,
    gpu: Option<PathBuf>,
    gpu_hwmon: Option<PathBuf>,
    gpu_temp: Option<PathBuf>,
    storage_temp: Option<PathBuf>,
    sensors: Vec<Probe>,
    power: Vec<Probe>,
    mounts: Vec<Mount>,
    disks: Vec<String>,
}

impl Default for Sampler {
    fn default() -> Self {
        let gpu = discrete_gpu();
        let gpu_hwmon = gpu
            .as_deref()
            .and_then(|device| first_dir(&device.join("hwmon")));
        let gpu_label = gpu.as_deref().map_or_else(String::new, gpu_name);
        let sensors = sensors(gpu_hwmon.as_deref());
        let storage_temp = sensors
            .iter()
            .find(|probe| probe.label.starts_with("NVMe"))
            .map(|probe| probe.path.clone());
        let power = power_sources(gpu_hwmon.as_deref());
        let power_stats = power
            .iter()
            .map(|probe| PowerSource {
                label: probe.label.clone(),
                watts: 0.0,
                history: Vec::with_capacity(HISTORY),
            })
            .collect();
        let mounts = mounts();
        Self {
            stats: SysStats {
                info: Info {
                    hostname: read_text("/proc/sys/kernel/hostname"),
                    cpu: cpu_model(),
                    gpu: gpu_label,
                    os: os_name(),
                    kernel: read_text("/proc/sys/kernel/osrelease"),
                    session: env::var("XDG_CURRENT_DESKTOP")
                        .ok()
                        .and_then(|desktops| desktops.split(':').next().map(str::to_owned))
                        .filter(|desktop| !desktop.is_empty())
                        .unwrap_or_else(|| "Wayland".to_owned()),
                    generation: generation(),
                },
                threads: std::thread::available_parallelism().map_or(0, usize::from),
                gpu: gpu.as_ref().map(|_| Gpu::default()),
                cpu_history: Vec::with_capacity(HISTORY),
                gpu_history: Vec::with_capacity(HISTORY),
                rx_history: Vec::with_capacity(HISTORY),
                tx_history: Vec::with_capacity(HISTORY),
                read_history: Vec::with_capacity(HISTORY),
                write_history: Vec::with_capacity(HISTORY),
                cpu_temp_history: Vec::with_capacity(HISTORY),
                gpu_temp_history: Vec::with_capacity(HISTORY),
                storage_temp_history: Vec::with_capacity(HISTORY),
                mem_history: Vec::with_capacity(HISTORY),
                power: power_stats,
                ..SysStats::default()
            },
            prev_cpu: None,
            prev_cores: Vec::new(),
            prev_net: None,
            prev_disk: None,
            prev_processes: HashMap::new(),
            prev_process_at: None,
            programs: HashMap::new(),
            cpu_temp: hwmon_temp("k10temp", "Tctl")
                .or_else(|| hwmon_temp("coretemp", "Package id 0")),
            gpu_temp: gpu_hwmon.as_deref().and_then(|dir| temp_input(dir, "edge")),
            power,
            gpu_hwmon,
            gpu,
            storage_temp,
            sensors,
            disks: drives(&[]).into_iter().map(|drive| drive.name).collect(),
            mounts,
        }
    }
}

impl Sampler {
    /// Measures each program's CPU share since the last scan, and makes this the next one's base.
    /// `None` on the first scan, which has nothing to measure from.
    pub(super) fn scan_processes(&mut self) -> (Option<Vec<Process>>, usize) {
        let now = Instant::now();
        let threads = self.stats.threads.max(1);
        let (ticks, count) = process_ticks(&mut self.programs);
        let top = self.prev_process_at.map(|at| {
            let seconds = now.duration_since(at).as_secs_f64().max(f64::EPSILON);
            let mut by_name: HashMap<&str, u64> = HashMap::new();
            for (pid, (name, used)) in &ticks {
                let before = self.prev_processes.get(pid).copied().unwrap_or(*used);
                *by_name.entry(name.as_str()).or_default() += used.saturating_sub(before);
            }
            let mut top: Vec<Process> = by_name
                .into_iter()
                .filter(|(_, delta)| *delta > 0)
                .map(|(name, delta)| Process {
                    name: name.to_owned(),
                    cpu: (delta as f64 / TICKS / seconds / threads as f64) as f32,
                })
                .collect();
            top.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
            top.truncate(TOP_PROCESSES);
            top
        });
        self.prev_processes = ticks
            .into_iter()
            .map(|(pid, (_, used))| (pid, used))
            .collect();
        self.prev_process_at = Some(now);
        (top, count)
    }

    pub(super) fn look(&mut self) {
        let partitions: Vec<Partition> = self
            .mounts
            .iter()
            .filter_map(|mount| {
                let (used, total) = statvfs(&mount.mount)?;
                Some(Partition {
                    mount: mount.mount.clone(),
                    device: mount.device.clone(),
                    used,
                    total,
                })
            })
            .collect();
        let (processes, process_count) = self.scan_processes();
        self.stats.cpu_ghz = cpu_ghz();
        self.stats.drives = drives(&partitions);
        self.stats.partitions = partitions;
        self.stats.sensors = self
            .sensors
            .iter()
            .filter_map(|probe| {
                Some(Sensor {
                    label: probe.label.clone(),
                    celsius: read_temp(&probe.path)?,
                })
            })
            .collect();
        if let Some(processes) = processes {
            self.stats.processes = processes;
        }
        self.stats.process_count = process_count;
    }

    /// Point-in-time readings that have no history chart, taken only when the panel is open.
    pub(super) fn read_instant(&mut self) {
        if let Ok(stat) = fs::read_to_string("/proc/stat") {
            self.update_cores(stat.lines().skip(1));
        }
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            self.update_mem(&meminfo, true);
        }
        self.update_gpu_instant();
        if let Ok(loadavg) = fs::read_to_string("/proc/loadavg") {
            for (slot, value) in self.stats.load.iter_mut().zip(loadavg.split_whitespace()) {
                *slot = value.parse().unwrap_or(0.0);
            }
        }
        if let Ok(uptime) = fs::read_to_string("/proc/uptime") {
            self.stats.uptime_secs = uptime
                .split_whitespace()
                .next()
                .and_then(|secs| secs.parse::<f64>().ok())
                .map_or(0, |secs| secs as u64);
        }
    }

    fn update_cores<'a>(&mut self, lines: impl Iterator<Item = &'a str>) {
        let cores: Vec<(u64, u64)> = lines
            .take_while(|line| line.starts_with("cpu"))
            .filter_map(parse_cpu_line)
            .collect();
        self.stats.cores.clear();
        self.stats.cores.extend(
            cores
                .iter()
                .zip(
                    self.prev_cores
                        .iter()
                        .copied()
                        .chain(std::iter::repeat((0, 0))),
                )
                .map(|(core, prev)| {
                    if prev.1 == 0 {
                        0.0
                    } else {
                        busy(*core, prev).unwrap_or(0.0)
                    }
                }),
        );
        self.prev_cores = cores;
    }

    fn update_mem(&mut self, meminfo: &str, watched: bool) {
        let (mut mem_total, mut mem_avail, mut swap_total, mut swap_free) =
            (None, None, None, None);
        for line in meminfo.lines() {
            let parse = |rest: &str| rest.split_whitespace().next()?.parse::<u64>().ok();
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                mem_total = parse(rest);
            } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                mem_avail = parse(rest);
                if !watched && mem_total.is_some() {
                    break;
                }
            } else if watched && let Some(rest) = line.strip_prefix("SwapTotal:") {
                swap_total = parse(rest);
            } else if watched && let Some(rest) = line.strip_prefix("SwapFree:") {
                swap_free = parse(rest);
            }
            if mem_total.is_some()
                && mem_avail.is_some()
                && (!watched || (swap_total.is_some() && swap_free.is_some()))
            {
                break;
            }
        }
        if let (Some(total), Some(available)) = (mem_total, mem_avail) {
            self.stats.mem_total_kib = total;
            self.stats.mem_used_kib = total.saturating_sub(available);
        }
        if let (Some(total), Some(free)) = (swap_total, swap_free) {
            self.stats.swap_total_kib = total;
            self.stats.swap_used_kib = total.saturating_sub(free);
        }
    }

    fn update_gpu_instant(&mut self) {
        let (Some(device), Some(gpu)) = (&self.gpu, &mut self.stats.gpu) else {
            return;
        };
        gpu.vram_used = read_u64(&device.join("mem_info_vram_used")).unwrap_or(0);
        gpu.vram_total = read_u64(&device.join("mem_info_vram_total")).unwrap_or(0);
        gpu.fan_rpm = self
            .gpu_hwmon
            .as_deref()
            .and_then(|dir| read_u64(&dir.join("fan1_input")))
            .and_then(|rpm| u32::try_from(rpm).ok());
    }

    /// Updates the history charts, and when `watched` also reads what only the open panel shows.
    pub fn sample(&mut self, watched: bool) {
        let now = Instant::now();

        if let Ok(stat) = fs::read_to_string("/proc/stat") {
            let mut lines = stat.lines();
            if let Some(total) = lines.next().and_then(parse_cpu_line) {
                if let Some(cpu) = self.prev_cpu.and_then(|prev| busy(total, prev)) {
                    self.stats.cpu = cpu;
                    push(&mut self.stats.cpu_history, cpu);
                }
                self.prev_cpu = Some(total);
            }
            if watched {
                self.update_cores(lines);
            }
        }
        self.stats.cpu_temp = self.cpu_temp.as_deref().and_then(read_temp);
        if let Some(temp) = self.stats.cpu_temp {
            push(&mut self.stats.cpu_temp_history, temp);
        }

        for (probe, source) in self.power.iter().zip(&mut self.stats.power) {
            if let Some(microwatts) = read_u64(&probe.path) {
                let watts = microwatts as f32 / 1_000_000.0;
                source.watts = watts;
                push(&mut source.history, watts);
            }
        }

        if let (Some(device), Some(gpu)) = (&self.gpu, &mut self.stats.gpu) {
            gpu.busy = read_u64(&device.join("gpu_busy_percent")).unwrap_or(0) as f32 / 100.0;
            gpu.temp = self.gpu_temp.as_deref().and_then(read_temp);
            gpu.power = self
                .stats
                .power
                .iter()
                .find(|source| source.label == "GPU")
                .map(|source| source.watts);
            push(&mut self.stats.gpu_history, gpu.busy);
            if let Some(temp) = gpu.temp {
                push(&mut self.stats.gpu_temp_history, temp);
            }
            if watched {
                self.update_gpu_instant();
            }
        }

        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            self.update_mem(&meminfo, watched);
        }
        if self.stats.mem_total_kib > 0 {
            push(
                &mut self.stats.mem_history,
                self.stats.mem_used_kib as f32 / self.stats.mem_total_kib as f32,
            );
        }

        if let Some((rx, tx)) = read_net() {
            let sample = (rx, tx, now);
            if let Some((rx_rate, tx_rate)) = rate(sample, self.prev_net) {
                self.stats.rx_rate = rx_rate;
                self.stats.tx_rate = tx_rate;
                push(&mut self.stats.rx_history, rx_rate as f32);
                push(&mut self.stats.tx_history, tx_rate as f32);
            }
            self.prev_net = Some(sample);
        }
        if let Some((read, written)) = read_disks(&self.disks) {
            let sample = (read, written, now);
            if let Some((read_rate, write_rate)) = rate(sample, self.prev_disk) {
                self.stats.read_rate = read_rate;
                self.stats.write_rate = write_rate;
                push(&mut self.stats.read_history, read_rate as f32);
                push(&mut self.stats.write_history, write_rate as f32);
            }
            self.prev_disk = Some(sample);
        }

        if let Some(temp) = self.storage_temp.as_deref().and_then(read_temp) {
            push(&mut self.stats.storage_temp_history, temp);
        }

        if watched {
            self.look();
            if let Ok(loadavg) = fs::read_to_string("/proc/loadavg") {
                for (slot, value) in self.stats.load.iter_mut().zip(loadavg.split_whitespace()) {
                    *slot = value.parse().unwrap_or(0.0);
                }
            }
            if let Ok(uptime) = fs::read_to_string("/proc/uptime") {
                self.stats.uptime_secs = uptime
                    .split_whitespace()
                    .next()
                    .and_then(|secs| secs.parse::<f64>().ok())
                    .map_or(0, |secs| secs as u64);
            }
        }
    }
}
