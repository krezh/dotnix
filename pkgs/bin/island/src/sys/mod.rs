mod sampler;
mod sources;
mod stats;
mod watch;

pub use stats::{Sensor, SysStats};
pub use watch::{INTERVAL, Samples, Watch, watch};
