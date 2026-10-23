use super::{connector_for, connector_name};

#[test]
fn a_connector_is_named_without_its_card() {
    assert_eq!(connector_name("card1-DP-1").as_deref(), Some("DP-1"));
    assert_eq!(connector_name("card0-eDP-1").as_deref(), Some("eDP-1"));
    assert_eq!(connector_name("card1"), None);
    assert_eq!(connector_name("renderD128"), None);
}

#[test]
fn a_monitor_is_found_by_its_edid_base_block() {
    let first = vec![1; 128];
    let mut second = vec![2; 384];
    let connectors = vec![("DP-1".to_owned(), first), ("DP-2".to_owned(), second.clone())];
    second.truncate(128);
    assert_eq!(connector_for(&connectors, &second).as_deref(), Some("DP-2"));
    assert_eq!(connector_for(&connectors, &[3; 128]), None);
    assert_eq!(connector_for(&connectors, &[2; 16]), None);
}
