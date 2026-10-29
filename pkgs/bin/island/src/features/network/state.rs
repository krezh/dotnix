//! What the network feature knows, as the worker last read it.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// NetworkManager answered the last time it was asked.
    pub reachable: bool,
    pub internet: Internet,
    pub wifi: Option<Wifi>,
    pub wired: Option<Wired>,
    pub vpns: Vec<Vpn>,
    /// The connection traffic leaves by.
    pub link: Option<Link>,
    pub joining: Option<Joining>,
    pub traffic: Traffic,
    pub signal: Signal,
    pub latency: Latency,
    pub dns: Dns,
    pub usage: Usage,
    pub interfaces: Vec<Interface>,
    pub routes: Vec<Route>,
    pub sockets: Sockets,
    pub ports: Vec<Port>,
}

/// Whether the internet answers past the link, as NetworkManager last checked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Internet {
    /// NetworkManager does not check, or has not yet.
    #[default]
    Unknown,
    None,
    /// A sign-in page stands in the way, such as a café's.
    Portal,
    Limited,
    Full,
}

impl Internet {
    pub fn from_nm(connectivity: u32) -> Self {
        match connectivity {
            1 => Internet::None,
            2 => Internet::Portal,
            3 => Internet::Limited,
            4 => Internet::Full,
            _ => Internet::Unknown,
        }
    }

    /// The link is up but the internet does not answer.
    pub fn missing(self) -> bool {
        matches!(self, Internet::None | Internet::Portal | Internet::Limited)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Security {
    #[default]
    Open,
    Wpa2,
    Wpa3,
    /// Needs more than a password, such as a username and certificates.
    Enterprise,
}

impl Security {
    pub fn label(self) -> &'static str {
        match self {
            Security::Open => "Open",
            Security::Wpa2 => "WPA2",
            Security::Wpa3 => "WPA3",
            Security::Enterprise => "Enterprise",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wifi {
    pub enabled: bool,
    pub interface: String,
    pub scanning: bool,
    /// Strongest first, each name once.
    pub networks: Vec<AccessPoint>,
    /// What the link runs at now, in Mbit/s.
    pub bitrate: u32,
}

impl Wifi {
    pub fn active(&self) -> Option<&AccessPoint> {
        self.networks.iter().find(|network| network.active)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AccessPoint {
    pub ssid: String,
    /// 0 to 100.
    pub strength: u8,
    /// MHz.
    pub frequency: u32,
    pub security: Security,
    pub saved: bool,
    pub active: bool,
    pub bssid: String,
    /// Mbit/s.
    pub max_bitrate: u32,
    /// MHz, 0 when NetworkManager does not say.
    pub width: u32,
}

impl AccessPoint {
    pub fn band(&self) -> &'static str {
        match self.frequency {
            5925.. => "6 GHz",
            4900.. => "5 GHz",
            _ => "2.4 GHz",
        }
    }

    pub fn channel(&self) -> u32 {
        channel(self.frequency)
    }
}

/// The Wi-Fi channel a centre frequency in MHz belongs to.
pub fn channel(frequency: u32) -> u32 {
    match frequency {
        2484 => 14,
        2412..=2472 => (frequency - 2407) / 5,
        5955..=7115 => (frequency - 5950) / 5,
        5000..=5925 => (frequency - 5000) / 5,
        _ => 0,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wired {
    pub interface: String,
    /// The connection's name, such as "Wired connection 1".
    pub name: String,
    pub carrier: bool,
    pub active: bool,
    /// Mbit/s.
    pub speed: u32,
    pub mac: String,
    pub errors: u64,
    pub drops: u64,
    pub carrier_changes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    Off,
    Connecting,
    On,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vpn {
    pub name: String,
    /// Such as "WireGuard".
    pub kind: String,
    pub activity: Activity,
    pub address: Option<String>,
    pub interface: Option<String>,
    /// Where it connects to, as its settings say.
    pub endpoint: Option<String>,
    /// Unix seconds it came up, as island saw it.
    pub since: Option<i64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinkKind {
    Wifi,
    #[default]
    Wired,
    Other,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Link {
    pub interface: String,
    pub name: String,
    pub kind: LinkKind,
    pub ipv4: Option<String>,
    pub prefix: u32,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
    pub domain: Option<String>,
    pub ipv6: Option<String>,
    pub mtu: u32,
    /// Unix seconds the DHCP lease runs out.
    pub lease: Option<i64>,
}

/// A network being joined, or one that could not be.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Joining {
    pub ssid: String,
    pub failed: Option<String>,
}

/// Rates in bytes a second, histories oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Traffic {
    pub rx_rate: f64,
    pub tx_rate: f64,
    pub rx_history: Vec<f32>,
    pub tx_history: Vec<f32>,
    pub vpn_history: Vec<f32>,
    pub vpn_rx: u64,
    pub vpn_tx: u64,
    /// When the histories last grew, so a chart of them can scroll on their own clock.
    pub sampled: Option<std::time::Instant>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Signal {
    pub dbm: Option<i32>,
    pub noise: Option<i32>,
    /// 0 to 1, as the driver rates the link.
    pub quality: Option<f32>,
    /// dBm, oldest first.
    pub history: Vec<f32>,
    /// Share of frames sent again, as a percentage.
    pub retries: Option<f32>,
}

/// Milliseconds, histories oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Latency {
    pub gateway: Option<f32>,
    pub internet: Option<f32>,
    /// Share of recent pings to the internet that went unanswered, 0 to 1.
    pub loss: f32,
    pub gateway_history: Vec<f32>,
    pub internet_history: Vec<f32>,
    pub target: String,
    /// When the histories last grew.
    pub sampled: Option<std::time::Instant>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dns {
    /// Milliseconds a lookup took.
    pub lookup: Option<f32>,
    pub history: Vec<f32>,
    /// When the history last grew.
    pub sampled: Option<std::time::Instant>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Usage {
    pub today_rx: u64,
    pub today_tx: u64,
    /// Bytes each day, oldest first, ending today.
    pub days: Vec<(String, u64)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Interface {
    pub name: String,
    pub kind: String,
    pub up: bool,
    /// Bytes a second, both ways.
    pub rate: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Route {
    pub destination: String,
    pub via: Option<String>,
    pub interface: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sockets {
    pub established: u32,
    pub listening: u32,
    pub waiting: u32,
    /// Apps with the most connections open, most first.
    pub apps: Vec<(String, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Port {
    pub protocol: &'static str,
    pub port: u16,
    pub app: String,
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
