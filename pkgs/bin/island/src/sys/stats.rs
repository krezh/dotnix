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
