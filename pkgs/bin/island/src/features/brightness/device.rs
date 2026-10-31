//! The screens island can set the brightness of: a laptop's backlight through logind, and monitors
//! over DDC/CI. Each is named after its connector as the compositor names it, such as `DP-1`.

use std::{
    cmp, fs,
    os::unix::{fs::MetadataExt, io::AsRawFd},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use tracing::warn;
use zbus::{Connection, block_on};

use crate::dbus::uncached;

const BRIGHTNESS: u8 = 0x10;

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

enum Output {
    Backlight(PathBuf),
    /// With the maximum it reported when found, so setting it needs no read.
    Ddc {
        display: DdcDevice,
        maximum: u16,
    },
}

impl Screen {
    pub(super) fn read(&mut self) -> Option<f32> {
        match &mut self.output {
            Output::Backlight(device) => backlight_level(device),
            Output::Ddc { display, maximum } => match display.get_brightness() {
                Some((value, _)) => Some(f32::from(value) / f32::from(*maximum)),
                None => {
                    warn!(screen = %self.name, "cannot read the brightness over DDC/CI");
                    None
                }
            },
        }
    }

    pub(super) fn write(&mut self, level: f32) -> Option<()> {
        match &mut self.output {
            Output::Backlight(device) => set_backlight(device, level),
            Output::Ddc { display, maximum } => {
                let target = (level.clamp(0.0, 1.0) * f32::from(*maximum)).round() as u16;
                display.set_brightness(target).or_else(|| {
                    warn!(screen = %self.name, "cannot set the brightness over DDC/CI");
                    None
                })
            }
        }
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

/// Each connected connector's name and EDID.
fn connectors() -> Vec<(String, Vec<u8>)> {
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
            connected.then(|| (name, fs::read(path.join("edid")).unwrap_or_default()))
        })
        .collect()
}

/// `card1-DP-1` is `DP-1`; `card1` itself is no connector.
pub(super) fn connector_name(entry: &str) -> Option<String> {
    let (card, name) = entry.split_once('-')?;
    card.starts_with("card").then(|| name.to_owned())
}

/// The connector showing the monitor with `edid`.
pub(super) fn connector_for(connectors: &[(String, Vec<u8>)], edid: &[u8]) -> Option<String> {
    let base = edid.get(..EDID_BLOCK)?;
    connectors
        .iter()
        .find(|(_, theirs)| theirs.get(..EDID_BLOCK) == Some(base))
        .map(|(name, _)| name.clone())
}

fn backlight(connectors: &[(String, Vec<u8>)]) -> Option<Screen> {
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
                .map(|(name, _)| name)
                .find(|name| name.starts_with("eDP") || name.starts_with("LVDS"))
                .cloned()
        })
        .or_else(|| Some(device.file_name()?.to_string_lossy().into_owned()))?;
    Some(Screen {
        name,
        model: "Built-in display".to_owned(),
        output: Output::Backlight(device),
    })
}

fn monitors(connectors: &[(String, Vec<u8>)]) -> Vec<Screen> {
    let Ok(entries) = fs::read_dir("/sys/class/i2c-dev") else {
        return Vec::new();
    };
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
            let mut edid = vec![0; 2 * EDID_BLOCK];
            if display.read_edid(&mut edid).is_none() {
                return None;
            }
            let maximum = match display.get_brightness() {
                Some((_, max)) if max > 0 => max,
                Some(_) => return None,
                None => {
                    warn!(monitor = %id, "monitor does not report its brightness over DDC/CI");
                    return None;
                }
            };
            Some(Screen {
                name: connector_for(connectors, &edid).unwrap_or(id),
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
        if let Some(last) = self.last_command {
            let elapsed = last.elapsed();
            if elapsed < Duration::from_millis(50) {
                thread::sleep(Duration::from_millis(50) - elapsed);
            }
        }
    }

    fn read_edid(&mut self, edid: &mut [u8]) -> Option<()> {
        let first_len = cmp::min(EDID_BLOCK, edid.len());
        self.transfer_edid(0, &mut edid[..first_len])?;
        if edid.len() < 8 || edid[..8] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] {
            return None;
        }
        if edid.len() >= 2 * EDID_BLOCK && edid.get(126).copied().unwrap_or(0) > 0 {
            let _ = self.transfer_edid(EDID_BLOCK as u8, &mut edid[EDID_BLOCK..2 * EDID_BLOCK]);
        }
        Some(())
    }

    fn transfer_edid(&self, offset: u8, buf: &mut [u8]) -> Option<()> {
        let mut reg = [offset];
        let mut msgs = [
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
        ];
        let mut data = I2cRdwrIoctlData {
            msgs: msgs.as_mut_ptr(),
            nmsgs: 2,
        };
        (unsafe { libc::ioctl(self.file.as_raw_fd(), I2C_RDWR, &mut data) } >= 0).then_some(())
    }

    fn get_brightness(&mut self) -> Option<(u16, u16)> {
        self.wait_before_command();
        let cmd = [0x51, 0x82, 0x01, BRIGHTNESS, 0xAC];
        let mut write_msg = [I2cMsg {
            addr: I2C_ADDR_DDC,
            flags: 0,
            len: cmd.len() as u16,
            buf: cmd.as_ptr() as *mut u8,
        }];
        let mut write_data = I2cRdwrIoctlData {
            msgs: write_msg.as_mut_ptr(),
            nmsgs: 1,
        };
        if unsafe { libc::ioctl(self.file.as_raw_fd(), I2C_RDWR, &mut write_data) } < 0 {
            return None;
        }
        thread::sleep(Duration::from_millis(40));
        let mut resp = [0u8; 11];
        let mut read_msg = [I2cMsg {
            addr: I2C_ADDR_DDC,
            flags: I2C_M_RD,
            len: resp.len() as u16,
            buf: resp.as_mut_ptr(),
        }];
        let mut read_data = I2cRdwrIoctlData {
            msgs: read_msg.as_mut_ptr(),
            nmsgs: 1,
        };
        if unsafe { libc::ioctl(self.file.as_raw_fd(), I2C_RDWR, &mut read_data) } < 0 {
            return None;
        }
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
        let mut write_msg = [I2cMsg {
            addr: I2C_ADDR_DDC,
            flags: 0,
            len: cmd.len() as u16,
            buf: cmd.as_ptr() as *mut u8,
        }];
        let mut write_data = I2cRdwrIoctlData {
            msgs: write_msg.as_mut_ptr(),
            nmsgs: 1,
        };
        let res = (unsafe { libc::ioctl(self.file.as_raw_fd(), I2C_RDWR, &mut write_data) } >= 0)
            .then_some(());
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
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn backlight_level(device: &Path) -> Option<f32> {
    let current = read_number(&device.join("brightness"))? as f32;
    let max = read_number(&device.join("max_brightness"))? as f32;
    (max > 0.0).then(|| current / max)
}

fn set_backlight(device: &Path, level: f32) -> Option<()> {
    let max = read_number(&device.join("max_brightness"))?;
    let value = (level.clamp(0.01, 1.0) * max as f32).round() as u32;
    let name = device.file_name()?.to_string_lossy().into_owned();
    block_on(async {
        let connection = Connection::system().await.ok()?;
        let session = uncached(
            &connection,
            "org.freedesktop.login1",
            "/org/freedesktop/login1/session/auto",
            "org.freedesktop.login1.Session",
        )
        .await
        .ok()?;
        session
            .call_method("SetBrightness", &("backlight", name.as_str(), value))
            .await
            .ok()?;
        Some(())
    })
}

#[cfg(test)]
#[path = "device_test.rs"]
mod tests;
