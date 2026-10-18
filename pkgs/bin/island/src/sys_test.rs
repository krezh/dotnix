use super::{marketing, pci_name};

const DATABASE: &str = "\
# header comment
1002  Advanced Micro Devices, Inc. [AMD/ATI]
\t1478  Navi 10 XL Upstream Port of PCI Express Switch
# Used in the Sony PlayStation 5 Phat
\t13c0  Granite Ridge [Radeon Graphics]
\t7550  Navi 48 [Radeon RX 9070/9070 XT/9070 GRE]
\t\t1458 2437  Navi 48 XTX [Radeon RX 9070 XT Gaming OC ICE 16G]
\t\t148c 2435  Radeon RX 9070 XT 16GB
\t7551  Navi 48 [Radeon AI PRO R9700]
1022  Advanced Micro Devices, Inc. [AMD]
\t7550  Not a GPU
";

#[test]
fn finds_boards_past_comment_lines() {
    assert_eq!(
        pci_name(DATABASE, "1002", "7550", "148c 2435").as_deref(),
        Some("Radeon RX 9070 XT")
    );
    assert_eq!(
        pci_name(DATABASE, "1002", "7550", "1da2 e490").as_deref(),
        Some("Radeon RX 9070")
    );
    assert_eq!(
        pci_name(DATABASE, "1002", "13c0", "1462 7e51").as_deref(),
        Some("Radeon Graphics")
    );
    assert_eq!(pci_name(DATABASE, "1002", "ffff", "0000 0000"), None);
}

#[test]
fn program_names_drop_nix_wrapper_decoration() {
    use super::program_name;
    assert_eq!(program_name(".zen-twilight-wrapped"), "zen-twilight");
    assert_eq!(program_name(".Hyprland-wrapp"), "Hyprland");
    assert_eq!(program_name(".island-wrapped"), "island");
    assert_eq!(program_name("pipewire"), "pipewire");
    assert_eq!(program_name("gtk-wrapper"), "gtk-wrapper");
}

#[test]
fn marketing_names_come_from_pci_ids_entries() {
    assert_eq!(marketing("Radeon RX 9070 XT 16GB"), "Radeon RX 9070 XT");
    assert_eq!(
        marketing("Navi 48 [Radeon RX 9070/9070 XT/9070 GRE]"),
        "Radeon RX 9070"
    );
    assert_eq!(
        marketing("Navi 48 XTX [Sapphire Pulse Radeon RX 9070 XT]"),
        "Sapphire Pulse Radeon RX 9070 XT"
    );
    assert_eq!(marketing("GeForce RTX 4090"), "GeForce RTX 4090");
}
