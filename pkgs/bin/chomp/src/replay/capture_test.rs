use super::*;

#[test]
fn derives_bitrate_from_capture_size() {
    assert_eq!(
        resolve_bitrate_bytes("", 1920, 1080, 60).unwrap(),
        5_000_000
    );
    assert_eq!(
        resolve_bitrate_bytes("", 3840, 2160, 60).unwrap(),
        22_000_000
    );
}

#[test]
fn parses_configured_byte_bitrate() {
    assert_eq!(
        resolve_bitrate_bytes("12 MB", 1920, 1080, 60).unwrap(),
        12_000_000
    );
}

fn packet(size: usize, key: bool) -> ffmpeg::Packet {
    let mut packet = ffmpeg::Packet::new(size);
    if key {
        packet.set_flags(ffmpeg::codec::packet::Flags::KEY);
    }
    packet
}

#[test]
fn duration_eviction_preserves_a_decodable_keyframe() {
    let mut ring = PacketRing::new(Duration::from_secs(2));
    ring.push(packet(10, true), 0);
    ring.push(packet(10, false), 1_000_000);
    ring.push(packet(10, true), 2_000_000);
    ring.push(packet(10, false), 4_000_000);

    assert_eq!(ring.packets.front().unwrap().timestamp_micros, 2_000_000);
    assert!(ring.packets.front().unwrap().packet.is_key());
    assert_eq!(ring.buffered_duration(), Duration::from_secs(2));
}

#[test]
fn byte_eviction_advances_to_the_next_keyframe() {
    let mut ring = PacketRing::new(Duration::from_secs(30));
    ring.set_max_bytes(25);
    ring.push(packet(10, true), 0);
    ring.push(packet(10, false), 1_000_000);
    ring.push(packet(10, true), 2_000_000);

    assert_eq!(ring.packets.len(), 1);
    assert_eq!(ring.bytes, 10);
    assert!(ring.packets.front().unwrap().packet.is_key());
}

#[test]
fn oversized_gop_discards_interframes_until_the_next_keyframe() {
    let mut ring = PacketRing::new(Duration::from_secs(30));
    ring.set_max_bytes(15);
    ring.push(packet(10, true), 0);
    ring.push(packet(10, false), 1_000_000);
    ring.push(packet(10, false), 2_000_000);

    assert!(ring.packets.is_empty());
    assert_eq!(ring.bytes, 0);

    ring.push(packet(10, true), 3_000_000);
    assert_eq!(ring.packets.len(), 1);
    assert!(ring.packets.front().unwrap().packet.is_key());
}

#[test]
fn packets_for_duration_slices_from_nearest_keyframe() {
    let mut ring = PacketRing::new(Duration::from_secs(30));
    // Keyframe at 0s, P-frames at 1s, 2s
    ring.push(packet(10, true), 0);
    ring.push(packet(10, false), 1_000_000);
    ring.push(packet(10, false), 2_000_000);
    // Keyframe at 3s, P-frame at 4s
    ring.push(packet(10, true), 3_000_000);
    ring.push(packet(10, false), 4_000_000);

    // Request last 2 seconds (from 4s cutoff is 2s, nearest keyframe at or before 2s is at 0s)
    let (packets, duration) = ring.packets_for_duration(Some(Duration::from_secs(2)));
    assert_eq!(packets.len(), 5);
    assert_eq!(duration, Duration::from_secs(4));

    // Request last 1 second (from 4s cutoff is 3s, keyframe is at 3s)
    let (packets, duration) = ring.packets_for_duration(Some(Duration::from_secs(1)));
    assert_eq!(packets.len(), 2);
    assert!(packets[0].is_key());
    assert_eq!(duration, Duration::from_secs(1));

    // Request None returns all packets
    let (packets, duration) = ring.packets_for_duration(None);
    assert_eq!(packets.len(), 5);
    assert_eq!(duration, Duration::from_secs(4));
}

#[test]
fn set_duration_evicts_older_packets() {
    let mut ring = PacketRing::new(Duration::from_secs(30));
    ring.push(packet(10, true), 0);
    ring.push(packet(10, false), 10_000_000);
    ring.push(packet(10, true), 20_000_000);
    ring.push(packet(10, false), 25_000_000);

    assert_eq!(ring.packets.len(), 4);
    assert_eq!(ring.buffered_duration(), Duration::from_secs(25));

    // Lower duration to 10s: cutoff from 25s is 15s. Retain keyframe at or before 15s is at 0s.
    // But if we lower duration to 5s: cutoff from 25s is 20s. Keyframe at 20s is retained!
    ring.set_duration(Duration::from_secs(5));
    assert_eq!(ring.packets.len(), 2);
    assert_eq!(ring.packets.front().unwrap().timestamp_micros, 20_000_000);
    assert!(ring.packets.front().unwrap().packet.is_key());
}

fn tagged_window(address: u64, title: &str, focus_history_id: i64) -> TaggedWindow {
    TaggedWindow {
        address,
        capture_handle: address as u32,
        geometry: [0, 0, 1920, 1080],
        class: "steam_app_default".to_string(),
        title: title.to_string(),
        focus_history_id,
    }
}

#[test]
fn hands_capture_from_launcher_to_game_and_keeps_it_when_focus_leaves() {
    let mut windows = vec![tagged_window(1, "Battle.net", 0)];
    let launcher = select_target(&windows, None).unwrap().address;
    assert_eq!(launcher, 1);

    windows[0].focus_history_id = 1;
    windows.push(tagged_window(2, "World of Warcraft", 0));
    let game = select_target(&windows, Some(launcher)).unwrap().address;
    assert_eq!(game, 2);

    windows[0].focus_history_id = 4;
    windows[1].focus_history_id = 2;
    assert_eq!(select_target(&windows, Some(game)).unwrap().address, 2);

    windows.remove(1);
    assert_eq!(select_target(&windows, Some(game)).unwrap().address, 1);
    assert!(select_target(&[], Some(game)).is_none());
}

#[test]
fn ignores_never_focused_windows_until_no_focused_target_remains() {
    let mut windows = vec![
        tagged_window(1, "Battle.net", 1),
        tagged_window(2, "Game splash", -1),
    ];
    assert_eq!(select_target(&windows, Some(2)).unwrap().address, 1);

    windows[0].focus_history_id = -1;
    assert_eq!(select_target(&windows, Some(2)).unwrap().address, 2);
    assert_eq!(select_target(&windows, None).unwrap().address, 1);
}
