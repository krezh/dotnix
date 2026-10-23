//! What the kernel says about the network without asking for privileges: byte counters, the
//! wireless link, sockets, listening ports and routes.

use std::{collections::HashMap, fs, net::Ipv4Addr};

use super::state::{Port, Route, Sockets};

/// One interface's counters since it came up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub errors: u64,
    pub drops: u64,
    pub carrier_changes: u64,
}

fn number(path: &str) -> u64 {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

pub fn counters(interface: &str) -> Counters {
    let stat = |name: &str| number(&format!("/sys/class/net/{interface}/statistics/{name}"));
    Counters {
        rx_bytes: stat("rx_bytes"),
        tx_bytes: stat("tx_bytes"),
        tx_packets: stat("tx_packets"),
        errors: stat("rx_errors") + stat("tx_errors"),
        drops: stat("rx_dropped") + stat("tx_dropped"),
        carrier_changes: number(&format!("/sys/class/net/{interface}/carrier_changes")),
    }
}

pub fn interfaces() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir("/sys/class/net")
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

pub fn is_up(interface: &str) -> bool {
    fs::read_to_string(format!("/sys/class/net/{interface}/operstate"))
        .is_ok_and(|state| matches!(state.trim(), "up" | "unknown"))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wireless {
    /// 0 to 1.
    pub quality: f32,
    pub dbm: i32,
    pub noise: Option<i32>,
    /// Frames dropped after too many retries, since the interface came up.
    pub retries: u64,
}

/// `text` is /proc/net/wireless.
pub fn parse_wireless(text: &str, interface: &str) -> Option<Wireless> {
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with(&format!("{interface}:")))?;
    let fields: Vec<&str> = line.split_once(':')?.1.split_whitespace().collect();
    let value = |index: usize| -> Option<f32> {
        fields
            .get(index)?
            .trim_end_matches('.')
            .parse::<f32>()
            .ok()
    };
    let noise = value(3)? as i32;
    Some(Wireless {
        quality: (value(1)? / 70.0).clamp(0.0, 1.0),
        dbm: value(2)? as i32,
        noise: (noise > -256 && noise != 0).then_some(noise),
        retries: fields.get(7)?.parse().ok()?,
    })
}

pub fn wireless(interface: &str) -> Option<Wireless> {
    parse_wireless(&fs::read_to_string("/proc/net/wireless").ok()?, interface)
}

/// A little-endian hex address as /proc/net/route writes it.
fn route_address(hex: &str) -> Option<Ipv4Addr> {
    u32::from_str_radix(hex, 16)
        .ok()
        .map(|raw| Ipv4Addr::from(raw.to_le_bytes()))
}

/// `text` is /proc/net/route; the default route first.
pub fn parse_routes(text: &str) -> Vec<Route> {
    let mut routes: Vec<(u32, Route)> = text
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let (interface, destination, gateway, mask) =
                (fields.first()?, fields.get(1)?, fields.get(2)?, fields.get(7)?);
            let metric: u32 = fields.get(6)?.parse().ok()?;
            let destination = route_address(destination)?;
            let gateway = route_address(gateway)?;
            let prefix = route_address(mask)?.to_bits().count_ones();
            let destination = if prefix == 0 {
                "default".to_owned()
            } else {
                format!("{destination}/{prefix}")
            };
            Some((
                metric,
                Route {
                    destination,
                    via: (!gateway.is_unspecified()).then(|| gateway.to_string()),
                    interface: (*interface).to_owned(),
                },
            ))
        })
        .collect();
    routes.sort_by_key(|(metric, route)| (route.destination != "default", *metric));
    let mut seen = Vec::new();
    routes
        .into_iter()
        .map(|(_, route)| route)
        .filter(|route| {
            let key = (route.destination.clone(), route.interface.clone());
            let fresh = !seen.contains(&key);
            seen.push(key);
            fresh
        })
        .collect()
}

pub fn routes() -> Vec<Route> {
    fs::read_to_string("/proc/net/route")
        .map(|text| parse_routes(&text))
        .unwrap_or_default()
}

/// One line of /proc/net/{tcp,tcp6,udp,udp6}.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Socket {
    pub port: u16,
    pub state: u8,
    pub inode: u64,
}

pub const ESTABLISHED: u8 = 0x01;
pub const TIME_WAIT: u8 = 0x06;
pub const CLOSE_WAIT: u8 = 0x08;
pub const LISTEN: u8 = 0x0a;
/// What an unconnected UDP socket waiting for datagrams reads as.
pub const UDP_OPEN: u8 = 0x07;

pub fn parse_sockets(text: &str) -> Vec<Socket> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let port = fields.get(1)?.rsplit_once(':')?.1;
            Some(Socket {
                port: u16::from_str_radix(port, 16).ok()?,
                state: u8::from_str_radix(fields.get(3)?, 16).ok()?,
                inode: fields.get(9)?.parse().ok()?,
            })
        })
        .collect()
}

fn read_sockets(names: &[&str]) -> Vec<Socket> {
    names
        .iter()
        .filter_map(|name| fs::read_to_string(format!("/proc/net/{name}")).ok())
        .flat_map(|text| parse_sockets(&text))
        .collect()
}

/// An app's name as people know it, without the dot and suffix a Nix wrapper adds, such as
/// ".zen-twilight-wrapped" for zen-twilight. The kernel cuts names to 15 bytes, so a cut-off
/// suffix goes too.
pub fn app_name(comm: &str) -> String {
    let name = comm.trim_start_matches('.');
    let name = ["-wrapped", "-wrappe", "-wrapp", "-wrap", "-wra", "-wr", "-w"]
        .iter()
        .find_map(|suffix| comm.starts_with('.').then(|| name.strip_suffix(suffix)).flatten())
        .unwrap_or(name);
    name.to_owned()
}

/// Port numbers' service names, as /etc/services lists them, for ports whose app island cannot
/// see.
fn services() -> HashMap<(u16, &'static str), String> {
    let text = fs::read_to_string("/etc/services").unwrap_or_default();
    let mut names = HashMap::new();
    for line in text.lines() {
        let mut fields = line.split('#').next().unwrap_or("").split_whitespace();
        let (Some(name), Some(port)) = (fields.next(), fields.next()) else {
            continue;
        };
        let Some((number, protocol)) = port.split_once('/') else {
            continue;
        };
        let protocol = match protocol {
            "tcp" => "TCP",
            "udp" => "UDP",
            _ => continue,
        };
        if let Ok(number) = number.parse() {
            names.entry((number, protocol)).or_insert_with(|| name.to_owned());
        }
    }
    names
}

/// Which app holds each socket, by inode, for the processes island may look into.
fn owners() -> HashMap<u64, String> {
    let mut owners = HashMap::new();
    for process in fs::read_dir("/proc").into_iter().flatten().flatten() {
        let pid = process.file_name();
        if !pid.to_string_lossy().bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let Ok(fds) = fs::read_dir(process.path().join("fd")) else {
            continue;
        };
        let mut name = None;
        for fd in fds.flatten() {
            let Ok(target) = fs::read_link(fd.path()) else {
                continue;
            };
            let Some(inode) = target
                .to_str()
                .and_then(|target| target.strip_prefix("socket:["))
                .and_then(|rest| rest.strip_suffix(']'))
                .and_then(|inode| inode.parse::<u64>().ok())
            else {
                continue;
            };
            let name = name.get_or_insert_with(|| {
                fs::read_to_string(process.path().join("comm"))
                    .map(|comm| app_name(comm.trim()))
                    .unwrap_or_default()
            });
            owners.insert(inode, name.clone());
        }
    }
    owners
}

pub fn sockets_and_ports() -> (Sockets, Vec<Port>) {
    let tcp = read_sockets(&["tcp", "tcp6"]);
    let udp = read_sockets(&["udp", "udp6"]);
    let owners = owners();
    let services = services();
    let app = |inode: u64, port: u16, protocol: &'static str| {
        owners
            .get(&inode)
            .or_else(|| services.get(&(port, protocol)))
            .cloned()
            .unwrap_or_else(|| "system".to_owned())
    };
    let mut per_app: HashMap<String, u32> = HashMap::new();
    let mut sockets = Sockets::default();
    for socket in &tcp {
        match socket.state {
            ESTABLISHED => {
                sockets.established += 1;
                if let Some(name) = owners.get(&socket.inode) {
                    *per_app.entry(name.clone()).or_default() += 1;
                }
            }
            LISTEN => sockets.listening += 1,
            TIME_WAIT | CLOSE_WAIT => sockets.waiting += 1,
            _ => {}
        }
    }
    let mut apps: Vec<(String, u32)> = per_app.into_iter().collect();
    apps.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    apps.truncate(4);
    sockets.apps = apps;
    let mut ports: Vec<Port> = tcp
        .iter()
        .filter(|socket| socket.state == LISTEN)
        .map(|socket| ("TCP", socket))
        .chain(
            udp.iter()
                .filter(|socket| socket.state == UDP_OPEN)
                .map(|socket| ("UDP", socket)),
        )
        .map(|(protocol, socket)| Port {
            protocol,
            port: socket.port,
            app: app(socket.inode, socket.port, protocol),
        })
        .collect();
    ports.sort_by(|a, b| a.port.cmp(&b.port).then(a.protocol.cmp(b.protocol)));
    ports.dedup_by(|a, b| a.port == b.port && a.protocol == b.protocol);
    (sockets, ports)
}

#[cfg(test)]
#[path = "kernel_test.rs"]
mod tests;
