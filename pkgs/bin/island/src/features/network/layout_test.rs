use super::{Card, layout};

#[test]
fn every_row_fills_the_four_columns() {
    for (wifi, vpn) in [(true, true), (true, false), (false, true), (false, false)] {
        for row in layout(wifi, vpn) {
            assert_eq!(row.iter().map(|(_, span)| span).sum::<usize>(), 4);
        }
    }
}

#[test]
fn each_card_shows_once_and_only_when_it_has_something() {
    let cards = |wifi, vpn| -> Vec<Card> {
        layout(wifi, vpn)
            .into_iter()
            .flatten()
            .map(|(card, _)| card)
            .collect()
    };
    for (wifi, vpn) in [(true, true), (true, false), (false, true), (false, false)] {
        let shown = cards(wifi, vpn);
        assert_eq!(shown.contains(&Card::Wifi), wifi);
        assert_eq!(
            shown.contains(&Card::Vpn) || shown.contains(&Card::VpnShort),
            vpn
        );
        for always in [Card::Internet, Card::Interfaces, Card::Traffic, Card::Activity] {
            assert_eq!(shown.iter().filter(|card| **card == always).count(), 1);
        }
    }
}

#[test]
fn the_vpn_card_is_short_beside_wifi() {
    assert!(layout(true, true)[1].contains(&(Card::VpnShort, 1)));
    assert!(layout(false, true)[0].contains(&(Card::Vpn, 1)));
}
