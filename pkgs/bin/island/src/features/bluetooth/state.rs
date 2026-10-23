//! What the Bluetooth feature knows, as the worker last read it from BlueZ.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// BlueZ answered and has an adapter.
    pub available: bool,
    pub powered: bool,
    pub discovering: bool,
    pub discoverable: bool,
    /// Unix seconds visibility ends, as island set it.
    pub visible_until: Option<i64>,
    pub name: String,
    pub address: String,
    /// Paired ones and ones found nearby, each once.
    pub devices: Vec<Device>,
    pub asking: Option<Asking>,
}

impl State {
    pub fn connected(&self) -> impl Iterator<Item = &Device> {
        self.devices
            .iter()
            .filter(|device| device.connected || device.busy == Some(Busy::Disconnecting))
    }

    pub fn paired_idle(&self) -> impl Iterator<Item = &Device> {
        self.devices.iter().filter(|device| {
            device.paired && !device.connected && device.busy != Some(Busy::Disconnecting)
        })
    }

    pub fn nearby(&self) -> impl Iterator<Item = &Device> {
        self.devices.iter().filter(|device| !device.paired)
    }

    /// The lowest battery of what is connected, for the bar.
    pub fn lowest_battery(&self) -> Option<u8> {
        self.devices
            .iter()
            .filter(|device| device.connected)
            .filter_map(|device| device.battery)
            .min()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Busy {
    Connecting,
    Disconnecting,
    Pairing,
}

impl Busy {
    pub fn label(self) -> &'static str {
        match self {
            Busy::Connecting => "Connecting…",
            Busy::Disconnecting => "Disconnecting…",
            Busy::Pairing => "Pairing…",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Device {
    /// BlueZ's object path, which requests name it by.
    pub path: String,
    pub name: String,
    /// BlueZ's icon name, such as "audio-headset".
    pub icon: String,
    pub address: String,
    pub paired: bool,
    pub trusted: bool,
    pub connected: bool,
    pub battery: Option<u8>,
    pub rssi: Option<i16>,
    pub busy: Option<Busy>,
    /// Unix seconds it connected, when island saw it.
    pub since: Option<i64>,
    /// What it is used for, such as "Music" or "Input".
    pub service: String,
    pub codec: Option<String>,
}

impl Device {
    /// What it is, from its icon.
    pub fn kind(&self) -> &'static str {
        match self.icon.as_str() {
            "audio-headset" | "audio-headphones" => "Headphones",
            "audio-card" => "Speaker",
            "input-gaming" => "Gamepad",
            "input-keyboard" => "Keyboard",
            "input-mouse" => "Mouse",
            "input-tablet" => "Tablet",
            "phone" => "Phone",
            "computer" => "Computer",
            "video-display" => "Display",
            "printer" => "Printer",
            "multimedia-player" => "Player",
            _ => "Device",
        }
    }
}

/// A device asking island about pairing, through the agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asking {
    pub id: u64,
    pub device: String,
    pub code: String,
    /// The code shows on the device as well, and pairing waits for it to be confirmed here;
    /// otherwise it is typed on the device.
    pub confirm: bool,
}
