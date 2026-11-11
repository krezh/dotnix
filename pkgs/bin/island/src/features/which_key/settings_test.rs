use super::check;

fn checked(text: &str) -> Result<(), String> {
    check(&toml::from_str::<toml::Value>(text).unwrap())
}

#[test]
fn a_menu_of_commands_and_submenus_is_accepted() {
    let text = r#"
        [[menus.browser]]
        key = "h"
        desc = "Helium"
        cmd = "helium"

        [[menus.browser]]
        key = "w"
        desc = "Web apps"

        [[menus.browser.submenu]]
        key = "M"
        desc = "Mail"
        cmd = "proton-mail"
    "#;
    assert_eq!(checked(text), Ok(()));
}

#[test]
fn no_menus_is_fine() {
    assert_eq!(checked(""), Ok(()));
}

#[test]
fn a_key_is_one_character() {
    let text = r#"
        [[menus.browser]]
        key = "F1"
        desc = "Helium"
        cmd = "helium"
    "#;
    assert_eq!(
        checked(text),
        Err("menu browser: key \"F1\" must be one character".to_owned())
    );
}

#[test]
fn a_key_is_not_a_space() {
    let text = r#"
        [[menus.browser]]
        key = " "
        desc = "Helium"
        cmd = "helium"
    "#;
    assert!(checked(text).is_err());
}

#[test]
fn keys_are_unique_within_a_menu() {
    let text = r#"
        [[menus.browser]]
        key = "h"
        desc = "Helium"
        cmd = "helium"

        [[menus.browser]]
        key = "h"
        desc = "Other"
        cmd = "other"
    "#;
    assert_eq!(
        checked(text),
        Err("menu browser: key \"h\" is used twice".to_owned())
    );
}

#[test]
fn keys_differing_in_case_are_different_keys() {
    let text = r#"
        [[menus.power]]
        key = "s"
        desc = "Suspend"
        cmd = "systemctl suspend"

        [[menus.power]]
        key = "S"
        desc = "Shut down"
        cmd = "systemctl poweroff"
    "#;
    assert_eq!(checked(text), Ok(()));
}

#[test]
fn an_entry_needs_a_command_or_a_submenu() {
    let text = r#"
        [[menus.browser]]
        key = "h"
        desc = "Helium"
    "#;
    assert_eq!(
        checked(text),
        Err("menu browser: key \"h\" needs a cmd or a submenu".to_owned())
    );
}

#[test]
fn an_entry_has_not_both() {
    let text = r#"
        [[menus.browser]]
        key = "h"
        desc = "Helium"
        cmd = "helium"

        [[menus.browser.submenu]]
        key = "x"
        desc = "X"
        cmd = "x"
    "#;
    assert_eq!(
        checked(text),
        Err("menu browser: key \"h\" has both a cmd and a submenu".to_owned())
    );
}

#[test]
fn a_submenu_is_checked_too() {
    let text = r#"
        [[menus.browser]]
        key = "w"
        desc = "Web apps"

        [[menus.browser.submenu]]
        key = "mm"
        desc = "Mail"
        cmd = "proton-mail"
    "#;
    assert_eq!(
        checked(text),
        Err("menu browser › Web apps: key \"mm\" must be one character".to_owned())
    );
}

#[test]
fn unknown_fields_are_refused() {
    let text = r#"
        [[menus.browser]]
        key = "h"
        desc = "Helium"
        cmd = "helium"
        color = "blue"
    "#;
    assert!(checked(text).is_err());
}
