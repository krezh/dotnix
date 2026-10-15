use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use ddc_hi::{Ddc, Display};
use zbus::blocking::Connection;

use crate::dbus::uncached;

const BRIGHTNESS: u8 = 0x10;

fn backlight() -> Option<PathBuf> {
    fs::read_dir("/sys/class/backlight").ok()?.flatten().map(|entry| entry.path()).next()
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
    let connection = Connection::system().ok()?;
    let session = uncached(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1/session/auto",
        "org.freedesktop.login1.Session",
    )
    .ok()?;
    session.call_method("SetBrightness", &("backlight", name.as_str(), value)).ok()?;
    Some(())
}

fn displays() -> &'static Mutex<Vec<Display>> {
    static DISPLAYS: OnceLock<Mutex<Vec<Display>>> = OnceLock::new();
    DISPLAYS.get_or_init(|| Mutex::new(Display::enumerate()))
}

fn ddc_level() -> Option<f32> {
    let mut displays = displays().lock().ok()?;
    let display = displays.first_mut()?;
    let value = display.handle.get_vcp_feature(BRIGHTNESS).ok()?;
    let max = f32::from(value.maximum());
    (max > 0.0).then(|| f32::from(value.value()) / max)
}

fn set_ddc(level: f32) -> Option<()> {
    let mut displays = displays().lock().ok()?;
    for display in displays.iter_mut() {
        let Ok(value) = display.handle.get_vcp_feature(BRIGHTNESS) else {
            continue;
        };
        let target = (level.clamp(0.0, 1.0) * f32::from(value.maximum())).round() as u16;
        let _ = display.handle.set_vcp_feature(BRIGHTNESS, target);
    }
    Some(())
}

pub fn get() -> Option<f32> {
    match backlight() {
        Some(device) => backlight_level(&device),
        None => ddc_level(),
    }
}

pub fn step(delta: f32) -> Option<f32> {
    let next = (get()? + delta).clamp(0.0, 1.0);
    match backlight() {
        Some(device) => set_backlight(&device, next)?,
        None => set_ddc(next)?,
    }
    Some(next)
}
