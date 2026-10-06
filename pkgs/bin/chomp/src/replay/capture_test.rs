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
