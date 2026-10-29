//! The screens island can set the brightness of: a laptop's backlight through logind, and monitors
//! over DDC/CI. Each is named after its connector as the compositor names it, such as `DP-1`.

use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use ddc::{Ddc, Edid};
use ddc_i2c::{I2cDeviceDdc, I2cDeviceEnumerator};
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
        display: I2cDeviceDdc,
        maximum: u16,
    },
}

impl Screen {
    pub(super) fn read(&mut self) -> Option<f32> {
        match &mut self.output {
            Output::Backlight(device) => backlight_level(device),
            Output::Ddc { display, maximum } => match display.get_vcp_feature(BRIGHTNESS) {
                Ok(value) => Some(f32::from(value.value()) / f32::from(*maximum)),
                Err(err) => {
                    warn!(screen = %self.name, ?err, "cannot read the brightness over DDC/CI");
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
                display
                    .set_vcp_feature(BRIGHTNESS, target)
                    .map_err(|err| {
                        warn!(screen = %self.name, ?err, "cannot set the brightness over DDC/CI");
                    })
                    .ok()
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
    let Ok(found) = I2cDeviceEnumerator::new() else {
        return Vec::new();
    };
    found
        .filter_map(|mut display| {
            let id = display
                .inner_ref()
                .inner_ref()
                .metadata()
                .map_or(0, |meta| meta.rdev())
                .to_string();
            let mut edid = vec![0; 2 * EDID_BLOCK];
            if let Err(err) = display.read_edid(0, &mut edid) {
                warn!(monitor = %id, ?err, "cannot read the monitor's EDID");
                return None;
            }
            let maximum = match display.get_vcp_feature(BRIGHTNESS) {
                Ok(value) if value.maximum() > 0 => value.maximum(),
                Ok(_) => return None,
                Err(err) => {
                    warn!(monitor = %id, ?err, "monitor does not report its brightness over DDC/CI");
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
