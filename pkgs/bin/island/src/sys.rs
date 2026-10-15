use std::{
    collections::{HashMap, VecDeque},
    env,
    ffi::CString,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::supervisor::{self, Ended};

#[derive(Clone)]
pub struct Samples(Arc<Mutex<(SysStats, Instant)>>);

impl Samples {
    pub fn latest(&self) -> (SysStats, Instant) {
        self.0
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| (SysStats::default(), Instant::now()))
    }
}

pub fn watch() -> Samples {
    let mut sampler = Sampler::default();
    let shared = Arc::new(Mutex::new((sampler.sample(), Instant::now())));
    let writer = shared.clone();
    let spawned = supervisor::supervise("sys", move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            let stats = sampler.sample();
            match writer.lock() {
                Ok(mut guard) => *guard = (stats, Instant::now()),
                Err(_) => return Ended::Unwanted,
            }
        }
    });
    if let Err(error) = spawned {
        tracing::error!("system sampler failed to start: {error}");
    }
    Samples(shared)
}

const HISTORY: usize = 40;
const TOP_PROCESSES: usize = 5;
/// Clock ticks per second for /proc/<pid>/stat times; 100 on every Linux target island runs on.
const TICKS: f64 = 100.0;
const SECTOR: u64 = 512;

#[derive(Clone, Copy, Debug, Default)]
pub struct Gpu {
    pub busy: f32,
    pub temp: Option<f32>,
    pub vram_used: u64,
    pub vram_total: u64,
    pub power: Option<f32>,
    pub fan_rpm: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct Partition {
    pub mount: String,
    /// The block device behind the mount, e.g. `nvme0n1p2`.
    pub device: String,
    pub fs: String,
    pub used: u64,
    pub total: u64,
}

#[derive(Clone, Debug, Default)]
pub struct DrivePart {
    pub size: u64,
    /// Where the partition is mounted, if it is one of `SysStats::partitions`.
    pub mount: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Drive {
    pub name: String,
    pub model: String,
    pub size: u64,
    pub parts: Vec<DrivePart>,
}

#[derive(Clone, Debug, Default)]
pub struct Sensor {
    pub label: String,
    pub celsius: f32,
}

#[derive(Clone, Debug, Default)]
pub struct PowerSource {
    pub label: String,
    pub watts: f32,
    pub history: Vec<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct Process {
    pub name: String,
    /// Share of the whole CPU: 1.0 is every thread fully busy.
    pub cpu: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Info {
    pub hostname: String,
    pub cpu: String,
    pub gpu: String,
    pub os: String,
    pub kernel: String,
    pub session: String,
    pub generation: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct SysStats {
    pub info: Info,
    pub cpu: f32,
    pub cpu_history: Vec<f32>,
    pub cores: Vec<f32>,
    pub cpu_ghz: Option<f32>,
    pub cpu_temp: Option<f32>,
    pub cpu_temp_history: Vec<f32>,
    pub threads: usize,
    pub gpu: Option<Gpu>,
    pub gpu_history: Vec<f32>,
    pub gpu_temp_history: Vec<f32>,
    pub mem_used_kib: u64,
    pub mem_total_kib: u64,
    pub mem_history: Vec<f32>,
    pub swap_used_kib: u64,
    pub swap_total_kib: u64,
    pub rx_rate: f64,
    pub tx_rate: f64,
    pub rx_history: Vec<f32>,
    pub tx_history: Vec<f32>,
    pub read_rate: f64,
    pub write_rate: f64,
    pub read_history: Vec<f32>,
    pub write_history: Vec<f32>,
    pub partitions: Vec<Partition>,
    pub drives: Vec<Drive>,
    pub sensors: Vec<Sensor>,
    pub storage_temp_history: Vec<f32>,
    pub power: Vec<PowerSource>,
    pub processes: Vec<Process>,
    pub process_count: usize,
    pub load: [f32; 3],
    pub uptime_secs: u64,
}

struct Probe {
    label: String,
    path: PathBuf,
    rank: u32,
}

struct Mount {
    mount: String,
    device: String,
    fs: String,
}

pub struct Sampler {
    prev_cpu: Option<(u64, u64)>,
    prev_cores: Vec<(u64, u64)>,
    prev_net: Option<(u64, u64, Instant)>,
    prev_disk: Option<(u64, u64, Instant)>,
    prev_processes: HashMap<u32, u64>,
    prev_process_at: Option<Instant>,
    cpu_temp: Option<PathBuf>,
    gpu: Option<PathBuf>,
    gpu_hwmon: Option<PathBuf>,
    gpu_temp: Option<PathBuf>,
    storage_temp: Option<PathBuf>,
    sensors: Vec<Probe>,
    power: Vec<(Probe, VecDeque<f32>)>,
    cpu_history: VecDeque<f32>,
    gpu_history: VecDeque<f32>,
    rx_history: VecDeque<f32>,
    tx_history: VecDeque<f32>,
    read_history: VecDeque<f32>,
    write_history: VecDeque<f32>,
    cpu_temp_history: VecDeque<f32>,
    gpu_temp_history: VecDeque<f32>,
    storage_temp_history: VecDeque<f32>,
    mem_history: VecDeque<f32>,
    mounts: Vec<Mount>,
    disks: Vec<String>,
    info: Info,
}

impl Default for Sampler {
    fn default() -> Self {
        let gpu = discrete_gpu();
        let gpu_hwmon = gpu.as_deref().and_then(|device| first_dir(&device.join("hwmon")));
        let gpu_label = gpu.as_deref().map_or_else(String::new, gpu_name);
        let sensors = sensors(gpu_hwmon.as_deref());
        let storage_temp = sensors
            .iter()
            .find(|probe| probe.label.starts_with("NVMe"))
            .map(|probe| probe.path.clone());
        let mounts = mounts();
        Self {
            prev_cpu: None,
            prev_cores: Vec::new(),
            prev_net: None,
            prev_disk: None,
            prev_processes: HashMap::new(),
            prev_process_at: None,
            cpu_temp: hwmon_temp("k10temp", "Tctl").or_else(|| hwmon_temp("coretemp", "Package id 0")),
            gpu_temp: gpu_hwmon.as_deref().and_then(|dir| temp_input(dir, "edge")),
            power: power_sources(gpu_hwmon.as_deref())
                .into_iter()
                .map(|probe| (probe, VecDeque::with_capacity(HISTORY)))
                .collect(),
            gpu_hwmon,
            gpu,
            storage_temp,
            sensors,
            cpu_history: VecDeque::with_capacity(HISTORY),
            gpu_history: VecDeque::with_capacity(HISTORY),
            rx_history: VecDeque::with_capacity(HISTORY),
            tx_history: VecDeque::with_capacity(HISTORY),
            read_history: VecDeque::with_capacity(HISTORY),
            write_history: VecDeque::with_capacity(HISTORY),
            cpu_temp_history: VecDeque::with_capacity(HISTORY),
            gpu_temp_history: VecDeque::with_capacity(HISTORY),
            storage_temp_history: VecDeque::with_capacity(HISTORY),
            mem_history: VecDeque::with_capacity(HISTORY),
            disks: drives(&[]).into_iter().map(|drive| drive.name).collect(),
            mounts,
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
        }
    }
}

fn read_text(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).map(|text| text.trim().to_owned()).unwrap_or_default()
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_temp(path: &Path) -> Option<f32> {
    read_u64(path).map(|millidegrees| millidegrees as f32 / 1000.0)
}

fn first_dir(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir).ok()?.flatten().map(|entry| entry.path()).next()
}

fn temp_input(dir: &Path, label: &str) -> Option<PathBuf> {
    (1..=16).find_map(|index| {
        let found = fs::read_to_string(dir.join(format!("temp{index}_label")))
            .is_ok_and(|text| text.trim() == label);
        found.then(|| dir.join(format!("temp{index}_input")))
    })
}

fn hwmon_temp(name: &str, label: &str) -> Option<PathBuf> {
    fs::read_dir("/sys/class/hwmon").ok()?.flatten().find_map(|hwmon| {
        let dir = hwmon.path();
        fs::read_to_string(dir.join("name"))
            .is_ok_and(|text| text.trim() == name)
            .then(|| temp_input(&dir, label))?
    })
}

fn hwmons() -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = fs::read_dir("/sys/class/hwmon")
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .map(|dir| (read_text(dir.join("name")), dir))
        .collect();
    found.sort_by(|a, b| a.1.cmp(&b.1));
    found
}

/// Every temperature worth showing, named for people rather than drivers, in a stable order.
fn sensors(gpu_hwmon: Option<&Path>) -> Vec<Probe> {
    let mut probes = Vec::new();
    let (mut nvme, mut ram) = (0, 0);
    let gpu_hwmon = gpu_hwmon.and_then(|dir| dir.canonicalize().ok());
    for (name, dir) in hwmons() {
        let discrete = gpu_hwmon.as_deref().is_some_and(|gpu| dir.canonicalize().ok().as_deref() == Some(gpu));
        let mut add = |label: String, input: Option<PathBuf>, rank: u32| {
            if let Some(path) = input.filter(|path| path.exists()) {
                probes.push(Probe { label, path, rank });
            }
        };
        match name.as_str() {
            "k10temp" => {
                add("CPU".into(), temp_input(&dir, "Tctl"), 0);
                for ccd in 1..=8 {
                    add(format!("CCD {ccd}"), temp_input(&dir, &format!("Tccd{ccd}")), ccd);
                }
            }
            "coretemp" => add("CPU".into(), temp_input(&dir, "Package id 0"), 0),
            "amdgpu" if discrete => {
                add("GPU".into(), temp_input(&dir, "edge"), 20);
                add("Hotspot".into(), temp_input(&dir, "junction"), 21);
                add("GPU mem".into(), temp_input(&dir, "mem"), 22);
            }
            "amdgpu" => add("iGPU".into(), temp_input(&dir, "edge").or_else(|| Some(dir.join("temp1_input"))), 23),
            "nvme" => {
                nvme += 1;
                add(
                    format!("NVMe {nvme}"),
                    temp_input(&dir, "Composite").or_else(|| Some(dir.join("temp1_input"))),
                    30 + nvme,
                );
            }
            "spd5118" | "jc42" => {
                ram += 1;
                add(format!("RAM {ram}"), Some(dir.join("temp1_input")), 50 + ram);
            }
            name if name.contains("prom") => add("Chipset".into(), Some(dir.join("temp1_input")), 70),
            name if name.starts_with("iwlwifi") || name.starts_with("mt79") || name.starts_with("ath") => {
                add("Wi-Fi".into(), Some(dir.join("temp1_input")), 80);
            }
            name if ["r8169", "igc", "e1000", "atlantic", "ixgbe", "bnxt"].iter().any(|nic| name.starts_with(nic)) => {
                add("Network".into(), Some(dir.join("temp1_input")), 79);
            }
            _ => {}
        }
    }
    probes.sort_by_key(|probe| probe.rank);
    probes
}

/// Power draw in watts from GPU sensors: the discrete card first, then any integrated one.
fn power_sources(gpu_hwmon: Option<&Path>) -> Vec<Probe> {
    let gpu_hwmon = gpu_hwmon.and_then(|dir| dir.canonicalize().ok());
    let mut sources: Vec<Probe> = hwmons()
        .into_iter()
        .filter(|(name, _)| name == "amdgpu")
        .filter_map(|(_, dir)| {
            let discrete = gpu_hwmon.as_deref().is_some_and(|gpu| dir.canonicalize().ok().as_deref() == Some(gpu));
            let path = ["power1_average", "power1_input"]
                .into_iter()
                .map(|file| dir.join(file))
                .find(|path| path.exists())?;
            Some(Probe {
                label: if discrete { "GPU".into() } else { "iGPU".into() },
                path,
                rank: u32::from(!discrete),
            })
        })
        .collect();
    sources.sort_by_key(|probe| probe.rank);
    sources
}

fn discrete_gpu() -> Option<PathBuf> {
    fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .map(|card| card.path().join("device"))
        .filter(|device| device.join("gpu_busy_percent").exists())
        .max_by_key(|device| read_u64(&device.join("mem_info_vram_total")).unwrap_or(0))
}

/// Idle and total jiffies for the whole CPU and for each core, from /proc/stat.
fn read_cpu_times() -> Option<((u64, u64), Vec<(u64, u64)>)> {
    let stat = fs::read_to_string("/proc/stat").ok()?;
    let times = |line: &str| -> Option<(u64, u64)> {
        let fields: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .take(8)
            .filter_map(|field| field.parse().ok())
            .collect();
        (fields.len() >= 5).then(|| (fields[3] + fields[4], fields.iter().sum()))
    };
    let mut lines = stat.lines();
    let total = times(lines.next()?)?;
    let cores = lines
        .take_while(|line| line.starts_with("cpu"))
        .filter_map(times)
        .collect();
    Some((total, cores))
}

fn busy((idle, total): (u64, u64), (prev_idle, prev_total): (u64, u64)) -> Option<f32> {
    let d_total = total.saturating_sub(prev_total);
    (d_total > 0).then(|| 1.0 - idle.saturating_sub(prev_idle) as f32 / d_total as f32)
}

fn cpu_ghz() -> Option<f32> {
    let khz: Vec<u64> = fs::read_dir("/sys/devices/system/cpu")
        .ok()?
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("cpu"))
                .is_some_and(|rest| rest.chars().all(|char| char.is_ascii_digit()) && !rest.is_empty())
        })
        .filter_map(|entry| read_u64(&entry.path().join("cpufreq/scaling_cur_freq")))
        .collect();
    (!khz.is_empty()).then(|| khz.iter().sum::<u64>() as f32 / khz.len() as f32 / 1_000_000.0)
}

fn read_net() -> Option<(u64, u64)> {
    let dev = fs::read_to_string("/proc/net/dev").ok()?;
    let mut totals = (0u64, 0u64);
    for line in dev.lines().skip(2) {
        let Some((name, counters)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name == "lo" || name.starts_with("docker") || name.starts_with("veth") || name.starts_with("br-") {
            continue;
        }
        let fields: Vec<u64> = counters
            .split_whitespace()
            .filter_map(|field| field.parse().ok())
            .collect();
        if fields.len() >= 9 {
            totals.0 += fields[0];
            totals.1 += fields[8];
        }
    }
    Some(totals)
}

/// Bytes read and written by the physical drives (not their partitions), from /proc/diskstats.
fn read_disks(disks: &[String]) -> Option<(u64, u64)> {
    let stats = fs::read_to_string("/proc/diskstats").ok()?;
    let mut totals = (0u64, 0u64);
    for line in stats.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 || !disks.iter().any(|disk| disk == fields[2]) {
            continue;
        }
        totals.0 += fields[5].parse::<u64>().unwrap_or(0) * SECTOR;
        totals.1 += fields[9].parse::<u64>().unwrap_or(0) * SECTOR;
    }
    Some(totals)
}

fn statvfs(path: &str) -> Option<(u64, u64)> {
    let path = CString::new(path).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(path.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    let block = stat.f_frsize as u64;
    let total = stat.f_blocks as u64 * block;
    let available = stat.f_bavail as u64 * block;
    Some((total.saturating_sub(available), total))
}

fn cpu_model() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|info| {
            info.lines()
                .find_map(|line| line.strip_prefix("model name"))
                .and_then(|rest| rest.split_once(':'))
                .map(|(_, model)| {
                    model
                        .split_whitespace()
                        .filter(|word| !word.ends_with("-Core") && *word != "Processor" && *word != "AMD")
                        .collect::<Vec<_>>()
                        .join(" ")
                })
        })
        .unwrap_or_default()
}

/// The bracketed marketing name in a pci.ids entry ("Navi 48 [Radeon RX 9070 XT]" → "Radeon RX 9070 XT"),
/// without a trailing memory size; a family list keeps its first member.
fn marketing(entry: &str) -> String {
    let name = entry.split(['[', ']']).nth(1).unwrap_or(entry);
    let name = name.split('/').next().unwrap_or(name).trim();
    match name.rsplit_once(' ') {
        Some((model, size)) if size.ends_with("GB") && size[..size.len() - 2].chars().all(|char| char.is_ascii_digit()) => {
            model.to_owned()
        }
        _ => name.to_owned(),
    }
}

/// Looks a device up in pci.ids text: the board under its subsystem entry if listed, else the chip.
/// Comment lines (`#`, unindented even inside a vendor's block) are skipped.
fn pci_name(database: &str, vendor: &str, product: &str, subsystem: &str) -> Option<String> {
    let mut lines = database
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .skip_while(|line| !line.starts_with(&format!("{vendor} ")))
        .skip(1)
        .take_while(|line| line.starts_with('\t'));
    let family = lines.find_map(|line| line.strip_prefix(&format!("\t{product} ")))?;
    let board = lines
        .take_while(|line| line.starts_with("\t\t"))
        .find_map(|line| line.strip_prefix(&format!("\t\t{subsystem} ")));
    Some(marketing(board.unwrap_or(family).trim()))
}

/// The card's name from the PCI database: the exact board from its subsystem entry
/// (e.g. "Radeon RX 9070 XT"), else the chip's family, else just the vendor.
fn gpu_name(device: &Path) -> String {
    let id = |file: &str| read_text(device.join(file)).trim_start_matches("0x").to_lowercase();
    let (vendor, product) = (id("vendor"), id("device"));
    let subsystem = format!("{} {}", id("subsystem_vendor"), id("subsystem_device"));
    let named = option_env!("ISLAND_PCI_IDS")
        .into_iter()
        .chain(["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"])
        .find_map(|path| fs::read_to_string(path).ok())
        .and_then(|database| pci_name(&database, &vendor, &product, &subsystem));
    named.unwrap_or_else(|| match vendor.as_str() {
        "1002" => "AMD GPU".to_owned(),
        "10de" => "NVIDIA GPU".to_owned(),
        "8086" => "Intel GPU".to_owned(),
        _ => "GPU".to_owned(),
    })
}

fn os_name() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|release| {
            let field = |key: &str| {
                release.lines().find_map(|line| {
                    line.strip_prefix(key).map(|value| value.trim_matches('"').to_owned())
                })
            };
            match (field("NAME="), field("VERSION_ID=")) {
                (Some(name), Some(version)) => Some(format!("{name} {version}")),
                _ => field("PRETTY_NAME="),
            }
        })
        .unwrap_or_else(|| "Linux".to_owned())
}

/// NixOS system generation number, from the `system-<n>-link` the profile points at.
fn generation() -> Option<u32> {
    let link = fs::read_link("/nix/var/nix/profiles/system").ok()?;
    link.file_name()?
        .to_str()?
        .strip_prefix("system-")?
        .strip_suffix("-link")?
        .parse()
        .ok()
}

/// The partition behind a device path, following device-mapper (LUKS, LVM) to what it sits on.
fn backing_partition(device: &str) -> String {
    let resolved = Path::new(device).canonicalize().unwrap_or_else(|_| PathBuf::from(device));
    let mut name = resolved
        .file_name()
        .map_or_else(|| device.to_owned(), |name| name.to_string_lossy().into_owned());
    for _ in 0..4 {
        if !name.starts_with("dm-") {
            break;
        }
        match first_dir(&Path::new("/sys/block").join(&name).join("slaves")) {
            Some(slave) => name = slave.file_name().map_or(name, |slave| slave.to_string_lossy().into_owned()),
            None => break,
        }
    }
    name
}

fn mounts() -> Vec<Mount> {
    const TYPES: [&str; 7] = ["ext4", "btrfs", "xfs", "f2fs", "zfs", "bcachefs", "vfat"];
    // The name a partition is shown under when it is mounted in several places; with an
    // impermanent root, /persist is what the partition is for.
    let rank = |mount: &str| match mount {
        "/" => 0,
        "/persist" => 1,
        "/nix" => 2,
        "/home" => 3,
        "/boot" => 1000,
        other => 4 + other.len(),
    };
    let mut best: Vec<(String, Mount)> = Vec::new();
    for line in fs::read_to_string("/proc/mounts").unwrap_or_default().lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [device, mount, kind, ..] = fields[..] else {
            continue;
        };
        if !TYPES.contains(&kind) || !device.starts_with("/dev/") {
            continue;
        }
        let mount = mount.replace("\\040", " ");
        match best.iter_mut().find(|(known, _)| known == device) {
            Some((_, current)) if rank(&mount) < rank(&current.mount) => current.mount = mount,
            Some(_) => {}
            None => best.push((
                device.to_owned(),
                Mount {
                    mount,
                    device: backing_partition(device),
                    fs: kind.to_owned(),
                },
            )),
        }
    }
    let mut mounts: Vec<Mount> = best.into_iter().map(|(_, mount)| mount).collect();
    mounts.sort_by_key(|mount| (rank(&mount.mount), mount.mount.clone()));
    mounts
}

/// Physical drives with their partitions; partitions that are mounted carry their mount point.
fn drives(partitions: &[Partition]) -> Vec<Drive> {
    let skip = ["loop", "ram", "zram", "dm-", "sr", "md", "nbd"];
    let mut drives: Vec<Drive> = fs::read_dir("/sys/block")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if skip.iter().any(|prefix| name.starts_with(prefix)) {
                return None;
            }
            let dir = entry.path();
            let size = read_u64(&dir.join("size")).unwrap_or(0) * SECTOR;
            if size == 0 {
                return None;
            }
            let mut parts: Vec<(String, DrivePart)> = fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|part| part.path().join("partition").exists())
                .map(|part| {
                    let part_name = part.file_name().to_string_lossy().into_owned();
                    let mount = partitions
                        .iter()
                        .find(|partition| partition.device == part_name)
                        .map(|partition| partition.mount.clone());
                    (
                        part_name,
                        DrivePart {
                            size: read_u64(&part.path().join("size")).unwrap_or(0) * SECTOR,
                            mount,
                        },
                    )
                })
                .collect();
            parts.sort_by(|a, b| a.0.cmp(&b.0));
            Some(Drive {
                model: read_text(dir.join("device/model")),
                name,
                size,
                parts: parts.into_iter().map(|(_, part)| part).collect(),
            })
        })
        .collect();
    drives.sort_by(|a, b| a.name.cmp(&b.name));
    drives
}

/// A program name without Nix wrapper decoration: `.zen-twilight-wrapped` and the kernel's
/// 15-byte `.Hyprland-wrapp` both become their plain names.
fn program_name(raw: &str) -> String {
    let name = raw.strip_prefix('.').unwrap_or(raw);
    let name = match name.rfind("-wrap") {
        Some(at) if raw.starts_with('.') || name[at..] == *"-wrapped" => &name[..at],
        _ => name,
    };
    name.to_owned()
}

/// Which program a process belongs to: its executable, so a browser's content processes
/// ("Isolated Web Co", "Web Content") count as the browser; the short name when that is unreadable.
fn program(pid_dir: &Path, comm: &str) -> String {
    fs::read_link(pid_dir.join("exe"))
        .ok()
        .and_then(|exe| exe.file_name().map(|name| name.to_string_lossy().trim_end_matches(" (deleted)").to_owned()))
        .map_or_else(|| program_name(comm), |exe| program_name(&exe))
}

/// CPU ticks per process, labelled by program, so a browser's many processes count as one.
fn process_ticks() -> (HashMap<u32, (String, u64)>, usize) {
    let mut ticks = HashMap::new();
    let mut count = 0;
    for entry in fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        count += 1;
        let (Some(open), Some(close)) = (stat.find('('), stat.rfind(')')) else {
            continue;
        };
        let name = program(&entry.path(), &stat[open + 1..close]);
        let fields: Vec<&str> = stat[close + 1..].split_whitespace().collect();
        let used = fields.get(11).and_then(|value| value.parse::<u64>().ok()).unwrap_or(0)
            + fields.get(12).and_then(|value| value.parse::<u64>().ok()).unwrap_or(0);
        ticks.insert(pid, (name, used));
    }
    (ticks, count)
}

fn push(history: &mut VecDeque<f32>, value: f32) {
    if history.len() == HISTORY {
        history.pop_front();
    }
    history.push_back(value);
}

fn rate(now: (u64, u64, Instant), prev: Option<(u64, u64, Instant)>) -> Option<(f64, f64)> {
    let (prev_a, prev_b, at) = prev?;
    let seconds = now.2.duration_since(at).as_secs_f64();
    (seconds > 0.0).then(|| {
        (
            now.0.saturating_sub(prev_a) as f64 / seconds,
            now.1.saturating_sub(prev_b) as f64 / seconds,
        )
    })
}

impl Sampler {
    pub fn sample(&mut self) -> SysStats {
        let now = Instant::now();
        let mut stats = SysStats {
            threads: std::thread::available_parallelism().map_or(0, usize::from),
            ..SysStats::default()
        };

        if let Some((total, cores)) = read_cpu_times() {
            if let Some(cpu) = self.prev_cpu.and_then(|prev| busy(total, prev)) {
                stats.cpu = cpu;
                push(&mut self.cpu_history, cpu);
            }
            stats.cores = cores
                .iter()
                .zip(self.prev_cores.iter().copied().chain(std::iter::repeat((0, 0))))
                .map(|(core, prev)| if prev.1 == 0 { 0.0 } else { busy(*core, prev).unwrap_or(0.0) })
                .collect();
            self.prev_cpu = Some(total);
            self.prev_cores = cores;
        }
        stats.cpu_ghz = cpu_ghz();
        stats.cpu_temp = self.cpu_temp.as_deref().and_then(read_temp);
        if let Some(temp) = stats.cpu_temp {
            push(&mut self.cpu_temp_history, temp);
        }

        if let Some(device) = &self.gpu {
            let hwmon = self.gpu_hwmon.as_deref();
            let gpu = Gpu {
                busy: read_u64(&device.join("gpu_busy_percent")).unwrap_or(0) as f32 / 100.0,
                temp: self.gpu_temp.as_deref().and_then(read_temp),
                vram_used: read_u64(&device.join("mem_info_vram_used")).unwrap_or(0),
                vram_total: read_u64(&device.join("mem_info_vram_total")).unwrap_or(0),
                power: self
                    .power
                    .iter()
                    .find(|(probe, _)| probe.label == "GPU")
                    .and_then(|(probe, _)| read_u64(&probe.path))
                    .map(|microwatts| microwatts as f32 / 1_000_000.0),
                fan_rpm: hwmon
                    .and_then(|dir| read_u64(&dir.join("fan1_input")))
                    .and_then(|rpm| u32::try_from(rpm).ok()),
            };
            push(&mut self.gpu_history, gpu.busy);
            if let Some(temp) = gpu.temp {
                push(&mut self.gpu_temp_history, temp);
            }
            stats.gpu = Some(gpu);
        }

        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            let field = |name: &str| {
                meminfo
                    .lines()
                    .find_map(|line| line.strip_prefix(name))
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|kib| kib.parse::<u64>().ok())
            };
            if let (Some(total), Some(available)) = (field("MemTotal:"), field("MemAvailable:")) {
                stats.mem_total_kib = total;
                stats.mem_used_kib = total.saturating_sub(available);
            }
            if let (Some(total), Some(free)) = (field("SwapTotal:"), field("SwapFree:")) {
                stats.swap_total_kib = total;
                stats.swap_used_kib = total.saturating_sub(free);
            }
        }
        if stats.mem_total_kib > 0 {
            push(&mut self.mem_history, stats.mem_used_kib as f32 / stats.mem_total_kib as f32);
        }

        stats.partitions = self
            .mounts
            .iter()
            .filter_map(|mount| {
                let (used, total) = statvfs(&mount.mount)?;
                Some(Partition {
                    mount: mount.mount.clone(),
                    device: mount.device.clone(),
                    fs: mount.fs.clone(),
                    used,
                    total,
                })
            })
            .collect();
        stats.drives = drives(&stats.partitions);

        if let Some((rx, tx)) = read_net() {
            let sample = (rx, tx, now);
            if let Some((rx_rate, tx_rate)) = rate(sample, self.prev_net) {
                stats.rx_rate = rx_rate;
                stats.tx_rate = tx_rate;
                push(&mut self.rx_history, rx_rate as f32);
                push(&mut self.tx_history, tx_rate as f32);
            }
            self.prev_net = Some(sample);
        }
        if let Some((read, written)) = read_disks(&self.disks) {
            let sample = (read, written, now);
            if let Some((read_rate, write_rate)) = rate(sample, self.prev_disk) {
                stats.read_rate = read_rate;
                stats.write_rate = write_rate;
                push(&mut self.read_history, read_rate as f32);
                push(&mut self.write_history, write_rate as f32);
            }
            self.prev_disk = Some(sample);
        }

        stats.sensors = self
            .sensors
            .iter()
            .filter_map(|probe| {
                Some(Sensor {
                    label: probe.label.clone(),
                    celsius: read_temp(&probe.path)?,
                })
            })
            .collect();
        if let Some(temp) = self.storage_temp.as_deref().and_then(read_temp) {
            push(&mut self.storage_temp_history, temp);
        }
        stats.power = self
            .power
            .iter_mut()
            .filter_map(|(probe, history)| {
                let watts = read_u64(&probe.path)? as f32 / 1_000_000.0;
                push(history, watts);
                Some(PowerSource {
                    label: probe.label.clone(),
                    watts,
                    history: history.iter().copied().collect(),
                })
            })
            .collect();

        let (ticks, count) = process_ticks();
        stats.process_count = count;
        if let Some(at) = self.prev_process_at {
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
                    cpu: (delta as f64 / TICKS / seconds / stats.threads.max(1) as f64) as f32,
                })
                .collect();
            top.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
            top.truncate(TOP_PROCESSES);
            stats.processes = top;
        }
        self.prev_processes = ticks.into_iter().map(|(pid, (_, used))| (pid, used)).collect();
        self.prev_process_at = Some(now);

        if let Ok(loadavg) = fs::read_to_string("/proc/loadavg") {
            for (slot, value) in stats.load.iter_mut().zip(loadavg.split_whitespace()) {
                *slot = value.parse().unwrap_or(0.0);
            }
        }
        if let Ok(uptime) = fs::read_to_string("/proc/uptime") {
            stats.uptime_secs = uptime
                .split_whitespace()
                .next()
                .and_then(|secs| secs.parse::<f64>().ok())
                .map_or(0, |secs| secs as u64);
        }

        stats.info = self.info.clone();
        stats.cpu_history = self.cpu_history.iter().copied().collect();
        stats.gpu_history = self.gpu_history.iter().copied().collect();
        stats.cpu_temp_history = self.cpu_temp_history.iter().copied().collect();
        stats.gpu_temp_history = self.gpu_temp_history.iter().copied().collect();
        stats.storage_temp_history = self.storage_temp_history.iter().copied().collect();
        stats.mem_history = self.mem_history.iter().copied().collect();
        stats.rx_history = self.rx_history.iter().copied().collect();
        stats.tx_history = self.tx_history.iter().copied().collect();
        stats.read_history = self.read_history.iter().copied().collect();
        stats.write_history = self.write_history.iter().copied().collect();
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::{marketing, pci_name};

    const DATABASE: &str = "\
# header comment
1002  Advanced Micro Devices, Inc. [AMD/ATI]
\t1478  Navi 10 XL Upstream Port of PCI Express Switch
# Used in the Sony PlayStation 5 Phat
\t13c0  Granite Ridge [Radeon Graphics]
\t7550  Navi 48 [Radeon RX 9070/9070 XT/9070 GRE]
\t\t1458 2437  Navi 48 XTX [Radeon RX 9070 XT Gaming OC ICE 16G]
\t\t148c 2435  Radeon RX 9070 XT 16GB
\t7551  Navi 48 [Radeon AI PRO R9700]
1022  Advanced Micro Devices, Inc. [AMD]
\t7550  Not a GPU
";

    #[test]
    fn finds_boards_past_comment_lines() {
        assert_eq!(pci_name(DATABASE, "1002", "7550", "148c 2435").as_deref(), Some("Radeon RX 9070 XT"));
        assert_eq!(pci_name(DATABASE, "1002", "7550", "1da2 e490").as_deref(), Some("Radeon RX 9070"));
        assert_eq!(pci_name(DATABASE, "1002", "13c0", "1462 7e51").as_deref(), Some("Radeon Graphics"));
        assert_eq!(pci_name(DATABASE, "1002", "ffff", "0000 0000"), None);
    }

    #[test]
    fn program_names_drop_nix_wrapper_decoration() {
        use super::program_name;
        assert_eq!(program_name(".zen-twilight-wrapped"), "zen-twilight");
        assert_eq!(program_name(".Hyprland-wrapp"), "Hyprland");
        assert_eq!(program_name(".island-wrapped"), "island");
        assert_eq!(program_name("pipewire"), "pipewire");
        assert_eq!(program_name("gtk-wrapper"), "gtk-wrapper");
    }

    #[test]
    fn marketing_names_come_from_pci_ids_entries() {
        assert_eq!(marketing("Radeon RX 9070 XT 16GB"), "Radeon RX 9070 XT");
        assert_eq!(marketing("Navi 48 [Radeon RX 9070/9070 XT/9070 GRE]"), "Radeon RX 9070");
        assert_eq!(marketing("Navi 48 XTX [Sapphire Pulse Radeon RX 9070 XT]"), "Sapphire Pulse Radeon RX 9070 XT");
        assert_eq!(marketing("GeForce RTX 4090"), "GeForce RTX 4090");
    }
}
