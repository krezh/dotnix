use super::{Activity, State, Vpn, channel, jitter, kill_switch};

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

fn vpn(name: &str, activity: Activity, full: bool) -> Vpn {
    Vpn {
        name: name.to_owned(),
        activity,
        full,
        ..Vpn::default()
    }
}

#[test]
fn a_server_loses_its_port_but_not_its_address() {
    let with = |endpoint: &str| Vpn {
        endpoint: Some(endpoint.to_owned()),
        ..Vpn::default()
    };
    assert_eq!(with("62.93.166.123:51820").server(), Some("62.93.166.123"));
    assert_eq!(with("vpn.example.com").server(), Some("vpn.example.com"));
    assert_eq!(with("[2a07:b944::2:2]:51820").server(), Some("2a07:b944::2:2"));
    assert_eq!(Vpn::default().server(), None);
}

#[test]
fn running_vpns_come_first_then_by_name() {
    let state = State {
        vpns: vec![
            vpn("Work", Activity::Off, true),
            vpn("home", Activity::On, false),
            vpn("Alpha", Activity::Off, true),
            vpn("Proton", Activity::Connecting, true),
        ],
        ..State::default()
    };
    let names: Vec<&str> = state
        .vpns_in_order()
        .iter()
        .map(|vpn| vpn.name.as_str())
        .collect();
    assert_eq!(names, ["home", "Proton", "Alpha", "Work"]);
}

#[test]
fn only_a_running_full_vpn_carries_everything() {
    let mut state = State {
        vpns: vec![
            vpn("Home", Activity::On, false),
            vpn("Proton", Activity::Connecting, true),
        ],
        ..State::default()
    };
    assert!(state.tunnel().is_none());
    state.vpns[1].activity = Activity::On;
    assert_eq!(state.tunnel().map(|vpn| vpn.name.as_str()), Some("Proton"));
}

#[test]
fn proton_kill_switch_is_recognised() {
    assert!(kill_switch("pvpn-killswitch"));
    assert!(kill_switch("pvpn-killswitch-ipv6"));
    assert!(!kill_switch("Wired connection 1"));
}

#[test]
fn jitter_needs_answers_and_ignores_misses() {
    assert_eq!(jitter(&[]), None);
    assert_eq!(jitter(&[0.0, 0.0, 18.0, 0.0]), None);
    let steady = jitter(&[18.0, 18.0, 18.0, 18.0]).unwrap();
    assert!(steady.abs() < f32::EPSILON);
    // Steps of 2, 4 and 2 between the answers, the miss left out.
    let wandering = jitter(&[10.0, 12.0, 0.0, 16.0, 14.0]).unwrap();
    assert!((wandering - 8.0 / 3.0).abs() < 1e-4);
}
