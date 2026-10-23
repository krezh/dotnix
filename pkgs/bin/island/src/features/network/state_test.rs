use super::channel;

#[test]
fn frequencies_map_to_their_channels() {
    assert_eq!(channel(2412), 1);
    assert_eq!(channel(2437), 6);
    assert_eq!(channel(2484), 14);
    assert_eq!(channel(5180), 36);
    assert_eq!(channel(5500), 100);
    assert_eq!(channel(5975), 5);
    assert_eq!(channel(900), 0);
}
