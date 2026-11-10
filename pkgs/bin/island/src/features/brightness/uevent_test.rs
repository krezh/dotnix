use super::{Uevent, parse};

fn message(fields: &[&str]) -> Vec<u8> {
    fields
        .iter()
        .flat_map(|field| field.bytes().chain([0]))
        .collect()
}

#[test]
fn a_backlight_change_is_heard() {
    let event = message(&[
        "change@/devices/pci0000:00/0000:00:08.1/0000:c1:00.0/backlight/amdgpu_bl1",
        "ACTION=change",
        "DEVPATH=/devices/pci0000:00/0000:00:08.1/0000:c1:00.0/backlight/amdgpu_bl1",
        "SUBSYSTEM=backlight",
        "SOURCE=sysfs",
        "SEQNUM=4242",
    ]);
    assert_eq!(parse(&event), Some(Uevent::Backlight));
}

#[test]
fn a_monitor_plugged_in_is_heard() {
    let event = message(&[
        "change@/devices/pci0000:00/0000:00:01.1/0000:03:00.0/drm/card1",
        "ACTION=change",
        "SUBSYSTEM=drm",
        "HOTPLUG=1",
        "DEVNAME=dri/card1",
    ]);
    assert_eq!(parse(&event), Some(Uevent::Hotplug));
}

#[test]
fn other_events_are_not() {
    let without_hotplug = message(&["change@/drm/card1", "ACTION=change", "SUBSYSTEM=drm"]);
    let added = message(&["add@/backlight/x", "ACTION=add", "SUBSYSTEM=backlight"]);
    let other = message(&[
        "change@/power_supply/BAT0",
        "ACTION=change",
        "SUBSYSTEM=power_supply",
    ]);
    for event in [without_hotplug, added, other] {
        assert_eq!(parse(&event), None);
    }
}
