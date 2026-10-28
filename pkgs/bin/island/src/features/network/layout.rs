//! Where each card of the Network tab goes, from what the machine has: Wi-Fi gets a column of its
//! own when there is any, and the VPN card only shows when there are VPNs to show.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Card {
    Internet,
    Wifi,
    Vpn,
    /// Its rows only, in the short second row.
    VpnShort,
    Interfaces,
    Traffic,
    Activity,
}

/// The two rows of cards, each with how many columns it spans.
pub fn layout(wifi: bool, vpn: bool) -> [Vec<(Card, usize)>; 2] {
    match (wifi, vpn) {
        (true, true) => [
            vec![(Card::Internet, 2), (Card::Wifi, 1), (Card::Interfaces, 1)],
            vec![(Card::Traffic, 2), (Card::VpnShort, 1), (Card::Activity, 1)],
        ],
        (true, false) => [
            vec![(Card::Internet, 2), (Card::Wifi, 1), (Card::Interfaces, 1)],
            vec![(Card::Traffic, 2), (Card::Activity, 2)],
        ],
        (false, true) => [
            vec![(Card::Internet, 2), (Card::Vpn, 1), (Card::Interfaces, 1)],
            vec![(Card::Traffic, 2), (Card::Activity, 2)],
        ],
        (false, false) => [
            vec![(Card::Internet, 2), (Card::Activity, 1), (Card::Interfaces, 1)],
            vec![(Card::Traffic, 4)],
        ],
    }
}

#[cfg(test)]
#[path = "layout_test.rs"]
mod tests;
