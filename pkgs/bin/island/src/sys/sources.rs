use std::{
    collections::HashMap,
    ffi::CString,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use super::stats::{Drive, DrivePart, Partition};

const SECTOR: u64 = 512;

pub(super) const HISTORY: usize = 40;

pub(super) struct Probe {
    pub(super) label: String,
    pub(super) path: PathBuf,
    pub(super) rank: u32,
}

pub(super) struct Mount {
    pub(super) mount: String,
    pub(super) device: String,
}

pub(super) fn read_text(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path)
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}

pub(super) fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

pub(super) fn read_temp(path: &Path) -> Option<f32> {
    read_u64(path).map(|millidegrees| millidegrees as f32 / 1000.0)
}

pub(super) fn first_dir(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .next()
}

pub(super) fn temp_input(dir: &Path, label: &str) -> Option<PathBuf> {
    (1..=16).find_map(|index| {
        let found = fs::read_to_string(dir.join(format!("temp{index}_label")))
            .is_ok_and(|text| text.trim() == label);
        found.then(|| dir.join(format!("temp{index}_input")))
    })
}

pub(super) fn hwmon_temp(name: &str, label: &str) -> Option<PathBuf> {
    fs::read_dir("/sys/class/hwmon")
        .ok()?
        .flatten()
        .find_map(|hwmon| {
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
pub(super) fn sensors(gpu_hwmon: Option<&Path>) -> Vec<Probe> {
    let mut probes = Vec::new();
    let (mut nvme, mut ram) = (0, 0);
    let gpu_hwmon = gpu_hwmon.and_then(|dir| dir.canonicalize().ok());
    for (name, dir) in hwmons() {
        let discrete = gpu_hwmon
            .as_deref()
            .is_some_and(|gpu| dir.canonicalize().ok().as_deref() == Some(gpu));
        let mut add = |label: String, input: Option<PathBuf>, rank: u32| {
            if let Some(path) = input.filter(|path| path.exists()) {
                probes.push(Probe { label, path, rank });
            }
        };
        match name.as_str() {
            "k10temp" => {
                add("CPU".into(), temp_input(&dir, "Tctl"), 0);
                for ccd in 1..=8 {
                    add(
                        format!("CCD {ccd}"),
                        temp_input(&dir, &format!("Tccd{ccd}")),
                        ccd,
                    );
                }
            }
            "coretemp" => add("CPU".into(), temp_input(&dir, "Package id 0"), 0),
            "amdgpu" if discrete => {
                add("GPU".into(), temp_input(&dir, "edge"), 20);
                add("Hotspot".into(), temp_input(&dir, "junction"), 21);
                add("GPU mem".into(), temp_input(&dir, "mem"), 22);
            }
            "amdgpu" => add(
                "iGPU".into(),
                temp_input(&dir, "edge").or_else(|| Some(dir.join("temp1_input"))),
                23,
            ),
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
                add(
                    format!("RAM {ram}"),
                    Some(dir.join("temp1_input")),
                    50 + ram,
                );
            }
            name if name.contains("prom") => {
                add("Chipset".into(), Some(dir.join("temp1_input")), 70)
            }
            name if name.starts_with("iwlwifi")
                || name.starts_with("mt79")
                || name.starts_with("ath") =>
            {
                add("Wi-Fi".into(), Some(dir.join("temp1_input")), 80);
            }
            name if ["r8169", "igc", "e1000", "atlantic", "ixgbe", "bnxt"]
                .iter()
                .any(|nic| name.starts_with(nic)) =>
            {
                add("Network".into(), Some(dir.join("temp1_input")), 79);
            }
            _ => {}
        }
    }
    probes.sort_by_key(|probe| probe.rank);
    probes
}

/// Power draw in watts from GPU sensors: the discrete card first, then any integrated one.
pub(super) fn power_sources(gpu_hwmon: Option<&Path>) -> Vec<Probe> {
    let gpu_hwmon = gpu_hwmon.and_then(|dir| dir.canonicalize().ok());
    let mut sources: Vec<Probe> = hwmons()
        .into_iter()
        .filter(|(name, _)| name == "amdgpu")
        .filter_map(|(_, dir)| {
            let discrete = gpu_hwmon
                .as_deref()
                .is_some_and(|gpu| dir.canonicalize().ok().as_deref() == Some(gpu));
            let path = ["power1_average", "power1_input"]
                .into_iter()
                .map(|file| dir.join(file))
                .find(|path| path.exists())?;
            Some(Probe {
                label: if discrete {
                    "GPU".into()
                } else {
                    "iGPU".into()
                },
                path,
                rank: u32::from(!discrete),
            })
        })
        .collect();
    sources.sort_by_key(|probe| probe.rank);
    sources
}

pub(super) fn discrete_gpu() -> Option<PathBuf> {
    fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .map(|card| card.path().join("device"))
        .filter(|device| device.join("gpu_busy_percent").exists())
        .max_by_key(|device| read_u64(&device.join("mem_info_vram_total")).unwrap_or(0))
}

/// Idle and total jiffies from one `cpu` line of `/proc/stat`.
pub(super) fn parse_cpu_line(line: &str) -> Option<(u64, u64)> {
    let mut fields = line
        .split_whitespace()
        .skip(1)
        .take(8)
        .filter_map(|field| field.parse::<u64>().ok());
    let user = fields.next()?;
    let nice = fields.next()?;
    let system = fields.next()?;
    let idle = fields.next()?;
    let iowait = fields.next()?;
    let rest: u64 = fields.sum();
    Some((idle + iowait, user + nice + system + idle + iowait + rest))
}

pub(super) fn busy((idle, total): (u64, u64), (prev_idle, prev_total): (u64, u64)) -> Option<f32> {
    let d_total = total.saturating_sub(prev_total);
    (d_total > 0).then(|| 1.0 - idle.saturating_sub(prev_idle) as f32 / d_total as f32)
}

pub(super) fn cpu_ghz() -> Option<f32> {
    let khz: Vec<u64> = fs::read_dir("/sys/devices/system/cpu")
        .ok()?
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("cpu"))
                .is_some_and(|rest| {
                    rest.chars().all(|char| char.is_ascii_digit()) && !rest.is_empty()
                })
        })
        .filter_map(|entry| read_u64(&entry.path().join("cpufreq/scaling_cur_freq")))
        .collect();
    (!khz.is_empty()).then(|| khz.iter().sum::<u64>() as f32 / khz.len() as f32 / 1_000_000.0)
}

pub(super) fn read_net() -> Option<(u64, u64)> {
    let dev = fs::read_to_string("/proc/net/dev").ok()?;
    let mut totals = (0u64, 0u64);
    for line in dev.lines().skip(2) {
        let Some((name, counters)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name == "lo"
            || name.starts_with("docker")
            || name.starts_with("veth")
            || name.starts_with("br-")
        {
            continue;
        }
        let mut fields = counters.split_whitespace();
        if let (Some(rx), Some(tx)) = (
            fields.next().and_then(|field| field.parse::<u64>().ok()),
            fields.nth(7).and_then(|field| field.parse::<u64>().ok()),
        ) {
            totals.0 += rx;
            totals.1 += tx;
        }
    }
    Some(totals)
}

/// Physical drives only, not their partitions.
pub(super) fn read_disks(disks: &[String]) -> Option<(u64, u64)> {
    let stats = fs::read_to_string("/proc/diskstats").ok()?;
    let mut totals = (0u64, 0u64);
    for line in stats.lines() {
        let mut fields = line.split_whitespace();
        let Some(name) = fields.nth(2) else {
            continue;
        };
        if !disks.iter().any(|disk| disk == name) {
            continue;
        }
        let Some(read) = fields.nth(2).and_then(|field| field.parse::<u64>().ok()) else {
            continue;
        };
        let Some(written) = fields.nth(3).and_then(|field| field.parse::<u64>().ok()) else {
            continue;
        };
        totals.0 += read * SECTOR;
        totals.1 += written * SECTOR;
    }
    Some(totals)
}

pub(super) fn statvfs(path: &str) -> Option<(u64, u64)> {
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

pub(super) fn cpu_model() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|info| {
            info.lines()
                .find_map(|line| line.strip_prefix("model name"))
                .and_then(|rest| rest.split_once(':'))
                .map(|(_, model)| {
                    model
                        .split_whitespace()
                        .filter(|word| {
                            !word.ends_with("-Core") && *word != "Processor" && *word != "AMD"
                        })
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
        Some((model, size))
            if size.ends_with("GB")
                && size[..size.len() - 2]
                    .chars()
                    .all(|char| char.is_ascii_digit()) =>
        {
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
pub(super) fn gpu_name(device: &Path) -> String {
    let id = |file: &str| {
        read_text(device.join(file))
            .trim_start_matches("0x")
            .to_lowercase()
    };
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

pub(super) fn os_name() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|release| {
            let field = |key: &str| {
                release.lines().find_map(|line| {
                    line.strip_prefix(key)
                        .map(|value| value.trim_matches('"').to_owned())
                })
            };
            match (field("NAME="), field("VERSION_ID=")) {
                (Some(name), Some(version)) => Some(format!("{name} {version}")),
                _ => field("PRETTY_NAME="),
            }
        })
        .unwrap_or_else(|| "Linux".to_owned())
}

pub(super) fn generation() -> Option<u32> {
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
    let resolved = Path::new(device)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(device));
    let mut name = resolved.file_name().map_or_else(
        || device.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    for _ in 0..4 {
        if !name.starts_with("dm-") {
            break;
        }
        match first_dir(&Path::new("/sys/block").join(&name).join("slaves")) {
            Some(slave) => {
                name = slave
                    .file_name()
                    .map_or(name, |slave| slave.to_string_lossy().into_owned())
            }
            None => break,
        }
    }
    name
}

pub(super) fn mounts() -> Vec<Mount> {
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
    for line in fs::read_to_string("/proc/mounts")
        .unwrap_or_default()
        .lines()
    {
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
                },
            )),
        }
    }
    let mut mounts: Vec<Mount> = best.into_iter().map(|(_, mount)| mount).collect();
    mounts.sort_by_key(|mount| (rank(&mount.mount), mount.mount.clone()));
    mounts
}

pub(super) fn drives(partitions: &[Partition]) -> Vec<Drive> {
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
pub(super) fn program(pid_dir: &Path, comm: &str) -> String {
    fs::read_link(pid_dir.join("exe"))
        .ok()
        .and_then(|exe| {
            exe.file_name().map(|name| {
                name.to_string_lossy()
                    .trim_end_matches(" (deleted)")
                    .to_owned()
            })
        })
        .map_or_else(|| program_name(comm), |exe| program_name(&exe))
}

/// CPU ticks per process, labelled by program, so a browser's many processes count as one.
/// `programs` keeps the names of the processes still running.
pub(super) fn process_ticks(
    programs: &mut HashMap<(u32, u64), String>,
) -> (HashMap<u32, (String, u64)>, usize) {
    let mut ticks = HashMap::new();
    let mut known = HashMap::with_capacity(programs.len());
    let mut count = 0;
    for entry in fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        count += 1;
        let (Some(open), Some(close)) = (stat.find('('), stat.rfind(')')) else {
            continue;
        };
        let fields: Vec<&str> = stat[close + 1..].split_whitespace().collect();
        let started = fields
            .get(19)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        let name = match programs.remove(&(pid, started)) {
            Some(name) => name,
            None => program(&entry.path(), &stat[open + 1..close]),
        };
        known.insert((pid, started), name.clone());
        let used = fields
            .get(11)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0)
            + fields
                .get(12)
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0);
        ticks.insert(pid, (name, used));
    }
    *programs = known;
    (ticks, count)
}

pub(super) fn push(history: &mut Vec<f32>, value: f32) {
    if history.len() == HISTORY {
        history.remove(0);
    }
    history.push(value);
}

pub(super) fn rate(
    now: (u64, u64, Instant),
    prev: Option<(u64, u64, Instant)>,
) -> Option<(f64, f64)> {
    let (prev_a, prev_b, at) = prev?;
    let seconds = now.2.duration_since(at).as_secs_f64();
    (seconds > 0.0).then(|| {
        (
            now.0.saturating_sub(prev_a) as f64 / seconds,
            now.1.saturating_sub(prev_b) as f64 / seconds,
        )
    })
}

#[cfg(test)]
#[path = "sources_test.rs"]
mod tests;
