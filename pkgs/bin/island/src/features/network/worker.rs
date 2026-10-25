//! The network feature's worker. It reads NetworkManager when NetworkManager says something
//! changed, measures what the charts show only while the panel is open, keeps the day's data used,
//! carries out requests, and publishes what the islands show. Nothing runs on a timer while the
//! panel is closed and the network is quiet.

use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use chrono::{Days, Local};
use futures::{
    FutureExt, StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    stream,
};
use tracing::{debug, warn};
use zbus::{Connection, MatchRule, MessageStream, OwnedObjectPath, block_on};

use super::{
    kernel::{self, Counters},
    nm::{self, Saved, Snapshot},
    runtime::{Probes, Request, Settings, Watched},
    state::{
        AccessPoint, Activity, Interface, Internet, Joining, Link, LinkKind, Signal, State, Usage,
        Vpn, Wifi, Wired,
    },
    usage,
};
use crate::{components::card::SLOTS, features::Feed, supervisor::Ended};

const SOCKETS_EVERY: u64 = 3;
/// The day's data used is written at most this often.
const USAGE_EVERY: Duration = Duration::from_secs(60);
/// NetworkManager's changes come in bursts; one read follows each burst.
const SETTLE: Duration = Duration::from_millis(150);
const JOIN_TIMEOUT: Duration = Duration::from_secs(45);
const SCAN_TIMEOUT: Duration = Duration::from_secs(15);
/// A scan this old is redone when the panel opens.
const SCAN_STALE_MS: i64 = 30_000;
/// NetworkManager's reasons a Wi-Fi connection ends without coming up that point at the password.
const SECRET_REASONS: [u32; 5] = [7, 8, 9, 10, 11];
const WIFI: u32 = 2;
const ETHERNET: u32 = 1;

struct Join {
    ssid: String,
    /// A connection saved for this attempt, taken back if it fails.
    saved: Option<OwnedObjectPath>,
    active: OwnedObjectPath,
    device: OwnedObjectPath,
    started: Instant,
}

/// What the bar shows, so the islands hear of nothing else while the panel is closed.
#[derive(Clone, PartialEq, Default)]
struct Glance {
    reachable: bool,
    wifi: Option<(bool, Option<String>, u8, bool)>,
    wired: Option<bool>,
    vpn: bool,
    joining: Option<Joining>,
    internet: Internet,
}

impl Glance {
    fn of(state: &State) -> Self {
        Self {
            reachable: state.reachable,
            wifi: state.wifi.as_ref().map(|wifi| {
                let active = wifi.active();
                (
                    wifi.enabled,
                    active.map(|network| network.ssid.clone()),
                    active.map_or(0, |network| network.strength / 10),
                    wifi.scanning,
                )
            }),
            wired: state.wired.as_ref().map(|wired| wired.active),
            vpn: state.vpns.iter().any(|vpn| vpn.activity == Activity::On),
            joining: state.joining.clone(),
            internet: state.internet,
        }
    }
}

#[derive(Default)]
struct World {
    state: State,
    snapshot: Snapshot,
    saved: Vec<Saved>,
    counters: HashMap<String, (Counters, Instant)>,
    rates: HashMap<String, (f64, f64)>,
    vpn_since: HashMap<String, i64>,
    seen_vpns: bool,
    joining: Option<Join>,
    scan: Option<(Instant, i64)>,
    pending: (u64, u64),
    day: String,
    stored: Vec<(String, u64, u64)>,
    retries: Option<(u64, u64)>,
    probes_seen: (u64, u64),
    samples: u64,
    flushed: Option<Instant>,
    last_counters: HashMap<String, Counters>,
    dirty: bool,
    published: Option<Glance>,
}

enum Event {
    /// NetworkManager said something changed, or came or went; `true` when the saved connections
    /// did.
    Bus(bool),
    Request(Request),
}

fn push(history: &mut Vec<f32>, value: f32) {
    history.push(value);
    if history.len() > SLOTS {
        let extra = history.len() - SLOTS;
        history.drain(..extra);
    }
}

fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn ipv4(text: Option<&String>) -> Option<Ipv4Addr> {
    text?.parse().ok()
}

impl World {
    fn wifi_device(&self) -> Option<&nm::Device> {
        self.snapshot.devices.iter().find(|device| device.kind == WIFI)
    }

    fn saved_wifi(&self, ssid: &str) -> Option<&Saved> {
        self.saved
            .iter()
            .find(|saved| saved.ssid.as_deref() == Some(ssid))
    }

    fn access_point(&self, ssid: &str) -> Option<(OwnedObjectPath, AccessPoint)> {
        self.wifi_device()?
            .wireless
            .as_ref()?
            .access_points
            .iter()
            .filter(|(_, ap)| ap.ssid == ssid)
            .max_by_key(|(_, ap)| ap.strength)
            .cloned()
    }

    fn read(&mut self, connection: &Connection) {
        match block_on(nm::snapshot(connection)) {
            Ok(snapshot) => {
                if !self.state.reachable {
                    self.dirty = true;
                }
                self.snapshot = snapshot;
                self.state.reachable = true;
                self.state.internet = Internet::from_nm(self.snapshot.connectivity);
            }
            Err(error) => {
                if self.state.reachable {
                    warn!("cannot read NetworkManager: {error}");
                }
                self.state.reachable = false;
                return;
            }
        }
        if self.dirty {
            match block_on(nm::saved(connection)) {
                Ok(saved) => {
                    self.saved = saved;
                    self.dirty = false;
                }
                Err(error) => warn!("cannot read the saved connections: {error}"),
            }
        }
    }

    /// Bytes a second each way on every interface, from how its counters moved.
    fn sample(&mut self) -> HashMap<String, Counters> {
        let now = Instant::now();
        let mut all = HashMap::new();
        for name in kernel::interfaces() {
            let counters = kernel::counters(&name);
            if let Some((before, at)) = self.counters.get(&name) {
                let seconds = now.duration_since(*at).as_secs_f64().max(0.001);
                let rx = counters.rx_bytes.saturating_sub(before.rx_bytes) as f64 / seconds;
                let tx = counters.tx_bytes.saturating_sub(before.tx_bytes) as f64 / seconds;
                self.rates.insert(name.clone(), (rx, tx));
                if self
                    .snapshot
                    .devices
                    .iter()
                    .any(|device| device.interface == name && matches!(device.kind, WIFI | ETHERNET))
                {
                    self.pending.0 += counters.rx_bytes.saturating_sub(before.rx_bytes);
                    self.pending.1 += counters.tx_bytes.saturating_sub(before.tx_bytes);
                }
            }
            self.counters.insert(name.clone(), (counters, now));
            all.insert(name, counters);
        }
        all
    }

    fn rate(&self, interface: &str) -> (f64, f64) {
        self.rates.get(interface).copied().unwrap_or((0.0, 0.0))
    }

    fn build(&mut self, connection: &Connection, counters: &HashMap<String, Counters>) {
        let snapshot = &self.snapshot;
        let active_of = |saved: &OwnedObjectPath| {
            snapshot
                .active
                .iter()
                .find(|active| &active.saved == saved)
        };
        self.state.wifi = self.wifi_device().map(|device| {
            let wireless = device.wireless.as_ref();
            let mut networks: Vec<AccessPoint> = Vec::new();
            for (path, ap) in wireless.map(|wireless| wireless.access_points.as_slice()).unwrap_or_default() {
                if ap.ssid.is_empty() {
                    continue;
                }
                let mut ap = ap.clone();
                ap.active = wireless.is_some_and(|wireless| &wireless.active_ap == path);
                ap.saved = self.saved_wifi(&ap.ssid).is_some();
                match networks.iter_mut().find(|known| known.ssid == ap.ssid) {
                    Some(known) if known.strength < ap.strength || ap.active => {
                        let active = known.active || ap.active;
                        *known = ap;
                        known.active = active;
                    }
                    Some(known) => known.active |= ap.active,
                    None => networks.push(ap),
                }
            }
            networks.sort_by(|a, b| {
                (b.active, b.saved, b.strength).cmp(&(a.active, a.saved, a.strength))
            });
            let scanning = self.scan.is_some_and(|(at, before)| {
                at.elapsed() < SCAN_TIMEOUT && wireless.is_some_and(|wireless| wireless.last_scan <= before)
            });
            Wifi {
                enabled: snapshot.wireless_enabled,
                interface: device.interface.clone(),
                scanning,
                networks,
                bitrate: wireless.map_or(0, |wireless| wireless.bitrate),
            }
        });
        self.state.wired = snapshot
            .devices
            .iter()
            .find(|device| device.kind == ETHERNET)
            .map(|device| {
                let counters = counters.get(&device.interface).copied().unwrap_or_default();
                let name = snapshot
                    .active
                    .iter()
                    .find(|active| active.path == device.active)
                    .map_or_else(|| "Ethernet".to_owned(), |active| active.name.clone());
                Wired {
                    interface: device.interface.clone(),
                    name,
                    carrier: device.wired.is_some_and(|wired| wired.carrier),
                    active: device.activated,
                    speed: device.wired.map_or(0, |wired| wired.speed),
                    mac: device.mac.clone(),
                    errors: counters.errors,
                    drops: counters.drops,
                    carrier_changes: counters.carrier_changes,
                }
            });
        let now = chrono::Utc::now().timestamp();
        let mut vpns = Vec::new();
        for saved in self
            .saved
            .iter()
            .filter(|saved| matches!(saved.kind.as_str(), "vpn" | "wireguard"))
        {
            let active = active_of(&saved.path);
            let activity = active.map_or(Activity::Off, |active| active.activity);
            let interface = active
                .and_then(|active| active.devices.first())
                .and_then(|path| snapshot.devices.iter().find(|device| &device.path == path))
                .map(|device| device.interface.clone());
            let address = match active {
                Some(active) if activity == Activity::On => {
                    block_on(nm::active_address(connection, &active.ip4))
                }
                _ => None,
            };
            if activity == Activity::On {
                // Only a VPN island saw come up has a start; one up already when island started has none.
                if self.seen_vpns {
                    self.vpn_since.entry(saved.name.clone()).or_insert(now);
                }
            } else {
                self.vpn_since.remove(&saved.name);
            }
            vpns.push(Vpn {
                name: saved.name.clone(),
                kind: if saved.kind == "wireguard" {
                    "WireGuard".to_owned()
                } else {
                    "VPN".to_owned()
                },
                activity,
                address,
                interface,
                endpoint: saved.endpoint.clone(),
                since: self.vpn_since.get(&saved.name).copied(),
            });
        }
        self.seen_vpns = true;
        vpns.sort_by(|a, b| a.name.cmp(&b.name));
        self.state.vpns = vpns;
        self.state.link = snapshot
            .primary
            .as_ref()
            .and_then(|primary| snapshot.active.iter().find(|active| &active.path == primary))
            .and_then(|active| active.devices.first())
            .and_then(|path| snapshot.devices.iter().find(|device| &device.path == path))
            .map(|device| Link {
                interface: device.interface.clone(),
                name: snapshot
                    .active
                    .iter()
                    .find(|active| active.path == device.active)
                    .map(|active| active.name.clone())
                    .unwrap_or_default(),
                kind: match device.kind {
                    WIFI => LinkKind::Wifi,
                    ETHERNET => LinkKind::Wired,
                    _ => LinkKind::Other,
                },
                ipv4: device.address.ipv4.clone(),
                prefix: device.address.prefix,
                gateway: device.address.gateway.clone(),
                dns: device.address.dns.clone(),
                domain: device.address.domain.clone(),
                ipv6: device.address.ipv6.clone(),
                mtu: device.mtu,
                lease: device.address.lease,
            });
        self.state.interfaces = snapshot
            .devices
            .iter()
            .filter(|device| device.kind != 32)
            .map(|device| {
                let (rx, tx) = self.rate(&device.interface);
                Interface {
                    name: device.interface.clone(),
                    kind: nm::kind_label(device.kind).to_owned(),
                    up: device.activated || kernel::is_up(&device.interface),
                    rate: rx + tx,
                }
            })
            .collect();
    }

    fn measure(&mut self, counters: &HashMap<String, Counters>) {
        let link = self.state.link.as_ref().map(|link| link.interface.clone());
        let (rx, tx) = link.as_deref().map_or((0.0, 0.0), |link| self.rate(link));
        let traffic = &mut self.state.traffic;
        traffic.rx_rate = rx;
        traffic.tx_rate = tx;
        push(&mut traffic.rx_history, rx as f32);
        push(&mut traffic.tx_history, tx as f32);
        let vpn = self
            .state
            .vpns
            .iter()
            .find(|vpn| vpn.activity == Activity::On)
            .and_then(|vpn| vpn.interface.clone());
        let vpn_rate = vpn.as_deref().map_or(0.0, |vpn| {
            let (rx, tx) = self.rate(vpn);
            rx + tx
        });
        let vpn_counters = vpn
            .as_deref()
            .and_then(|vpn| counters.get(vpn))
            .copied()
            .unwrap_or_default();
        let traffic = &mut self.state.traffic;
        push(&mut traffic.vpn_history, vpn_rate as f32);
        traffic.vpn_rx = vpn_counters.rx_bytes;
        traffic.vpn_tx = vpn_counters.tx_bytes;

        let wireless = self
            .state
            .wifi
            .as_ref()
            .filter(|wifi| wifi.active().is_some())
            .and_then(|wifi| {
                kernel::wireless(&wifi.interface)
                    .map(|wireless| (wireless, counters.get(&wifi.interface).copied()))
            });
        let signal = &mut self.state.signal;
        match wireless {
            Some((wireless, counters)) => {
                signal.dbm = Some(wireless.dbm);
                signal.noise = wireless.noise;
                signal.quality = Some(wireless.quality);
                push(&mut signal.history, wireless.dbm as f32);
                let packets = counters.map_or(0, |counters| counters.tx_packets);
                if let Some((retries, sent)) = self.retries {
                    let sent = packets.saturating_sub(sent);
                    if sent > 0 {
                        let again = wireless.retries.saturating_sub(retries) as f32;
                        signal.retries = Some((again / sent as f32 * 100.0).min(100.0));
                    }
                }
                self.retries = Some((wireless.retries, packets));
            }
            None => {
                *signal = Signal {
                    history: std::mem::take(&mut signal.history),
                    ..Signal::default()
                };
                self.retries = None;
            }
        }
    }

    fn probes(&mut self, probes: &Mutex<Probes>, settings: &Settings) {
        let link = self.state.link.as_ref();
        let mut probes = probes.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        probes.gateway_address = ipv4(link.and_then(|link| link.gateway.as_ref()));
        probes.dns_address = ipv4(link.and_then(|link| link.dns.first()));
        let latency = &mut self.state.latency;
        latency.target = settings.latency_target.clone();
        if probes.round != self.probes_seen.0 {
            self.probes_seen.0 = probes.round;
            latency.gateway = probes.gateway;
            latency.internet = probes.internet;
            push(&mut latency.gateway_history, probes.gateway.unwrap_or(0.0));
            push(&mut latency.internet_history, probes.internet.unwrap_or(0.0));
            let lost = probes.answered.iter().filter(|answered| !**answered).count();
            latency.loss = lost as f32 / probes.answered.len().max(1) as f32;
        }
        if probes.lookups != self.probes_seen.1 {
            self.probes_seen.1 = probes.lookups;
            let dns = &mut self.state.dns;
            dns.lookup = probes.lookup;
            push(&mut dns.history, probes.lookup.unwrap_or(0.0));
        }
    }

    /// Writes the day's data used now and then, and keeps a week of it in view.
    fn usage(&mut self, force: bool) {
        let day = today();
        let rolled = !self.day.is_empty() && self.day != day;
        let due = self.flushed.is_none_or(|at| at.elapsed() >= USAGE_EVERY);
        if force || rolled || due {
            self.flushed = Some(Instant::now());
            let at = if self.day.is_empty() { day.clone() } else { self.day.clone() };
            let (rx, tx) = std::mem::take(&mut self.pending);
            if (rx > 0 || tx > 0)
                && let Err(error) = usage::add(&at, rx, tx)
            {
                warn!("{error}");
                self.pending = (rx, tx);
            }
            let since = Local::now()
                .date_naive()
                .checked_sub_days(Days::new(usage::DAYS as u64 - 1))
                .map(|since| since.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            match usage::read(&since) {
                Ok(stored) => self.stored = stored,
                Err(error) => debug!("{error}"),
            }
            self.day = day;
        }
        let today = Local::now().date_naive();
        let mut days = Vec::new();
        for back in (0..usage::DAYS).rev() {
            let Some(date) = today.checked_sub_days(Days::new(back as u64)) else {
                continue;
            };
            let key = date.format("%Y-%m-%d").to_string();
            let (rx, tx) = self
                .stored
                .iter()
                .find(|(day, _, _)| *day == key)
                .map_or((0, 0), |(_, rx, tx)| (*rx, *tx));
            let (rx, tx) = if back == 0 {
                (rx + self.pending.0, tx + self.pending.1)
            } else {
                (rx, tx)
            };
            let label = if back == 0 {
                "Today".to_owned()
            } else {
                date.format("%a").to_string()
            };
            if back == 0 {
                self.state.usage.today_rx = rx;
                self.state.usage.today_tx = tx;
            }
            days.push((label, rx + tx));
        }
        self.state.usage = Usage { days, ..self.state.usage.clone() };
    }

    fn follow_join(&mut self, connection: &Connection) {
        let Some(join) = &self.joining else {
            return;
        };
        let activity = block_on(nm::activity(connection, &join.active));
        match activity {
            Some(Activity::On) => {
                debug!(ssid = join.ssid, "joined");
                self.state.joining = None;
                self.joining = None;
                self.dirty = true;
            }
            Some(Activity::Connecting) if join.started.elapsed() < JOIN_TIMEOUT => {
                self.state.joining = Some(Joining {
                    ssid: join.ssid.clone(),
                    failed: None,
                });
            }
            _ => {
                let reason = block_on(nm::reason(connection, &join.device));
                let secret = reason.is_some_and(|reason| SECRET_REASONS.contains(&reason));
                let failed = if secret && join.saved.is_some() {
                    "Wrong password. Try it again."
                } else {
                    "Could not connect."
                };
                warn!(ssid = join.ssid, ?reason, "could not join");
                if let Some(saved) = &join.saved
                    && let Err(error) = block_on(nm::forget(connection, saved))
                {
                    warn!("cannot remove the failed connection: {error}");
                }
                self.state.joining = Some(Joining {
                    ssid: join.ssid.clone(),
                    failed: Some(failed.to_owned()),
                });
                self.joining = None;
                self.dirty = true;
            }
        }
    }

    fn handle(&mut self, connection: &Connection, sender: &UnboundedSender<Request>, request: Request) {
        debug!(?request, "network request");
        let device = self.wifi_device().map(|device| device.path.clone());
        if matches!(request, Request::Connect(_) | Request::Join(..)) {
            // A join that never ends gets one more look once it has had its time.
            let sender = sender.clone();
            let _ = thread::Builder::new()
                .name("island-network-join".into())
                .spawn(move || {
                    thread::sleep(JOIN_TIMEOUT + Duration::from_secs(1));
                    let _ = sender.unbounded_send(Request::Refresh);
                });
        }
        let result: zbus::Result<()> = block_on(async {
            match request {
                Request::Wifi(on) => {
                    let on = on.unwrap_or(!self.snapshot.wireless_enabled);
                    nm::set_wireless(connection, on).await
                }
                Request::Scan => match &device {
                    Some(device) => {
                        let before = self
                            .wifi_device()
                            .and_then(|device| device.wireless.as_ref())
                            .map_or(0, |wireless| wireless.last_scan);
                        self.scan = Some((Instant::now(), before));
                        nm::scan(connection, device).await
                    }
                    None => Ok(()),
                },
                Request::Connect(ssid) => {
                    let Some(device) = device else { return Ok(()) };
                    self.state.joining = None;
                    if let Some(saved) = self.saved_wifi(&ssid).map(|saved| saved.path.clone()) {
                        let active = nm::activate(connection, &saved, Some(&device)).await?;
                        self.joining = Some(Join {
                            ssid,
                            saved: None,
                            active,
                            device,
                            started: Instant::now(),
                        });
                    } else if let Some((ap, found)) = self.access_point(&ssid) {
                        let (saved, active) =
                            nm::join(connection, &device, &ap, found.security, "").await?;
                        self.joining = Some(Join {
                            ssid,
                            saved: Some(saved),
                            active,
                            device,
                            started: Instant::now(),
                        });
                    }
                    Ok(())
                }
                Request::Join(ssid, password) => {
                    let Some(device) = device else { return Ok(()) };
                    let Some((ap, found)) = self.access_point(&ssid) else {
                        return Ok(());
                    };
                    self.state.joining = Some(Joining {
                        ssid: ssid.clone(),
                        failed: None,
                    });
                    let (saved, active) =
                        nm::join(connection, &device, &ap, found.security, &password).await?;
                    self.joining = Some(Join {
                        ssid,
                        saved: Some(saved),
                        active,
                        device,
                        started: Instant::now(),
                    });
                    Ok(())
                }
                Request::Disconnect => match self.wifi_device() {
                    Some(device) if device.activated => {
                        nm::deactivate(connection, &device.active.clone()).await
                    }
                    _ => Ok(()),
                },
                Request::Forget(ssid) => match self.saved_wifi(&ssid).map(|saved| saved.path.clone()) {
                    Some(saved) => nm::forget(connection, &saved).await,
                    None => Ok(()),
                },
                Request::Vpn(name, on) => {
                    let Some(saved) = self
                        .saved
                        .iter()
                        .find(|saved| saved.name == name && matches!(saved.kind.as_str(), "vpn" | "wireguard"))
                        .map(|saved| saved.path.clone())
                    else {
                        return Ok(());
                    };
                    let active = self
                        .snapshot
                        .active
                        .iter()
                        .find(|active| active.saved == saved)
                        .map(|active| active.path.clone());
                    match (active, on.unwrap_or(true)) {
                        (Some(active), false) => nm::deactivate(connection, &active).await,
                        (Some(active), true) if on.is_none() => nm::deactivate(connection, &active).await,
                        (None, true) => nm::activate(connection, &saved, None).await.map(drop),
                        _ => Ok(()),
                    }
                }
                Request::Watch | Request::Sample | Request::Midnight | Request::Refresh => Ok(()),
            }
        });
        if let Err(error) = result {
            warn!("NetworkManager refused: {error}");
        }
        self.dirty = true;
    }

    /// After NetworkManager said something changed: read it again, and see how a join is going.
    fn refresh(&mut self, connection: &Connection) {
        self.read(connection);
        // The counters too, so data used keeps up as links come and go.
        self.last_counters = self.sample();
        let counters = self.last_counters.clone();
        self.build(connection, &counters);
        self.follow_join(connection);
        self.usage(false);
    }

    /// What the charts show, measured while the panel is open.
    fn measure_all(&mut self, connection: &Connection, probes: &Mutex<Probes>, settings: &Settings) {
        self.samples += 1;
        self.last_counters = self.sample();
        let counters = self.last_counters.clone();
        self.build(connection, &counters);
        self.measure(&counters);
        self.probes(probes, settings);
        if self.samples % SOCKETS_EVERY == 1 {
            let (sockets, ports) = kernel::sockets_and_ports();
            self.state.sockets = sockets;
            self.state.ports = ports;
            self.state.routes = kernel::routes();
        }
        self.usage(false);
    }

    /// A scan when the panel opens, unless one ran lately.
    fn opened(&mut self, connection: &Connection, sender: &UnboundedSender<Request>) {
        self.samples = 0;
        let stale = self
            .wifi_device()
            .and_then(|device| device.wireless.as_ref())
            .is_some_and(|wireless| {
                chrono::Utc::now().timestamp_millis() - boot_ms_to_unix(wireless.last_scan)
                    > SCAN_STALE_MS
            });
        if stale && self.snapshot.wireless_enabled {
            self.handle(connection, sender, Request::Scan);
        }
    }
}

/// NetworkManager gives LastScan in milliseconds of CLOCK_BOOTTIME.
fn boot_ms_to_unix(boot_ms: i64) -> i64 {
    let uptime_ms = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse::<f64>().ok())
        .map_or(0, |seconds| (seconds * 1000.0) as i64);
    chrono::Utc::now().timestamp_millis() - uptime_ms + boot_ms
}

fn saved_changed(interface: Option<&str>, member: Option<&str>) -> bool {
    matches!(
        (interface, member),
        (Some(nm::SETTINGS), Some("NewConnection" | "ConnectionRemoved"))
            | (Some(nm::CONNECTION), Some("Updated"))
    )
}

fn listen(connection: &Connection, rule: &str) -> zbus::Result<MessageStream> {
    block_on(async { MessageStream::for_match_rule(MatchRule::try_from(rule)?, connection, None).await })
}

pub(super) fn run(
    feed: &Feed<State>,
    requests: &Mutex<UnboundedReceiver<Request>>,
    sender: &UnboundedSender<Request>,
    watched: &Watched,
    probes: &Mutex<Probes>,
    settings: &Settings,
) -> Ended {
    let mut requests = requests.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = match block_on(Connection::system()) {
        Ok(connection) => connection,
        Err(error) => return Ended::Failed(format!("cannot connect to the system bus: {error}")),
    };
    let (signals, owner) = match (
        listen(&connection, "type='signal',sender='org.freedesktop.NetworkManager'"),
        listen(
            &connection,
            "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.freedesktop.NetworkManager'",
        ),
    ) {
        (Ok(signals), Ok(owner)) => (signals, owner),
        (Err(error), _) | (_, Err(error)) => {
            return Ended::Failed(format!("cannot follow NetworkManager: {error}"));
        }
    };
    let signals = signals.map(|message| {
        let saved = message.as_ref().is_ok_and(|message| {
            let header = message.header();
            saved_changed(
                header.interface().map(|name| name.as_str()),
                header.member().map(|name| name.as_str()),
            )
        });
        Event::Bus(saved)
    });
    let bus = stream::select(signals, owner.map(|_| Event::Bus(true)));
    let mut events = stream::select(bus, (&mut *requests).map(Event::Request));
    let mut world = World::default();
    world.refresh(&connection);
    world.usage(true);
    if !feed.publish(world.state.clone()) {
        return Ended::Unwanted;
    }
    world.published = Some(Glance::of(&world.state));
    loop {
        let Some(first) = block_on(events.next()) else {
            return Ended::Unwanted;
        };
        if matches!(first, Event::Bus(_)) {
            thread::sleep(SETTLE);
        }
        // Whatever else has arrived meanwhile is handled with it, so a burst means one read.
        let mut batch = vec![first];
        while let Some(Some(more)) = events.next().now_or_never() {
            batch.push(more);
        }
        let (mut changed, mut sample) = (false, false);
        for event in batch {
            match event {
                Event::Bus(saved) => {
                    world.dirty |= saved;
                    changed = true;
                }
                Event::Request(Request::Refresh) => changed = true,
                Event::Request(Request::Sample) => sample = true,
                Event::Request(Request::Watch) => {
                    world.opened(&connection, sender);
                    (changed, sample) = (true, true);
                }
                Event::Request(Request::Midnight) => {
                    world.last_counters = world.sample();
                    world.usage(true);
                }
                Event::Request(request) => {
                    world.handle(&connection, sender, request);
                    changed = true;
                }
            }
        }
        let watching = watched.watching();
        if changed {
            world.refresh(&connection);
        }
        if sample && watching {
            world.measure_all(&connection, probes, settings);
        }
        let glance = Glance::of(&world.state);
        if (watching || world.published.as_ref() != Some(&glance))
            && !feed.publish(world.state.clone())
        {
            return Ended::Unwanted;
        }
        world.published = Some(glance);
    }
}

#[cfg(test)]
#[path = "worker_test.rs"]
mod tests;
