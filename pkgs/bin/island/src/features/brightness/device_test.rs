use super::{connector_for, connector_name, model_name};

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
    let connectors = vec![
        ("DP-1".to_owned(), first),
        ("DP-2".to_owned(), second.clone()),
    ];
    second.truncate(128);
    assert_eq!(connector_for(&connectors, &second).as_deref(), Some("DP-2"));
    assert_eq!(connector_for(&connectors, &[3; 128]), None);
    assert_eq!(connector_for(&connectors, &[2; 16]), None);
}

#[test]
fn the_model_name_comes_from_its_edid_descriptor() {
    let mut edid = vec![0xAA; 256];
    edid[72..90].copy_from_slice(&[
        0, 0, 0, 0xFF, 0, b'H', b'S', b'3', b'P', b'7', b'0', b'1', b'1', b'0', b'5', b'\n', b' ',
        b' ',
    ]);
    edid[90..108].copy_from_slice(&[
        0, 0, 0, 0xFC, 0, b'S', b'y', b'n', b'c', b'M', b'a', b's', b't', b'e', b'r', b'\n', b' ',
        b' ',
    ]);
    assert_eq!(model_name(&edid).as_deref(), Some("SyncMaster"));
    edid[93] = 0xFE;
    assert_eq!(model_name(&edid), None);
    assert_eq!(model_name(&[0; 64]), None);
}
