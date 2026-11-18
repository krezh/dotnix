use super::{Entry, last_part, parse};

#[test]
fn reads_name_icon_and_class_from_the_desktop_entry_only() {
    let raw = "[Desktop Entry]\nName=Ghostty\nIcon=com.mitchellh.ghostty\nStartupWMClass=com.mitchellh.ghostty\n\n[Desktop Action new-window]\nName=New Window\nIcon=other\n";
    assert_eq!(
        parse(raw),
        Some(Entry {
            name: "Ghostty".to_owned(),
            icon: "com.mitchellh.ghostty".to_owned(),
            class: Some("com.mitchellh.ghostty".to_owned()),
        })
    );
}

#[test]
fn an_entry_without_a_name_is_not_an_app() {
    assert_eq!(parse("[Desktop Entry]\nIcon=thing\n"), None);
}

#[test]
fn takes_the_last_part_of_a_reverse_domain_name() {
    assert_eq!(last_part("dev.zed.Zed"), "Zed");
    assert_eq!(last_part("zen"), "zen");
}
