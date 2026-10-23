use super::{Glance, push};
use crate::{
    components::card::SLOTS,
    features::network::state::{AccessPoint, State, Wifi},
};

#[test]
fn a_history_keeps_only_as_many_samples_as_a_chart_shows() {
    let mut history = Vec::new();
    for value in 0..(SLOTS + 5) {
        push(&mut history, value as f32);
    }
    assert_eq!(history.len(), SLOTS);
    assert_eq!(history[0], 5.0);
}

#[test]
fn traffic_alone_does_not_change_what_the_bar_shows() {
    let mut state = State {
        reachable: true,
        wifi: Some(Wifi {
            enabled: true,
            networks: vec![AccessPoint {
                ssid: "Plexuz".to_owned(),
                strength: 82,
                active: true,
                ..AccessPoint::default()
            }],
            ..Wifi::default()
        }),
        ..State::default()
    };
    let before = Glance::of(&state);
    state.traffic.rx_rate = 1e6;
    assert!(Glance::of(&state) == before);
    state.wifi.as_mut().unwrap().networks[0].strength = 40;
    assert!(Glance::of(&state) != before);
}
