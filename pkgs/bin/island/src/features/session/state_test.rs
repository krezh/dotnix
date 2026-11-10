use super::name_of;

#[test]
fn names_an_app_as_people_know_it() {
    assert_eq!(name_of("dev.zed.Zed"), "Zed");
    assert_eq!(name_of("com.mitchellh.ghostty"), "Ghostty");
    assert_eq!(name_of("zen"), "Zen");
    assert_eq!(name_of("vesktop"), "Vesktop");
    assert_eq!(name_of("trailing."), "Trailing.");
    assert_eq!(name_of(""), "");
}
