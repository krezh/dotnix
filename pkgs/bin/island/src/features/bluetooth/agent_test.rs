use super::{Asked, passkey};

#[test]
fn a_passkey_reads_as_two_groups_of_three() {
    assert_eq!(passkey(48_219), "048 219");
    assert_eq!(passkey(999_999), "999 999");
}

#[test]
fn only_the_question_still_asked_takes_an_answer() {
    let mut asked = Asked::default();
    let path = zbus::OwnedObjectPath::try_from("/org/bluez/hci0/dev_00").unwrap();
    let first = asked.ask(&path, "123 456".to_owned(), true);
    let second = asked.ask(&path, "654 321".to_owned(), true);
    asked.answer(first, true);
    assert_eq!(asked.current.as_ref().map(|ask| ask.id), Some(second));
    asked.answer(second, false);
    assert!(asked.current.is_none());
}
