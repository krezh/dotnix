use super::{columns, level, title};
use crate::features::which_key::settings::Settings;

fn settings() -> Settings {
    let text = r#"
        [[menus.default]]
        key = "b"
        desc = "Browser"

        [[menus.default.submenu]]
        key = "h"
        desc = "Helium"
        cmd = "helium"

        [[menus.default]]
        key = "e"
        desc = "Files"
        cmd = "nautilus"
    "#;
    toml::from_str(text).unwrap()
}

#[test]
fn the_top_of_a_menu_is_titled_by_its_name() {
    let settings = settings();
    let top = level(&settings, "default", &[]).unwrap();
    assert_eq!(top.path, ["Default"]);
    assert_eq!(top.entries.len(), 2);
}

#[test]
fn keys_lead_into_submenus() {
    let settings = settings();
    let inner = level(&settings, "default", &["b".to_owned()]).unwrap();
    assert_eq!(inner.path, ["Default", "Browser"]);
    assert_eq!(inner.entries[0].desc, "Helium");
}

#[test]
fn a_key_running_a_command_leads_nowhere() {
    let settings = settings();
    assert!(level(&settings, "default", &["e".to_owned()]).is_none());
}

#[test]
fn an_unknown_menu_has_no_level() {
    assert!(level(&settings(), "browser", &[]).is_none());
}

#[test]
fn titles_start_with_a_capital() {
    assert_eq!(title("browser"), "Browser");
    assert_eq!(title(""), "");
}

#[test]
fn entries_sit_side_by_side_up_to_three() {
    assert_eq!(columns(0), 1);
    assert_eq!(columns(1), 1);
    assert_eq!(columns(2), 2);
    assert_eq!(columns(3), 3);
    assert_eq!(columns(12), 3);
}
