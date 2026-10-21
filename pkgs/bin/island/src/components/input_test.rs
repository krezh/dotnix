use gpui::{Keystroke, Modifiers};

use super::{Edit, edit};

fn key(key: &str, modifiers: Modifiers, typed: Option<&str>) -> Keystroke {
    Keystroke {
        modifiers,
        key: key.to_owned(),
        key_char: typed.map(str::to_owned),
    }
}

fn plain(key_name: &str, typed: &str) -> Keystroke {
    key(key_name, Modifiers::default(), Some(typed))
}

#[test]
fn typing_appends_and_backspace_removes() {
    let mut text = String::new();
    assert_eq!(edit(&mut text, &plain("h", "h")), Edit::Changed);
    assert_eq!(edit(&mut text, &plain("i", "i")), Edit::Changed);
    assert_eq!(text, "hi");
    assert_eq!(edit(&mut text, &key("backspace", Modifiers::default(), None)), Edit::Changed);
    assert_eq!(text, "h");
}

#[test]
fn enter_sends_and_shift_enter_breaks_the_line() {
    let mut text = "first".to_owned();
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    assert_eq!(edit(&mut text, &key("enter", shift, None)), Edit::Changed);
    assert_eq!(text, "first\n");
    assert_eq!(edit(&mut text, &key("enter", Modifiers::default(), None)), Edit::Send);
    assert_eq!(edit(&mut text, &key("escape", Modifiers::default(), None)), Edit::Cancel);
}

#[test]
fn control_keys_edit_words_and_clear() {
    let control = Modifiers {
        control: true,
        ..Modifiers::default()
    };
    let mut text = "jj new  @-  ".to_owned();
    assert_eq!(edit(&mut text, &key("backspace", control, None)), Edit::Changed);
    assert_eq!(text, "jj new  ");
    assert_eq!(edit(&mut text, &key("u", control, Some("u"))), Edit::Changed);
    assert_eq!(text, "");
    assert_eq!(edit(&mut text, &key("c", control, Some("c"))), Edit::Ignored);
}
