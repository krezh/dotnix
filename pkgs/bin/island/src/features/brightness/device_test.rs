use super::{Connector, connector_for, connector_name, model_name};

fn connector(name: &str, edid: Vec<u8>, adapter: Option<&str>) -> Connector {
    Connector {
        name: name.to_owned(),
        edid,
        adapter: adapter.map(str::to_owned),
    }
}

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
        connector("DP-1", first, None),
        connector("DP-2", second.clone(), None),
    ];
    second.truncate(128);
    let find = |edid: &[u8]| connector_for(&connectors, "i2c-9", edid, &[]);
    assert_eq!(find(&second).as_deref(), Some("DP-2"));
    assert_eq!(find(&[3; 128]), None);
    assert_eq!(find(&[2; 16]), None);
}

#[test]
fn identical_monitors_get_their_own_connectors() {
    let same = vec![1; 128];
    let unlinked = vec![
        connector("DP-1", same.clone(), None),
        connector("DP-2", same.clone(), None),
    ];
    let first = connector_for(&unlinked, "i2c-5", &same, &[]).unwrap();
    let second = connector_for(&unlinked, "i2c-6", &same, &[first.clone()]);
    assert_eq!(first, "DP-1");
    assert_eq!(second.as_deref(), Some("DP-2"));

    let linked = vec![
        connector("DP-1", same.clone(), Some("i2c-5")),
        connector("DP-2", same.clone(), Some("i2c-6")),
    ];
    assert_eq!(
        connector_for(&linked, "i2c-6", &same, &[]).as_deref(),
        Some("DP-2")
    );
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
