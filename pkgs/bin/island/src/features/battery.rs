//! A battery a feature knows of, such as a mouse's: the one reading its bar gauge and the
//! dashboard's Batteries card are both drawn from, with one color scale and one idea of low.

use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU8, Ordering},
        mpsc::Receiver,
    },
    time::{Duration, Instant, SystemTime},
};

use crate::{
    config::Battery as BatterySettings,
    features::{
        Features,
        notice::{self, Notice, NoticeAction, Origin, Post, Reply},
    },
    panel::SharedString,
    theme::{self, GREEN, PEACH, RED, TEXT, YELLOW},
};

/// At or below this, a battery not charging is worth a warning, unless the config says otherwise.
pub const LOW: u8 = 20;
pub const CRITICAL: u8 = 10;
pub const DEFAULT_ACTION: &str = "default";
/// How long a device that stops reporting keeps what was said about it, so a brief drop does not
/// bring back an alert that was dismissed.
const FORGET: Duration = Duration::from_secs(3600);

/// What counts as low everywhere a battery is drawn, set once from the config.
static LOW_AT: AtomicU8 = AtomicU8::new(LOW);

pub fn set_low(low: u8) {
    LOW_AT.store(low, Ordering::Relaxed);
}

fn low_at() -> u8 {
    LOW_AT.load(Ordering::Relaxed)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Battery {
    /// Names it across frames, such as a device's address.
    pub id: SharedString,
    /// The key of the glyph for what it powers.
    pub glyph: &'static str,
    pub name: SharedString,
    /// 0 to 100.
    pub level: u8,
    pub charging: bool,
}

/// Red when low, then peach, yellow and green as it fills.
pub fn level_color(level: u8) -> u32 {
    match level {
        level if level <= low_at() => RED,
        ..=40 => PEACH,
        ..=60 => YELLOW,
        _ => GREEN,
    }
}

impl Battery {
    pub fn fraction(&self) -> f32 {
        f32::from(self.level.min(100)) / 100.0
    }

    /// Green while charging, otherwise by how full it is.
    pub fn color(&self) -> u32 {
        if self.charging {
            GREEN
        } else {
            level_color(self.level)
        }
    }

    pub fn low(&self) -> bool {
        !self.charging && self.level <= low_at()
    }

    /// What the glyph beside its gauge is drawn in: green while charging, red when low.
    pub fn glyph_color(&self) -> u32 {
        match self {
            battery if battery.charging => GREEN,
            battery if battery.low() => theme::CRITICAL,
            _ => TEXT,
        }
    }

    pub fn percent(&self) -> String {
        format!("{}%", self.level)
    }
}

/// Worse further down: a device is alerted again only when it gets worse than it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertTier {
    Low,
    Critical,
}

struct DeviceState {
    alerted: Option<AlertTier>,
    dismissed: Option<AlertTier>,
    seen: Instant,
}

pub struct Monitor {
    settings: BatterySettings,
    devices: HashMap<SharedString, DeviceState>,
    replies: Receiver<Reply>,
    /// The batteries last looked at, so a change elsewhere costs nothing.
    last: Option<Vec<Battery>>,
}

impl Monitor {
    pub fn new(settings: BatterySettings) -> Self {
        Self::with_receiver(settings, notice::listen("battery"))
    }

    pub fn with_receiver(settings: BatterySettings, replies: Receiver<Reply>) -> Self {
        Self {
            settings,
            devices: HashMap::new(),
            replies,
            last: None,
        }
    }

    pub fn update(&mut self, features: &Features) {
        self.drain_replies();
        if !self.settings.notify {
            return;
        }
        let batteries = features.batteries();
        if self.last.as_ref() == Some(&batteries) {
            return;
        }
        self.update_batteries(&batteries);
        self.last = Some(batteries);
    }

    pub fn drain_replies(&mut self) {
        while let Ok(reply) = self.replies.try_recv() {
            if let Reply::Dismissed(key) = reply {
                if let Some(device) = self.devices.get_mut(key.as_str()) {
                    device.dismissed = device.alerted;
                }
            }
        }
    }

    pub fn update_batteries(&mut self, batteries: &[Battery]) {
        if !self.settings.notify {
            return;
        }
        let now = Instant::now();
        let mut active_keys = Vec::new();
        let current_ids: HashSet<&str> = batteries.iter().map(|b| b.id.as_str()).collect();
        self.devices
            .retain(|id, device| current_ids.contains(id.as_str()) || now - device.seen < FORGET);

        for battery in batteries {
            let device = self
                .devices
                .entry(battery.id.clone())
                .or_insert(DeviceState {
                    alerted: None,
                    dismissed: None,
                    seen: now,
                });
            device.seen = now;
            let tier = if battery.charging {
                device.alerted = None;
                device.dismissed = None;
                None
            } else if battery.level <= self.settings.critical {
                Some(AlertTier::Critical)
            } else if battery.level <= self.settings.low {
                Some(AlertTier::Low)
            } else {
                if battery.level > self.settings.low.saturating_add(2) {
                    device.alerted = None;
                    device.dismissed = None;
                }
                None
            };

            if let Some(tier) = tier {
                active_keys.push(battery.id.to_string());
                if Some(tier) > device.alerted.max(device.dismissed) {
                    let critical = tier == AlertTier::Critical;
                    let title = if critical {
                        format!("{} battery critical", battery.name)
                    } else {
                        format!("{} is low", battery.name)
                    };
                    let body = if critical {
                        format!("{} remaining — connect to power", battery.percent())
                    } else {
                        format!("{} remaining", battery.percent())
                    };
                    let icon = Some(
                        (if critical {
                            "battery-warning"
                        } else {
                            "battery-low"
                        })
                        .to_owned(),
                    );
                    notice::post(Post::Show(Notice {
                        origin: Origin {
                            feature: "battery".into(),
                            source: "Battery".into(),
                            key: battery.id.to_string(),
                        },
                        app: "Battery".into(),
                        title,
                        body,
                        critical,
                        detail: Some(battery.percent()),
                        via: false,
                        lead: false,
                        icon,
                        actions: vec![NoticeAction {
                            key: DEFAULT_ACTION.into(),
                            label: "View".into(),
                            opens: Some(("dashboard".into(), "overview".into())),
                        }],
                        received: SystemTime::now(),
                        quiet: false,
                        transient: true,
                    }));
                    device.alerted = Some(tier);
                    device.dismissed = None;
                }
            }
        }

        notice::post(Post::Keep {
            feature: "battery".into(),
            keys: active_keys,
        });
    }
}

#[cfg(test)]
#[path = "battery_test.rs"]
mod tests;
