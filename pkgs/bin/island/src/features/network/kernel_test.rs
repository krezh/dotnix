use super::{LISTEN, Socket, app_name, parse_routes, parse_sockets, parse_wireless};

#[test]
fn wrapped_apps_go_by_their_own_names() {
    assert_eq!(app_name(".zen-twilight-w"), "zen-twilight");
    assert_eq!(app_name(".claude-wrapped"), "claude");
    assert_eq!(app_name("steamwebhelper"), "steamwebhelper");
    assert_eq!(app_name("node-wrapped"), "node-wrapped");
}

#[test]
fn the_wireless_line_gives_quality_level_and_retries() {
    let text = "Inter-| sta-|   Quality        |   Discarded packets               | Missed | WE
 face | tus | link level noise |  nwid  crypt   frag  retry   misc | beacon | 22
 wlan0: 0000   56.  -54.  -256        0      0      0     12      0        0
";
    let wireless = parse_wireless(text, "wlan0").unwrap();
    assert_eq!(wireless.dbm, -54);
    assert_eq!(wireless.noise, None);
    assert_eq!(wireless.retries, 12);
    assert!((wireless.quality - 0.8).abs() < 0.001);
    assert!(parse_wireless(text, "wlan1").is_none());
}

#[test]
fn routes_read_with_the_default_first_and_gateways_only_where_set() {
    let text = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
enp10s0\t0001A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0
enp10s0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0
docker0\t000011AC\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
";
    let routes = parse_routes(text);
    assert_eq!(routes[0].destination, "default");
    assert_eq!(routes[0].via.as_deref(), Some("192.168.1.1"));
    assert_eq!(routes[1].destination, "172.17.0.0/16");
    assert_eq!(routes[2].destination, "192.168.1.0/24");
    assert_eq!(routes[2].via, None);
}

#[test]
fn sockets_read_their_port_state_and_inode() {
    let text = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 41234 1 0000000000000000 100 0 0 10 0
   1: 0100007F:A2B4 0100007F:1F90 01 00000000:00000000 00:00000000 00000000  1000        0 41235 1 0000000000000000 20 4 30 10 -1
";
    assert_eq!(
        parse_sockets(text),
        vec![
            Socket {
                port: 8080,
                state: LISTEN,
                inode: 41234
            },
            Socket {
                port: 41652,
                state: 1,
                inode: 41235
            },
        ]
    );
}
