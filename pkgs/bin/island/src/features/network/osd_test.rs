use super::osd;
use crate::features::network::state::{Internet, Link, State, Wired};

fn wired(link: bool, internet: Internet) -> State {
    State {
        reachable: true,
        internet,
        wired: Some(Wired {
            active: link,
            name: "Wired connection 1".to_owned(),
            ..Wired::default()
        }),
        link: link.then(|| Link {
            name: "Wired connection 1".to_owned(),
            interface: "enp10s0".to_owned(),
            ..Link::default()
        }),
        ..State::default()
    }
}

#[test]
fn losing_the_link_pops_up_and_so_does_getting_it_back() {
    let up = wired(true, Internet::Full);
    let down = wired(false, Internet::None);
    assert_eq!(osd(&up, &down).unwrap().label, "Connection lost");
    assert_eq!(
        osd(&down, &up).unwrap().label,
        "Connected · Wired connection 1"
    );
}

#[test]
fn the_internet_going_behind_a_link_that_stays_up_pops_up() {
    let full = wired(true, Internet::Full);
    let none = wired(true, Internet::None);
    assert_eq!(osd(&full, &none).unwrap().label, "No internet");
    assert_eq!(osd(&none, &full).unwrap().label, "Internet is back");
    assert!(osd(&full, &full).is_none());
}

#[test]
fn nothing_pops_up_while_island_is_starting() {
    let starting = State::default();
    assert!(osd(&starting, &wired(true, Internet::Full)).is_none());
}

#[test]
fn an_unchecked_internet_never_pops_up() {
    let unknown = wired(true, Internet::Unknown);
    let full = wired(true, Internet::Full);
    assert!(osd(&unknown, &full).is_none());
}

#[test]
fn bad_news_stays_long_enough_to_see_in_a_game() {
    let up = wired(true, Internet::Full);
    let down = wired(false, Internet::None);
    let lost = osd(&up, &down).unwrap().lasts;
    let back = osd(&down, &up).unwrap().lasts;
    assert!(lost > back);
}
