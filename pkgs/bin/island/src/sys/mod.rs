mod sampler;
mod sources;
mod stats;
mod watch;

pub use sources::{read_u64, uptime};
pub use stats::{Sensor, SysStats};
pub use watch::{INTERVAL, Samples, watch};
