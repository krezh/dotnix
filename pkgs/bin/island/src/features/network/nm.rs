//! NetworkManager over the system bus: what it manages, and asking it to change.

use std::collections::HashMap;

use zbus::{Connection, ObjectPath, OwnedObjectPath, OwnedVariant as OwnedValue, Variant as Value};

use super::state::{AccessPoint, Activity, Security};
use crate::dbus::uncached;

const NM: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
/// Where NetworkManager's object manager lists everything it exports.
const TREE_PATH: &str = "/org/freedesktop";
const OBJECT_MANAGER: &str = "org.freedesktop.DBus.ObjectManager";
const PROPERTIES: &str = "org.freedesktop.DBus.Properties";
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

pub(super) const ETHERNET: u32 = 1;
pub(super) const WIFI: u32 = 2;
const ACTIVATED: u32 = 100;
const ACTIVE_ACTIVATING: u32 = 1;
const ACTIVE_ACTIVATED: u32 = 2;

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

impl Saved {
    pub fn is_vpn(&self) -> bool {
        matches!(self.kind.as_str(), "vpn" | "wireguard")
    }
}

#[derive(Clone, Debug)]
pub struct Active {
    pub path: OwnedObjectPath,
    pub saved: OwnedObjectPath,
    pub name: String,
    pub activity: Activity,
    pub devices: Vec<OwnedObjectPath>,
    pub ip4: OwnedObjectPath,
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
        let endpoint = text(&found, "vpn", "remote").or_else(|| {
            found
                .get("vpn")
                .and_then(|group| owned::<HashMap<String, String>>(group.get("data")))
                .and_then(|data| {
                    ["remote", "gateway", "server"]
                        .into_iter()
                        .find_map(|key| data.get(key).cloned())
                })
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

type Properties = HashMap<String, OwnedValue>;

/// Every object NetworkManager exports, with each interface's properties, as one call gives them.
struct Tree(HashMap<OwnedObjectPath, HashMap<String, Properties>>);

impl Tree {
    async fn read(connection: &Connection) -> zbus::Result<Self> {
        uncached(connection, NM, TREE_PATH, OBJECT_MANAGER)
            .await?
            .call("GetManagedObjects", &())
            .await
            .map(Self)
    }

    fn properties(&self, path: &str, interface: &str) -> Option<&Properties> {
        self.0
            .iter()
            .find(|(at, _)| at.as_str() == path)?
            .1
            .get(interface)
    }

    fn get<T: TryFrom<OwnedValue>>(&self, path: &str, interface: &str, name: &str) -> Option<T> {
        owned(self.properties(path, interface)?.get(name))
    }
}

fn access_point(tree: &Tree, path: &str) -> Option<AccessPoint> {
    let get = |name| tree.properties(path, ACCESS_POINT)?.get(name);
    let flags: u32 = owned(get("Flags"))?;
    let wpa: u32 = owned(get("WpaFlags"))?;
    let rsn: u32 = owned(get("RsnFlags"))?;
    Some(AccessPoint {
        ssid: ssid(&owned::<Vec<u8>>(get("Ssid"))?),
        strength: owned(get("Strength"))?,
        frequency: owned(get("Frequency"))?,
        security: security(flags, wpa, rsn),
        saved: false,
        active: false,
        bssid: owned(get("HwAddress")).unwrap_or_default(),
        max_bitrate: owned::<u32>(get("MaxBitrate")).unwrap_or(0) / 1000,
        width: owned(get("Bandwidth")).unwrap_or(0),
    })
}

fn address_of(entry: &HashMap<String, OwnedValue>) -> Option<String> {
    owned(entry.get("address"))
}

fn lease(tree: &Tree, dhcp4: &OwnedObjectPath) -> Option<i64> {
    if is_root(dhcp4) {
        return None;
    }
    let options: Properties = tree.get(dhcp4.as_str(), DHCP4, "Options")?;
    owned::<String>(options.get("expiry"))?.parse().ok()
}

fn address(tree: &Tree, ip4: &OwnedObjectPath, ip6: &OwnedObjectPath) -> Address {
    let mut address = Address::default();
    if !is_root(ip4) {
        let path = ip4.as_str();
        if let Some(entries) = tree.get::<Vec<Properties>>(path, IP4, "AddressData")
            && let Some(first) = entries.first()
        {
            address.ipv4 = address_of(first);
            address.prefix = owned(first.get("prefix")).unwrap_or(0);
        }
        address.gateway = tree
            .get::<String>(path, IP4, "Gateway")
            .filter(|gateway| !gateway.is_empty());
        address.dns = tree
            .get::<Vec<Properties>>(path, IP4, "NameserverData")
            .map(|entries| entries.iter().filter_map(address_of).collect())
            .unwrap_or_default();
        address.domain = tree
            .get::<Vec<String>>(path, IP4, "Domains")
            .and_then(|domains| domains.into_iter().next());
    }
    if !is_root(ip6)
        && let Some(entries) = tree.get::<Vec<Properties>>(ip6.as_str(), IP6, "AddressData")
    {
        address.ipv6 = entries
            .iter()
            .filter_map(address_of)
            .find(|ip| !ip.starts_with("fe80"));
    }
    address
}

/// `None` for one gone since the tree was read.
async fn device(connection: &Connection, tree: &Tree, path: OwnedObjectPath) -> Option<Device> {
    let at = path.as_str();
    let get = |name| tree.properties(at, DEVICE)?.get(name);
    let kind: u32 = owned(get("DeviceType"))?;
    let state: u32 = owned(get("State"))?;
    let wireless = if kind == WIFI {
        // All of them, unlike the property, which leaves out hidden networks.
        let aps: Vec<OwnedObjectPath> = uncached(connection, NM, at.to_owned(), WIRELESS)
            .await
            .ok()?
            .call("GetAllAccessPoints", &())
            .await
            .ok()?;
        let wifi = |name| tree.properties(at, WIRELESS)?.get(name);
        Some(WirelessDevice {
            bitrate: owned::<u32>(wifi("Bitrate")).unwrap_or(0) / 1000,
            last_scan: owned(wifi("LastScan")).unwrap_or(0),
            active_ap: owned(wifi("ActiveAccessPoint"))?,
            access_points: aps
                .into_iter()
                .filter_map(|ap| {
                    let found = access_point(tree, ap.as_str())?;
                    Some((ap, found))
                })
                .collect(),
        })
    } else {
        None
    };
    let wired = (kind == ETHERNET).then(|| WiredDevice {
        carrier: tree.get(at, WIRED, "Carrier").unwrap_or(false),
        speed: tree.get(at, WIRED, "Speed").unwrap_or(0),
    });
    let ip4: OwnedObjectPath = owned(get("Ip4Config"))?;
    let ip6: OwnedObjectPath = owned(get("Ip6Config"))?;
    Some(Device {
        interface: owned(get("Interface"))?,
        kind,
        activated: state == ACTIVATED,
        mtu: owned(get("Mtu")).unwrap_or(0),
        mac: owned(get("HwAddress")).unwrap_or_default(),
        active: owned(get("ActiveConnection"))?,
        address: if state == ACTIVATED {
            let dhcp4: OwnedObjectPath = owned(get("Dhcp4Config")).unwrap_or_else(|| root().into());
            Address {
                lease: lease(tree, &dhcp4),
                ..address(tree, &ip4, &ip6)
            }
        } else {
            Address::default()
        },
        wireless,
        wired,
        path,
    })
}

fn active(tree: &Tree, path: OwnedObjectPath) -> Option<Active> {
    let get = |name| tree.properties(path.as_str(), ACTIVE)?.get(name);
    let state: u32 = owned(get("State"))?;
    Some(Active {
        saved: owned(get("Connection"))?,
        name: owned(get("Id"))?,
        activity: activity_of(state),
        devices: owned(get("Devices")).unwrap_or_default(),
        ip4: owned(get("Ip4Config"))?,
        path,
    })
}

/// Read in two calls and one more per Wi-Fi device, rather than one per property.
pub async fn snapshot(connection: &Connection) -> zbus::Result<Snapshot> {
    let manager: Properties = uncached(connection, NM, NM_PATH, PROPERTIES)
        .await?
        .call("GetAll", &(NM,))
        .await?;
    let tree = Tree::read(connection).await?;
    let get = |name| manager.get(name);
    let property = |name| zbus::Error::Failure(format!("NetworkManager does not say its {name}"));
    let primary: OwnedObjectPath =
        owned(get("PrimaryConnection")).ok_or_else(|| property("PrimaryConnection"))?;
    let devices: Vec<OwnedObjectPath> = owned(get("Devices")).unwrap_or_default();
    let actives: Vec<OwnedObjectPath> = owned(get("ActiveConnections")).unwrap_or_default();
    let mut snapshot = Snapshot {
        wireless_enabled: owned(get("WirelessEnabled"))
            .ok_or_else(|| property("WirelessEnabled"))?,
        // Without its own check NetworkManager still reports a guess from the routes, which says
        // nothing about the internet, so it counts as not known.
        connectivity: if owned(get("ConnectivityCheckEnabled")).unwrap_or(false) {
            owned(get("Connectivity")).unwrap_or(0)
        } else {
            0
        },
        primary: (!is_root(&primary)).then_some(primary),
        ..Snapshot::default()
    };
    for path in devices {
        if let Some(found) = device(connection, &tree, path).await {
            snapshot.devices.push(found);
        }
    }
    snapshot.active = actives
        .into_iter()
        .filter_map(|path| active(&tree, path))
        .collect();
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
    let key_mgmt = match security {
        Security::Wpa2 => Some("wpa-psk"),
        Security::Wpa3 => Some("sae"),
        Security::Open | Security::Enterprise => None,
    };
    if let Some(key_mgmt) = key_mgmt {
        settings.insert(
            "802-11-wireless-security",
            HashMap::from([
                ("key-mgmt", Value::from(key_mgmt)),
                ("psk", Value::from(password)),
            ]),
        );
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
    Some(activity_of(state))
}

fn activity_of(state: u32) -> Activity {
    match state {
        ACTIVE_ACTIVATING => Activity::Connecting,
        ACTIVE_ACTIVATED => Activity::On,
        _ => Activity::Off,
    }
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
