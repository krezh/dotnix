use super::{link, signal};
use crate::features::network::state::{AccessPoint, Joining, State, Wifi, Wired};

fn laptop(strength: u8) -> State {
    State {
        wifi: Some(Wifi {
            enabled: true,
            networks: vec![AccessPoint {
                ssid: "Plexuz".to_owned(),
                strength,
                active: true,
                ..AccessPoint::default()
            }],
            ..Wifi::default()
        }),
        ..State::default()
    }
}

#[test]
fn the_signal_glyph_follows_its_strength() {
    assert_eq!(signal(95), "network-wifi-excellent");
    assert_eq!(signal(45), "network-wifi-ok");
    assert_eq!(signal(5), "network-wifi-none");
}

#[test]
fn the_bar_shows_wifi_first_then_the_cable() {
    assert_eq!(link(&laptop(70)), "network-wifi-good");
    let mut joining = laptop(70);
    joining.joining = Some(Joining {
        ssid: "Telia".to_owned(),
        failed: None,
    });
    assert_eq!(link(&joining), "network-wifi-acquiring");
    let desktop = State {
        wired: Some(Wired {
            active: true,
            ..Wired::default()
        }),
        ..State::default()
    };
    assert_eq!(link(&desktop), "network-wired");
}
