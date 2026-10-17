use super::Shortcut;

#[test]
fn reads_shortcuts() {
    let paste = Shortcut::parse("ctrl+shift+v").unwrap().unwrap();
    assert_eq!(paste.modifiers, vec![(29, 4), (42, 1)]);
    assert_eq!(paste.key, 47);
    assert_eq!(Shortcut::parse("Shift + Insert").unwrap().unwrap().key, 110);
    assert_eq!(Shortcut::parse(" ").unwrap(), None);
}

#[test]
fn rejects_unknown_keys() {
    assert!(Shortcut::parse("hyper+v").is_err());
    assert!(Shortcut::parse("ctrl+f5").is_err());
}
