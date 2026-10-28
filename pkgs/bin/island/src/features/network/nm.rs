//! NetworkManager over the system bus: what it manages, and asking it to change.

use std::collections::HashMap;

use zbus::{Connection, ObjectPath, OwnedObjectPath, OwnedVariant as OwnedValue, Variant as Value};

use super::state::{AccessPoint, Activity, Security};
use crate::dbus::uncached;

const NM: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
pub(super) const SETTINGS: &str = "org.freedesktop.NetworkManager.Settings";
const SETTINGS_PATH: &str = "/org/freedesktop/NetworkManager/Settings";
pub(super) const CONNECTION: &str = "org.freedesktop.NetworkManager.Settings.Connection";
const DEVICE: &str = "org.freedesktop.NetworkManager.Device";
const WIRELESS: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const WIRED: &str = "org.freedesktop.NetworkManager.Device.Wired";
const ACCESS_POINT: &str = "org.freedesktop.NetworkManager.AccessPoint";
const ACTIVE: &str = "org.freedesktop.NetworkManager.Connection.Active";
const IP4: &str = "org.freedesktop.NetworkManager.IP4Config";
const IP6: &str = "org.freedesktop.NetworkManager.IP6Config";
const DHCP4: &str = "org.freedesktop.NetworkManager.DHCP4Config";

const ETHERNET: u32 = 1;
const WIFI: u32 = 2;
const ACTIVATED: u32 = 100;
const ACTIVE_ACTIVATING: u32 = 1;
const ACTIVE_ACTIVATED: u32 = 2;
/// An active connection's flag for one NetworkManager only follows.
const ACTIVE_EXTERNAL: u32 = 0x80;

const SEC_KEY_MGMT_802_1X: u32 = 0x200;
const SEC_KEY_MGMT_SAE: u32 = 0x400;
const PRIVACY: u32 = 0x1;

type Settings = HashMap<String, HashMap<String, OwnedValue>>;

/// A connection NetworkManager keeps.
#[derive(Clone, Debug)]
pub struct Saved {
    pub path: OwnedObjectPath,
    pub name: String,
    pub kind: String,
    pub ssid: Option<String>,
    pub endpoint: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Active {
    pub path: OwnedObjectPath,
    pub saved: OwnedObjectPath,
    pub name: String,
    pub activity: Activity,
    pub devices: Vec<OwnedObjectPath>,
    pub ip4: OwnedObjectPath,
    /// It carries the default route, so everything goes through it, such as a full VPN.
    pub default: bool,
    /// Something other than NetworkManager set it up, such as Docker's bridge.
    pub external: bool,
}

/// Where an active connection sends traffic, and who answers its lookups.
#[derive(Clone, Debug, Default)]
pub struct Reach {
    /// Networks routed through it, such as "10.10.0.0/16", without the default route.
    pub networks: Vec<String>,
    pub dns: Vec<String>,
    /// It routes a default route, in any table, such as Proton's, which keeps its own table and
    /// steers everything to it, so NetworkManager does not call it the default.
    pub everything: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Address {
    pub ipv4: Option<String>,
    pub prefix: u32,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
    pub domain: Option<String>,
    pub ipv6: Option<String>,
    /// Unix seconds the DHCP lease runs out.
    pub lease: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct Device {
    pub path: OwnedObjectPath,
    pub interface: String,
    pub kind: u32,
    pub activated: bool,
    /// NetworkManager runs it, so it can bring it up and down.
    pub managed: bool,
    pub mtu: u32,
    pub mac: String,
    pub active: OwnedObjectPath,
    pub address: Address,
    pub wireless: Option<WirelessDevice>,
    pub wired: Option<WiredDevice>,
}

#[derive(Clone, Debug)]
pub struct WirelessDevice {
    pub bitrate: u32,
    pub last_scan: i64,
    pub active_ap: OwnedObjectPath,
    pub access_points: Vec<(OwnedObjectPath, AccessPoint)>,
}

#[derive(Clone, Copy, Debug)]
pub struct WiredDevice {
    pub carrier: bool,
    pub speed: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub wireless_enabled: bool,
    /// NetworkManager's own check of the internet: 1 none, 2 a sign-in page, 3 limited, 4 full,
    /// 0 when it does not check.
    pub connectivity: u32,
    pub primary: Option<OwnedObjectPath>,
    pub devices: Vec<Device>,
    pub active: Vec<Active>,
}

pub fn kind_label(kind: u32) -> &'static str {
    match kind {
        1 => "Ethernet",
        2 => "Wi-Fi",
        5 => "Bluetooth",
        13 => "Bridge",
        14 => "Generic",
        16 => "Tunnel",
        22 => "Veth",
        29 => "WireGuard",
        32 => "Loopback",
        _ => "Other",
    }
}

fn is_root(path: &OwnedObjectPath) -> bool {
    path.as_str() == "/"
}

fn root() -> ObjectPath<'static> {
    ObjectPath::from_static_str_unchecked("/")
}

fn owned<T>(value: Option<&OwnedValue>) -> Option<T>
where
    T: TryFrom<OwnedValue>,
{
    value?.try_clone().ok()?.try_into().ok()
}

fn text(settings: &Settings, group: &str, key: &str) -> Option<String> {
    owned(settings.get(group)?.get(key))
}

fn ssid(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

async fn get<T>(
    connection: &Connection,
    path: &str,
    interface: &'static str,
    name: &str,
) -> zbus::Result<T>
where
    T: serde::de::DeserializeOwned + zbus::Type,
{
    uncached(connection, NM, path.to_owned(), interface)
        .await?
        .get_property(name)
        .await
}

/// Each saved connection, without its secrets.
pub async fn saved(connection: &Connection) -> zbus::Result<Vec<Saved>> {
    let settings = uncached(connection, NM, SETTINGS_PATH, SETTINGS).await?;
    let paths: Vec<OwnedObjectPath> = settings.call("ListConnections", &()).await?;
    let mut all = Vec::new();
    for path in paths {
        let proxy = uncached(connection, NM, path.as_str().to_owned(), CONNECTION).await?;
        let Ok(found) = proxy.call::<_, _, Settings>("GetSettings", &()).await else {
            continue;
        };
        let kind = text(&found, "connection", "type").unwrap_or_default();
        let ssid = owned::<Vec<u8>>(
            found
                .get("802-11-wireless")
                .and_then(|group| group.get("ssid")),
        )
        .map(|bytes| ssid(&bytes));
        let endpoint = text(&found, "vpn", "remote")
            .or_else(|| {
                found
                    .get("vpn")
                    .and_then(|group| owned::<HashMap<String, String>>(group.get("data")))
                    .and_then(|data| {
                        ["remote", "gateway", "server"]
                            .into_iter()
                            .find_map(|key| data.get(key).cloned())
                    })
            })
            .or_else(|| {
                // WireGuard keeps its server as the first peer's endpoint.
                found
                    .get("wireguard")
                    .and_then(|group| {
                        owned::<Vec<HashMap<String, OwnedValue>>>(group.get("peers"))
                    })
                    .and_then(|peers| peers.first().and_then(|peer| owned(peer.get("endpoint"))))
            });
        all.push(Saved {
            path,
            name: text(&found, "connection", "id").unwrap_or_default(),
            kind,
            ssid,
            endpoint,
        });
    }
    Ok(all)
}

fn security(flags: u32, wpa: u32, rsn: u32) -> Security {
    if (wpa | rsn) & SEC_KEY_MGMT_802_1X != 0 {
        Security::Enterprise
    } else if rsn & SEC_KEY_MGMT_SAE != 0 {
        Security::Wpa3
    } else if wpa | rsn != 0 || flags & PRIVACY != 0 {
        Security::Wpa2
    } else {
        Security::Open
    }
}

async fn access_point(connection: &Connection, path: &str) -> zbus::Result<AccessPoint> {
    let proxy = uncached(connection, NM, path.to_owned(), ACCESS_POINT).await?;
    let bytes: Vec<u8> = proxy.get_property("Ssid").await?;
    let flags: u32 = proxy.get_property("Flags").await?;
    let wpa: u32 = proxy.get_property("WpaFlags").await?;
    let rsn: u32 = proxy.get_property("RsnFlags").await?;
    Ok(AccessPoint {
        ssid: ssid(&bytes),
        strength: proxy.get_property("Strength").await?,
        frequency: proxy.get_property("Frequency").await?,
        security: security(flags, wpa, rsn),
        saved: false,
        active: false,
        bssid: proxy.get_property("HwAddress").await.unwrap_or_default(),
        max_bitrate: proxy.get_property::<u32>("MaxBitrate").await.unwrap_or(0) / 1000,
        width: proxy.get_property("Bandwidth").await.unwrap_or(0),
    })
}

fn address_of(entry: &HashMap<String, OwnedValue>) -> Option<String> {
    owned(entry.get("address"))
}

async fn lease(connection: &Connection, dhcp4: &OwnedObjectPath) -> Option<i64> {
    if is_root(dhcp4) {
        return None;
    }
    let options: HashMap<String, OwnedValue> = get(connection, dhcp4.as_str(), DHCP4, "Options")
        .await
        .ok()?;
    owned::<String>(options.get("expiry"))?.parse().ok()
}

async fn address(connection: &Connection, ip4: &OwnedObjectPath, ip6: &OwnedObjectPath) -> Address {
    let mut address = Address::default();
    if !is_root(ip4) {
        let path = ip4.as_str();
        if let Ok(entries) =
            get::<Vec<HashMap<String, OwnedValue>>>(connection, path, IP4, "AddressData").await
            && let Some(first) = entries.first()
        {
            address.ipv4 = address_of(first);
            address.prefix = owned(first.get("prefix")).unwrap_or(0);
        }
        address.gateway = get::<String>(connection, path, IP4, "Gateway")
            .await
            .ok()
            .filter(|gateway| !gateway.is_empty());
        address.dns =
            get::<Vec<HashMap<String, OwnedValue>>>(connection, path, IP4, "NameserverData")
                .await
                .map(|entries| entries.iter().filter_map(address_of).collect())
                .unwrap_or_default();
        address.domain = get::<Vec<String>>(connection, path, IP4, "Domains")
            .await
            .ok()
            .and_then(|domains| domains.into_iter().next());
    }
    if !is_root(ip6)
        && let Ok(entries) =
            get::<Vec<HashMap<String, OwnedValue>>>(connection, ip6.as_str(), IP6, "AddressData")
                .await
    {
        address.ipv6 = entries
            .iter()
            .filter_map(address_of)
            .find(|ip| !ip.starts_with("fe80"));
    }
    address
}

async fn device(connection: &Connection, path: OwnedObjectPath) -> zbus::Result<Device> {
    let proxy = uncached(connection, NM, path.as_str().to_owned(), DEVICE).await?;
    let kind: u32 = proxy.get_property("DeviceType").await?;
    let state: u32 = proxy.get_property("State").await?;
    let ip4: OwnedObjectPath = proxy.get_property("Ip4Config").await?;
    let ip6: OwnedObjectPath = proxy.get_property("Ip6Config").await?;
    let wireless = if kind == WIFI {
        let wifi = uncached(connection, NM, path.as_str().to_owned(), WIRELESS).await?;
        let aps: Vec<OwnedObjectPath> = wifi.call("GetAllAccessPoints", &()).await?;
        let mut access_points = Vec::new();
        for ap in aps {
            if let Ok(found) = access_point(connection, ap.as_str()).await {
                access_points.push((ap, found));
            }
        }
        Some(WirelessDevice {
            bitrate: wifi.get_property::<u32>("Bitrate").await.unwrap_or(0) / 1000,
            last_scan: wifi.get_property("LastScan").await.unwrap_or(0),
            active_ap: wifi.get_property("ActiveAccessPoint").await?,
            access_points,
        })
    } else {
        None
    };
    let wired = if kind == ETHERNET {
        let wired = uncached(connection, NM, path.as_str().to_owned(), WIRED).await?;
        Some(WiredDevice {
            carrier: wired.get_property("Carrier").await.unwrap_or(false),
            speed: wired.get_property("Speed").await.unwrap_or(0),
        })
    } else {
        None
    };
    Ok(Device {
        interface: proxy.get_property("Interface").await?,
        kind,
        activated: state == ACTIVATED,
        managed: proxy.get_property("Managed").await.unwrap_or(true),
        mtu: proxy.get_property("Mtu").await.unwrap_or(0),
        mac: proxy.get_property("HwAddress").await.unwrap_or_default(),
        active: proxy.get_property("ActiveConnection").await?,
        address: if state == ACTIVATED {
            let dhcp4: OwnedObjectPath = proxy
                .get_property("Dhcp4Config")
                .await
                .unwrap_or_else(|_| root().into());
            Address {
                lease: lease(connection, &dhcp4).await,
                ..address(connection, &ip4, &ip6).await
            }
        } else {
            Address::default()
        },
        wireless,
        wired,
        path,
    })
}

async fn active(connection: &Connection, path: OwnedObjectPath) -> zbus::Result<Active> {
    let proxy = uncached(connection, NM, path.as_str().to_owned(), ACTIVE).await?;
    let state: u32 = proxy.get_property("State").await?;
    Ok(Active {
        saved: proxy.get_property("Connection").await?,
        name: proxy.get_property("Id").await?,
        activity: match state {
            ACTIVE_ACTIVATING => Activity::Connecting,
            ACTIVE_ACTIVATED => Activity::On,
            _ => Activity::Off,
        },
        devices: proxy.get_property("Devices").await.unwrap_or_default(),
        ip4: proxy.get_property("Ip4Config").await?,
        default: proxy.get_property("Default").await.unwrap_or(false),
        external: proxy
            .get_property::<u32>("StateFlags")
            .await
            .is_ok_and(|flags| flags & ACTIVE_EXTERNAL != 0),
        path,
    })
}

/// The networks an active connection routes, and its DNS servers.
pub async fn reach(connection: &Connection, ip4: &OwnedObjectPath) -> Reach {
    if is_root(ip4) {
        return Reach::default();
    }
    let path = ip4.as_str();
    let routes: Vec<(String, u32)> =
        get::<Vec<HashMap<String, OwnedValue>>>(connection, path, IP4, "RouteData")
            .await
            .map(|routes| {
                routes
                    .iter()
                    .filter_map(|route| {
                        let destination: String = owned(route.get("dest"))?;
                        let prefix: u32 = owned(route.get("prefix"))?;
                        Some((destination, prefix))
                    })
                    .collect()
            })
            .unwrap_or_default();
    let dns = get::<Vec<HashMap<String, OwnedValue>>>(connection, path, IP4, "NameserverData")
        .await
        .map(|entries| entries.iter().filter_map(address_of).collect())
        .unwrap_or_default();
    Reach {
        everything: routes.iter().any(|(_, prefix)| *prefix == 0),
        networks: routes
            .into_iter()
            .filter(|(_, prefix)| *prefix > 0)
            .map(|(destination, prefix)| format!("{destination}/{prefix}"))
            .collect(),
        dns,
    }
}

/// Takes a device down, and keeps it down until asked for it again.
pub async fn device_disconnect(
    connection: &Connection,
    device: &OwnedObjectPath,
) -> zbus::Result<()> {
    uncached(connection, NM, device.as_str().to_owned(), DEVICE)
        .await?
        .call("Disconnect", &())
        .await
}

/// Brings a device up with the connection NetworkManager picks for it.
pub async fn device_activate(
    connection: &Connection,
    device: &OwnedObjectPath,
) -> zbus::Result<()> {
    uncached(connection, NM, NM_PATH, NM)
        .await?
        .call::<_, _, OwnedObjectPath>("ActivateConnection", &(root(), device, root()))
        .await
        .map(drop)
}

pub async fn snapshot(connection: &Connection) -> zbus::Result<Snapshot> {
    let manager = uncached(connection, NM, NM_PATH, NM).await?;
    let primary: OwnedObjectPath = manager.get_property("PrimaryConnection").await?;
    let devices: Vec<OwnedObjectPath> = manager.call("GetDevices", &()).await?;
    let actives: Vec<OwnedObjectPath> = manager.get_property("ActiveConnections").await?;
    let mut snapshot = Snapshot {
        wireless_enabled: manager.get_property("WirelessEnabled").await?,
        // Without its own check NetworkManager still reports a guess from the routes, which says
        // nothing about the internet, so it counts as not known.
        connectivity: if manager
            .get_property("ConnectivityCheckEnabled")
            .await
            .unwrap_or(false)
        {
            manager.get_property("Connectivity").await.unwrap_or(0)
        } else {
            0
        },
        primary: (!is_root(&primary)).then_some(primary),
        ..Snapshot::default()
    };
    for path in devices {
        if let Ok(found) = device(connection, path).await {
            snapshot.devices.push(found);
        }
    }
    for path in actives {
        if let Ok(found) = active(connection, path).await {
            snapshot.active.push(found);
        }
    }
    Ok(snapshot)
}

/// The address an active connection was given, such as a VPN's.
pub async fn active_address(connection: &Connection, ip4: &OwnedObjectPath) -> Option<String> {
    if is_root(ip4) {
        return None;
    }
    let entries =
        get::<Vec<HashMap<String, OwnedValue>>>(connection, ip4.as_str(), IP4, "AddressData")
            .await
            .ok()?;
    entries.first().and_then(address_of)
}

pub async fn set_wireless(connection: &Connection, enabled: bool) -> zbus::Result<()> {
    uncached(connection, NM, NM_PATH, NM)
        .await?
        .set_property("WirelessEnabled", enabled)
        .await
        .map_err(Into::into)
}

pub async fn scan(connection: &Connection, device: &OwnedObjectPath) -> zbus::Result<()> {
    let options: HashMap<&str, Value<'_>> = HashMap::new();
    uncached(connection, NM, device.as_str().to_owned(), WIRELESS)
        .await?
        .call("RequestScan", &(options,))
        .await
}

pub async fn activate(
    connection: &Connection,
    saved: &OwnedObjectPath,
    device: Option<&OwnedObjectPath>,
) -> zbus::Result<OwnedObjectPath> {
    let device = device.cloned().unwrap_or_else(|| root().into());
    uncached(connection, NM, NM_PATH, NM)
        .await?
        .call("ActivateConnection", &(saved, &device, root()))
        .await
}

/// Saves a connection for the network `ap` and brings it up; the saved connection and the active
/// one, so a failure can take the saved one back.
pub async fn join(
    connection: &Connection,
    device: &OwnedObjectPath,
    ap: &OwnedObjectPath,
    security: Security,
    password: &str,
) -> zbus::Result<(OwnedObjectPath, OwnedObjectPath)> {
    let mut settings: HashMap<&str, HashMap<&str, Value<'_>>> = HashMap::new();
    match security {
        Security::Wpa2 => {
            settings.insert(
                "802-11-wireless-security",
                HashMap::from([
                    ("key-mgmt", Value::from("wpa-psk")),
                    ("psk", Value::from(password)),
                ]),
            );
        }
        Security::Wpa3 => {
            settings.insert(
                "802-11-wireless-security",
                HashMap::from([
                    ("key-mgmt", Value::from("sae")),
                    ("psk", Value::from(password)),
                ]),
            );
        }
        Security::Open | Security::Enterprise => {}
    }
    uncached(connection, NM, NM_PATH, NM)
        .await?
        .call("AddAndActivateConnection", &(settings, device, ap))
        .await
}

pub async fn deactivate(connection: &Connection, active: &OwnedObjectPath) -> zbus::Result<()> {
    uncached(connection, NM, NM_PATH, NM)
        .await?
        .call("DeactivateConnection", &(active,))
        .await
}

pub async fn forget(connection: &Connection, saved: &OwnedObjectPath) -> zbus::Result<()> {
    uncached(connection, NM, saved.as_str().to_owned(), CONNECTION)
        .await?
        .call("Delete", &())
        .await
}

/// Where an active connection is: `None` once NetworkManager has let it go.
pub async fn activity(connection: &Connection, active: &OwnedObjectPath) -> Option<Activity> {
    let state: u32 = get(connection, active.as_str(), ACTIVE, "State")
        .await
        .ok()?;
    Some(match state {
        ACTIVE_ACTIVATING => Activity::Connecting,
        ACTIVE_ACTIVATED => Activity::On,
        _ => Activity::Off,
    })
}

/// Why a device last changed state, as NetworkManager numbers its reasons.
pub async fn reason(connection: &Connection, device: &OwnedObjectPath) -> Option<u32> {
    let (_, reason): (u32, u32) = get(connection, device.as_str(), DEVICE, "StateReason")
        .await
        .ok()?;
    Some(reason)
}

#[cfg(test)]
#[path = "nm_test.rs"]
mod tests;
