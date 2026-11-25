//! The screens island can set the brightness of: a laptop's backlight through logind, and monitors
//! over DDC/CI. Each is named after its connector as the compositor names it, such as `DP-1`.
//!
//! The kernel announces every backlight change, wherever it came from; DDC/CI announces nothing, so a
//! monitor's level is as island last read or set it.

use std::{
    fs,
    os::unix::{fs::MetadataExt, io::AsRawFd},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use tracing::warn;
use zbus::{Connection, block_on};

use crate::{dbus::uncached, logind, sys::read_u64};

const BRIGHTNESS: u8 = 0x10;
/// The pause DDC/CI wants between commands.
const COMMAND_GAP: Duration = Duration::from_millis(50);

/// An EDID's base block, which tells monitors apart.
const EDID_BLOCK: usize = 128;
/// Where the base block's four 18-byte descriptors start.
const DESCRIPTORS: usize = 54;
const DESCRIPTOR: usize = 18;
const PRODUCT_NAME: u8 = 0xFC;

pub(super) struct Screen {
    pub(super) name: String,
    /// Such as the monitor's model; may be empty.
    pub(super) model: String,
    output: Output,
}

/// Each with the maximum it reported when found, so setting it needs no read.
enum Output {
    Backlight {
        device: PathBuf,
        maximum: u32,
        /// Kept between changes; dropped when a call on it fails.
        bus: Option<Connection>,
    },
    Ddc {
        display: DdcDevice,
        maximum: u16,
    },
}

impl Screen {
    pub(super) fn read(&mut self) -> Option<f32> {
        match &mut self.output {
            Output::Backlight {
                device, maximum, ..
            } => backlight_level(device, *maximum),
            Output::Ddc { display, maximum } => match display.get_brightness() {
                Some((value, _)) => Some(f32::from(value) / f32::from(*maximum)),
                None => {
                    warn!(screen = %self.name, "cannot read the brightness over DDC/CI");
                    None
                }
            },
        }
    }

    /// The level it was left at: a backlight's is read back, so its own change event matches it.
    pub(super) fn write(&mut self, level: f32) -> Option<f32> {
        match &mut self.output {
            Output::Backlight {
                device,
                maximum,
                bus,
            } => {
                set_backlight(device, *maximum, bus, level)?;
                backlight_level(device, *maximum).or(Some(level))
            }
            Output::Ddc { display, maximum } => {
                let target = (level.clamp(0.0, 1.0) * f32::from(*maximum)).round() as u16;
                match display.set_brightness(target) {
                    Some(()) => Some(level),
                    None => {
                        warn!(screen = %self.name, "cannot set the brightness over DDC/CI");
                        None
                    }
                }
            }
        }
    }

    pub(super) fn is_backlight(&self) -> bool {
        matches!(self.output, Output::Backlight { .. })
    }
}

/// Every screen there is, the backlight first. Slow with monitors, which answer DDC/CI slowly.
pub(super) fn find() -> Vec<Screen> {
    let connectors = connectors();
    backlight(&connectors)
        .into_iter()
        .chain(monitors(&connectors))
        .collect()
}

pub(super) struct Connector {
    pub(super) name: String,
    pub(super) edid: Vec<u8>,
    /// The i2c adapter its monitor answers DDC/CI on, such as `i2c-5`, when the driver says.
    pub(super) adapter: Option<String>,
}

/// Each connected connector.
fn connectors() -> Vec<Connector> {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = connector_name(&entry.file_name().to_string_lossy())?;
            let path = entry.path();
            let connected = fs::read_to_string(path.join("status"))
                .is_ok_and(|status| status.trim() == "connected");
            connected.then(|| Connector {
                name,
                edid: fs::read(path.join("edid")).unwrap_or_default(),
                adapter: fs::read_link(path.join("ddc"))
                    .ok()
                    .and_then(|link| Some(link.file_name()?.to_string_lossy().into_owned())),
            })
        })
        .collect()
}

/// `card1-DP-1` is `DP-1`; `card1` itself is no connector.
pub(super) fn connector_name(entry: &str) -> Option<String> {
    let (card, name) = entry.split_once('-')?;
    card.starts_with("card").then(|| name.to_owned())
}

/// The connector showing the monitor on i2c `adapter` with `edid`: the one whose DDC link is that
/// adapter, or else the first not yet `taken` with the same EDID base block, since identical
/// monitors without a serial share it.
pub(super) fn connector_for(
    connectors: &[Connector],
    adapter: &str,
    edid: &[u8],
    taken: &[String],
) -> Option<String> {
    if let Some(linked) = connectors
        .iter()
        .find(|connector| connector.adapter.as_deref() == Some(adapter))
    {
        return Some(linked.name.clone());
    }
    let base = edid.get(..EDID_BLOCK)?;
    connectors
        .iter()
        .filter(|connector| !taken.contains(&connector.name))
        .find(|connector| connector.edid.get(..EDID_BLOCK) == Some(base))
        .map(|connector| connector.name.clone())
}

fn backlight(connectors: &[Connector]) -> Option<Screen> {
    let device = fs::read_dir("/sys/class/backlight")
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .min()?;
    // Its device is the connector on some GPUs and the GPU itself on others.
    let name = fs::read_link(device.join("device"))
        .ok()
        .and_then(|link| connector_name(&link.file_name()?.to_string_lossy()))
        .or_else(|| {
            connectors
                .iter()
                .map(|connector| &connector.name)
                .find(|name| name.starts_with("eDP") || name.starts_with("LVDS"))
                .cloned()
        })
        .or_else(|| Some(device.file_name()?.to_string_lossy().into_owned()))?;
    let maximum = read_number(&device.join("max_brightness")).filter(|max| *max > 0)?;
    Some(Screen {
        name,
        model: "Built-in display".to_owned(),
        output: Output::Backlight {
            device,
            maximum,
            bus: None,
        },
    })
}

fn monitors(connectors: &[Connector]) -> Vec<Screen> {
    let Ok(entries) = fs::read_dir("/sys/class/i2c-dev") else {
        return Vec::new();
    };
    let mut taken = Vec::new();
    entries
        .flatten()
        .filter_map(|entry| {
            let name = fs::read_to_string(entry.path().join("name")).unwrap_or_default();
            let skip_patterns = ["SMBus", "soc:i2cdsi", "smu", "SMU", "mac-io", "u4"];
            if skip_patterns.iter().any(|p| name.contains(p)) {
                return None;
            }
            let dev_path = Path::new("/dev").join(entry.file_name());
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&dev_path)
                .ok()?;
            let id = file.metadata().map_or(0, |meta| meta.rdev()).to_string();
            let mut display = DdcDevice::new(file);
            let edid = display.read_edid()?;
            let maximum = match display.get_brightness() {
                Some((_, max)) if max > 0 => max,
                Some(_) => return None,
                None => {
                    warn!(monitor = %id, "monitor does not report its brightness over DDC/CI");
                    return None;
                }
            };
            let adapter = entry.file_name().to_string_lossy().into_owned();
            let name = match connector_for(connectors, &adapter, &edid, &taken) {
                Some(name) => {
                    taken.push(name.clone());
                    name
                }
                None => id,
            };
            Some(Screen {
                name,
                model: model_name(&edid).unwrap_or_default(),
                output: Output::Ddc { display, maximum },
            })
        })
        .collect()
}

const I2C_RDWR: libc::c_ulong = 0x0707;
const I2C_M_RD: u16 = 0x0001;
const I2C_ADDR_EDID: u16 = 0x50;
const I2C_ADDR_DDC: u16 = 0x37;

#[repr(C)]
struct I2cMsg {
    addr: u16,
    flags: u16,
    len: u16,
    buf: *mut u8,
}

#[repr(C)]
struct I2cRdwrIoctlData {
    msgs: *mut I2cMsg,
    nmsgs: u32,
}

struct DdcDevice {
    file: fs::File,
    last_command: Option<Instant>,
}

impl DdcDevice {
    fn new(file: fs::File) -> Self {
        Self {
            file,
            last_command: None,
        }
    }

    fn wait_before_command(&mut self) {
        if let Some(rest) = self
            .last_command
            .and_then(|last| COMMAND_GAP.checked_sub(last.elapsed()))
        {
            thread::sleep(rest);
        }
    }

    /// The base block and, when the monitor has one, the first extension block.
    fn read_edid(&mut self) -> Option<[u8; 2 * EDID_BLOCK]> {
        let mut edid = [0; 2 * EDID_BLOCK];
        let (base, extension) = edid.split_at_mut(EDID_BLOCK);
        self.transfer_edid(0, base)?;
        if base[..8] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] {
            return None;
        }
        if base[126] > 0 {
            let _ = self.transfer_edid(EDID_BLOCK as u8, extension);
        }
        Some(edid)
    }

    fn transfer(&self, msgs: &mut [I2cMsg]) -> Option<()> {
        let mut data = I2cRdwrIoctlData {
            msgs: msgs.as_mut_ptr(),
            nmsgs: msgs.len() as u32,
        };
        (unsafe { libc::ioctl(self.file.as_raw_fd(), I2C_RDWR, &mut data) } >= 0).then_some(())
    }

    fn transfer_edid(&self, offset: u8, buf: &mut [u8]) -> Option<()> {
        let mut reg = [offset];
        self.transfer(&mut [
            I2cMsg {
                addr: I2C_ADDR_EDID,
                flags: 0,
                len: 1,
                buf: reg.as_mut_ptr(),
            },
            I2cMsg {
                addr: I2C_ADDR_EDID,
                flags: I2C_M_RD,
                len: buf.len() as u16,
                buf: buf.as_mut_ptr(),
            },
        ])
    }

    fn get_brightness(&mut self) -> Option<(u16, u16)> {
        self.wait_before_command();
        let cmd = [0x51, 0x82, 0x01, BRIGHTNESS, 0xAC];
        self.transfer(&mut [I2cMsg {
            addr: I2C_ADDR_DDC,
            flags: 0,
            len: cmd.len() as u16,
            buf: cmd.as_ptr() as *mut u8,
        }])?;
        thread::sleep(Duration::from_millis(40));
        let mut resp = [0u8; 11];
        self.transfer(&mut [I2cMsg {
            addr: I2C_ADDR_DDC,
            flags: I2C_M_RD,
            len: resp.len() as u16,
            buf: resp.as_mut_ptr(),
        }])?;
        self.last_command = Some(Instant::now());
        if resp[2] != 0x02 || resp[3] != 0x00 || resp[4] != BRIGHTNESS {
            return None;
        }
        let chk = resp[1..10].iter().fold(0x6F ^ 0x51, |acc, &b| acc ^ b);
        if resp[10] != chk {
            return None;
        }
        let max = u16::from_be_bytes([resp[6], resp[7]]);
        let cur = u16::from_be_bytes([resp[8], resp[9]]);
        Some((cur, max))
    }

    fn set_brightness(&mut self, value: u16) -> Option<()> {
        self.wait_before_command();
        let [hi, lo] = value.to_be_bytes();
        let chk = 0x6E ^ 0x51 ^ 0x84 ^ 0x03 ^ BRIGHTNESS ^ hi ^ lo;
        let cmd = [0x51, 0x84, 0x03, BRIGHTNESS, hi, lo, chk];
        let res = self.transfer(&mut [I2cMsg {
            addr: I2C_ADDR_DDC,
            flags: 0,
            len: cmd.len() as u16,
            buf: cmd.as_ptr() as *mut u8,
        }]);
        self.last_command = Some(Instant::now());
        res
    }
}

/// The model name an EDID's base block gives, such as `DELL U2723QE`.
pub(super) fn model_name(edid: &[u8]) -> Option<String> {
    edid.get(DESCRIPTORS..EDID_BLOCK - 2)?
        .chunks_exact(DESCRIPTOR)
        .find(|descriptor| descriptor[..2] == [0, 0] && descriptor[3] == PRODUCT_NAME)
        .map(|descriptor| {
            descriptor[5..]
                .iter()
                .filter(|&&byte| byte != b'\n')
                .map(|&byte| char::from(byte))
                .collect::<String>()
                .trim()
                .to_owned()
        })
}

fn read_number(path: &Path) -> Option<u32> {
    read_u64(path)?.try_into().ok()
}

fn backlight_level(device: &Path, maximum: u32) -> Option<f32> {
    let current = read_number(&device.join("brightness"))?;
    Some(current as f32 / maximum as f32)
}

fn set_backlight(
    device: &Path,
    maximum: u32,
    bus: &mut Option<Connection>,
    level: f32,
) -> Option<()> {
    let value = (level.clamp(0.01, 1.0) * maximum as f32).round() as u32;
    let name = device.file_name()?.to_string_lossy().into_owned();
    let set = block_on(async {
        if bus.is_none() {
            *bus = Some(Connection::system().await.ok()?);
        }
        let connection = bus.clone()?;
        uncached(
            &connection,
            logind::NAME,
            logind::SESSION_PATH,
            logind::SESSION,
        )
        .await
        .ok()?
        .call_method("SetBrightness", &("backlight", name.as_str(), value))
        .await
        .ok()
    });
    if set.is_none() {
        *bus = None;
    }
    set.map(drop)
}

#[cfg(test)]
#[path = "device_test.rs"]
mod tests;
