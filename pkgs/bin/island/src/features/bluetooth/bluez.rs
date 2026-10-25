//! BlueZ over the system bus: its adapter and devices, read at once, and asking it to change.

use std::collections::HashMap;

use zbus::{Connection, ObjectPath, OwnedObjectPath, OwnedVariant, Variant};

use super::state::Device;
use crate::dbus::uncached;

pub const BLUEZ: &str = "org.bluez";
const ADAPTER: &str = "org.bluez.Adapter1";
const DEVICE: &str = "org.bluez.Device1";
const BATTERY: &str = "org.bluez.Battery1";
const TRANSPORT: &str = "org.bluez.MediaTransport1";
const AGENT_MANAGER: &str = "org.bluez.AgentManager1";

pub type Objects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedVariant>>>;

/// The adapter as BlueZ has it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Adapter {
    pub path: String,
    pub name: String,
    pub address: String,
    pub powered: bool,
    pub discovering: bool,
    pub discoverable: bool,
}

fn get<T: TryFrom<OwnedVariant>>(props: &HashMap<String, OwnedVariant>, key: &str) -> Option<T> {
    props.get(key)?.try_clone().ok()?.try_into().ok()
}

const AUDIO_SINK: &str = "0000110b";
const AUDIO_SOURCE: &str = "0000110a";
const HANDS_FREE: &str = "0000111e";
const HEADSET: &str = "00001108";
const HID: &str = "00001124";
const HOG: &str = "00001812";

/// What a device is used for, from the services it offers.
pub fn service(uuids: &[String]) -> &'static str {
    let has = |prefix: &str| uuids.iter().any(|uuid| uuid.starts_with(prefix));
    if has(AUDIO_SINK) {
        "Music"
    } else if has(HID) || has(HOG) {
        "Input"
    } else if has(HANDS_FREE) || has(HEADSET) {
        "Calls"
    } else if has(AUDIO_SOURCE) {
        "Audio"
    } else {
        ""
    }
}

/// A2DP's own codecs; a vendor's, such as LDAC, read as nothing.
fn codec(id: u8) -> Option<&'static str> {
    match id {
        0 => Some("SBC"),
        1 => Some("MP3"),
        2 => Some("AAC"),
        _ => None,
    }
}

/// The first adapter and its devices, with batteries and codecs folded in.
pub fn parse(objects: &Objects) -> Option<(Adapter, Vec<Device>)> {
    let (path, props) = objects
        .iter()
        .filter_map(|(path, interfaces)| Some((path, interfaces.get(ADAPTER)?)))
        .min_by(|a, b| a.0.as_str().cmp(b.0.as_str()))?;
    let adapter = Adapter {
        path: path.as_str().to_owned(),
        name: get(props, "Alias")
            .or_else(|| get(props, "Name"))
            .unwrap_or_default(),
        address: get(props, "Address").unwrap_or_default(),
        powered: get(props, "Powered").unwrap_or(false),
        discovering: get(props, "Discovering").unwrap_or(false),
        discoverable: get(props, "Discoverable").unwrap_or(false),
    };
    let codecs: HashMap<String, &'static str> = objects
        .values()
        .filter_map(|interfaces| {
            let transport = interfaces.get(TRANSPORT)?;
            let device: OwnedObjectPath = get(transport, "Device")?;
            let codec = codec(get(transport, "Codec")?)?;
            Some((device.as_str().to_owned(), codec))
        })
        .collect();
    let mut devices: Vec<Device> = objects
        .iter()
        .filter_map(|(path, interfaces)| {
            let props = interfaces.get(DEVICE)?;
            let owner: OwnedObjectPath = get(props, "Adapter")?;
            if owner.as_str() != adapter.path {
                return None;
            }
            let address: String = get(props, "Address").unwrap_or_default();
            let name: String = get(props, "Alias")
                .or_else(|| get(props, "Name"))
                .unwrap_or_else(|| address.clone());
            let uuids: Vec<String> = get(props, "UUIDs").unwrap_or_default();
            Some(Device {
                path: path.as_str().to_owned(),
                name,
                icon: get(props, "Icon").unwrap_or_default(),
                address,
                paired: get(props, "Paired").unwrap_or(false),
                trusted: get(props, "Trusted").unwrap_or(false),
                connected: get(props, "Connected").unwrap_or(false),
                battery: interfaces
                    .get(BATTERY)
                    .and_then(|battery| get(battery, "Percentage")),
                rssi: get(props, "RSSI"),
                service: service(&uuids).to_owned(),
                codec: codecs.get(path.as_str()).map(|codec| (*codec).to_owned()),
                ..Device::default()
            })
        })
        .collect();
    // Paired first, then nearby by signal, strongest first.
    devices.sort_by(|a, b| {
        (b.paired, b.connected, b.rssi.unwrap_or(i16::MIN))
            .cmp(&(a.paired, a.connected, a.rssi.unwrap_or(i16::MIN)))
            .then_with(|| a.name.cmp(&b.name))
    });
    Some((adapter, devices))
}

pub async fn objects(connection: &Connection) -> zbus::Result<Objects> {
    uncached(connection, BLUEZ, "/", "org.freedesktop.DBus.ObjectManager")
        .await?
        .call("GetManagedObjects", &())
        .await
}

pub async fn set_adapter<'a>(
    connection: &Connection,
    adapter: &str,
    name: &str,
    value: impl Into<Variant<'a>>,
) -> zbus::Result<()> {
    uncached(connection, BLUEZ, adapter.to_owned(), ADAPTER)
        .await?
        .set_property(name, value.into())
        .await
        .map_err(Into::into)
}

pub async fn adapter_call(connection: &Connection, adapter: &str, method: &str) -> zbus::Result<()> {
    uncached(connection, BLUEZ, adapter.to_owned(), ADAPTER)
        .await?
        .call(method, &())
        .await
}

pub async fn remove(connection: &Connection, adapter: &str, device: &str) -> zbus::Result<()> {
    let device = ObjectPath::try_from(device)?;
    uncached(connection, BLUEZ, adapter.to_owned(), ADAPTER)
        .await?
        .call("RemoveDevice", &(device,))
        .await
}

pub async fn device_call(connection: &Connection, device: &str, method: &str) -> zbus::Result<()> {
    uncached(connection, BLUEZ, device.to_owned(), DEVICE)
        .await?
        .call(method, &())
        .await
}

pub async fn set_trusted(connection: &Connection, device: &str, trusted: bool) -> zbus::Result<()> {
    uncached(connection, BLUEZ, device.to_owned(), DEVICE)
        .await?
        .set_property("Trusted", trusted)
        .await
        .map_err(Into::into)
}

/// Makes island the agent BlueZ asks about pairing.
pub async fn register_agent(connection: &Connection, path: &str) -> zbus::Result<()> {
    let manager = uncached(connection, BLUEZ, "/org/bluez", AGENT_MANAGER).await?;
    let agent = ObjectPath::try_from(path)?;
    manager
        .call::<_, _, ()>("RegisterAgent", &(agent.clone(), "KeyboardDisplay"))
        .await?;
    manager.call("RequestDefaultAgent", &(agent,)).await
}
