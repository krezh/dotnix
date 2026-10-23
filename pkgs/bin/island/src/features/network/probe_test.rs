use super::{checksum, echo, is_reply, query};

#[test]
fn an_echo_request_carries_its_sequence_and_a_valid_checksum() {
    let packet = echo(513);
    assert_eq!(packet[0], 8);
    assert_eq!(&packet[6..8], &[2, 1]);
    assert_eq!(checksum(&packet), 0);
}

#[test]
fn only_a_reply_to_the_same_sequence_counts() {
    let mut reply = echo(7);
    reply[0] = 0;
    assert!(is_reply(&reply, 7));
    assert!(!is_reply(&reply, 8));
    assert!(!is_reply(&echo(7), 7));
}

#[test]
fn a_query_names_each_label_and_asks_for_one_a_record() {
    let packet = query(0x1234, "example.com");
    assert_eq!(&packet[..2], &[0x12, 0x34]);
    assert_eq!(&packet[12..20], b"\x07example");
    assert_eq!(&packet[20..24], b"\x03com");
    assert_eq!(&packet[24..], &[0, 0, 1, 0, 1]);
}
