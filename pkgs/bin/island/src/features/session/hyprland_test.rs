use std::collections::HashMap;

use super::{Client, Layers, check_address, parse_lock};

#[test]
fn reads_hyprlands_process_from_its_lock() {
    assert_eq!(parse_lock("1234\nwayland-1\n").unwrap(), 1234);
    for lock in ["", "1\nwayland-1\n", "1234\n", "1234\nwayland-1\nextra"] {
        assert!(parse_lock(lock).is_err(), "{lock:?}");
    }
}

#[test]
fn takes_only_window_addresses_safe_in_a_dispatcher() {
    for address in ["", "0x0", "0x+1", "0x1' })", "address:0x1", "0xno"] {
        assert!(check_address(address).is_err(), "{address:?}");
    }
    assert!(check_address("0x123abc").is_ok());
}

#[test]
fn refuses_windows_and_layers_it_cannot_read() {
    assert!(serde_json::from_str::<Vec<Client>>("{}").is_err());
    assert!(
        serde_json::from_str::<Vec<Client>>(
            r#"[{"address":"0x1","class":"x","title":"x","pid":"12"}]"#
        )
        .is_err()
    );
    assert!(serde_json::from_str::<HashMap<String, Layers>>(r#"{"DP-1":{}}"#).is_err());
}
