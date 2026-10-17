use super::*;

#[test]
fn layouts_name_feature_widgets() {
    let bar: Bar = toml::from_str("left = [\"notifications\", \"clock\", \"zowie\"]").unwrap();
    assert_eq!(bar.left, vec![Widget("notifications"), Widget("clock"), Widget("zowie")]);
}

#[test]
fn feature_sections_are_checked() {
    let check = |raw: &str| toml::from_str::<Config>(raw).map_err(|error| error.to_string()).and_then(Config::check);
    assert!(check("[notifications]\nprivate = true").is_ok());
    let typo = check("[notifications]\nprivat = true").unwrap_err();
    assert!(typo.contains("[notifications]"), "{typo}");
    let unknown = check("[notificatons]\nprivate = true").unwrap_err();
    assert!(unknown.contains("unknown section [notificatons]"), "{unknown}");
    let none = check("[caffeine]\non = true").unwrap_err();
    assert!(none.contains("takes no settings"), "{none}");
}

#[test]
fn partitions_are_hidden_or_picked() {
    let mounts = ["/", "/boot", "/home", "/mnt/data"];
    let shown =
        |system: System| system.shown_partitions(&mounts, |mount| *mount).into_iter().copied().collect::<Vec<_>>();
    assert_eq!(shown(System::default()), ["/", "/home", "/mnt/data"]);
    let picked: System = toml::from_str("partitions = [\"/home\", \"/\", \"/gone\"]").unwrap();
    assert_eq!(shown(picked), ["/home", "/"]);
    let everything: System = toml::from_str("hidden_partitions = []").unwrap();
    assert_eq!(shown(everything), mounts);
}

#[test]
fn unknown_widgets_list_the_known_ones() {
    let error = toml::from_str::<Bar>("left = [\"headsett\"]").unwrap_err().to_string();
    assert!(error.contains("unknown widget 'headsett'"), "{error}");
    assert!(error.contains("headset"), "{error}");
    assert!(error.contains("zowie"), "{error}");
}
