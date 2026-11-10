use std::sync::mpsc;

use super::*;
use crate::config::Battery as BatterySettings;

fn device(id: &str, name: &str, level: u8, charging: bool) -> Battery {
    Battery {
        id: id.into(),
        glyph: "mouse",
        name: name.into(),
        level,
        charging,
    }
}

fn monitor() -> (Monitor, mpsc::Sender<Reply>) {
    let (tx, rx) = mpsc::channel();
    let monitor = Monitor::with_receiver(BatterySettings::default(), rx);
    (monitor, tx)
}

#[test]
fn device_above_low_posts_no_notice() {
    let (mut monitor, _) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 25, false)]);
    assert!(
        monitor
            .devices
            .get("mouse")
            .map_or(true, |d| d.alerted.is_none())
    );
}

#[test]
fn device_low_triggers_warning() {
    let (mut monitor, _) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 18, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Low)
    );
    assert_eq!(monitor.devices.get("mouse").unwrap().dismissed, None);
}

#[test]
fn subsequent_poll_does_not_duplicate() {
    let (mut monitor, _) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 18, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Low)
    );
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 17, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Low)
    );
}

#[test]
fn dismissal_suppresses_re_alerts() {
    let (mut monitor, tx) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 18, false)]);
    tx.send(Reply::Dismissed("mouse".into())).unwrap();
    monitor.drain_replies();
    assert_eq!(
        monitor.devices.get("mouse").unwrap().dismissed,
        Some(AlertTier::Low)
    );

    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 15, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Low)
    );
    assert_eq!(
        monitor.devices.get("mouse").unwrap().dismissed,
        Some(AlertTier::Low)
    );
}

#[test]
fn escalation_to_critical_overrides_dismissal() {
    let (mut monitor, tx) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 18, false)]);
    tx.send(Reply::Dismissed("mouse".into())).unwrap();
    monitor.drain_replies();

    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 8, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Critical)
    );
    assert_eq!(monitor.devices.get("mouse").unwrap().dismissed, None);
}

#[test]
fn charging_clears_alerts() {
    let (mut monitor, _) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 8, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Critical)
    );

    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 8, true)]);
    assert_eq!(monitor.devices.get("mouse").unwrap().alerted, None);
    assert_eq!(monitor.devices.get("mouse").unwrap().dismissed, None);
}

#[test]
fn unplugging_at_critical_re_alerts() {
    let (mut monitor, _) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 8, false)]);
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 8, true)]);
    assert_eq!(monitor.devices.get("mouse").unwrap().alerted, None);

    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 8, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Critical)
    );
}

#[test]
fn hysteresis_prevents_fluctuation_re_alerts() {
    let (mut monitor, tx) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 18, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Low)
    );

    tx.send(Reply::Dismissed("mouse".into())).unwrap();
    monitor.drain_replies();

    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 21, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Low)
    );
    assert_eq!(
        monitor.devices.get("mouse").unwrap().dismissed,
        Some(AlertTier::Low)
    );

    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 19, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Low)
    );
    assert_eq!(
        monitor.devices.get("mouse").unwrap().dismissed,
        Some(AlertTier::Low)
    );

    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 23, false)]);
    assert_eq!(monitor.devices.get("mouse").unwrap().alerted, None);
    assert_eq!(monitor.devices.get("mouse").unwrap().dismissed, None);

    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 19, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        Some(AlertTier::Low)
    );
    assert_eq!(monitor.devices.get("mouse").unwrap().dismissed, None);
}

#[test]
fn a_device_that_drops_out_briefly_keeps_its_dismissal() {
    let (mut monitor, tx) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 18, false)]);
    tx.send(Reply::Dismissed("mouse".to_owned())).unwrap();
    monitor.drain_replies();
    monitor.update_batteries(&[]);
    assert!(
        monitor.devices.contains_key("mouse"),
        "kept a while after it goes"
    );
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 18, false)]);
    let mouse = monitor.devices.get("mouse").unwrap();
    assert_eq!(
        mouse.dismissed,
        Some(AlertTier::Low),
        "still dismissed once back"
    );
}

#[test]
fn flapping_round_critical_does_not_alert_again() {
    let (mut monitor, _) = monitor();
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 10, false)]);
    let critical = Some(AlertTier::Critical);
    assert_eq!(monitor.devices.get("mouse").unwrap().alerted, critical);
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 11, false)]);
    assert_eq!(
        monitor.devices.get("mouse").unwrap().alerted,
        critical,
        "no step down to low"
    );
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 10, false)]);
    assert_eq!(monitor.devices.get("mouse").unwrap().alerted, critical);
}

#[test]
fn disabled_notifications_skips_alerts() {
    let (_, rx) = mpsc::channel();
    let settings = BatterySettings {
        notify: false,
        ..BatterySettings::default()
    };
    let mut monitor = Monitor::with_receiver(settings, rx);
    monitor.update_batteries(&[device("mouse", "Zowie Mouse", 5, false)]);
    assert!(monitor.devices.is_empty(), "a battery at 5% is not alerted");
}
