use super::*;

#[test]
fn layouts_name_feature_widgets() {
    let bar: Bar = toml::from_str("left = [\"notifications\", \"clock\", \"zowie\"]").unwrap();
    assert_eq!(
        bar.left,
        vec![Widget("notifications"), Widget("clock"), Widget("zowie")]
    );
}

#[test]
fn invalid_feature_settings_are_rejected() {
    for raw in [
        "[features.notifications.privacy]\nprivat = true",
        "[features.notificatons.privacy]\nprivate = true",
        "[features.caffeine]\non = true",
        "[features.system.storage]\npartition = ['/']",
        "[features.notifications]\nprivate = true",
        "[features.system]\npartitions = ['/']",
        "[notifications]\nprivate = true",
        "[system]\npartitions = ['/']",
        "[visualizer]\nglow = 0.8",
    ] {
        let result = toml::from_str::<Config>(raw)
            .map_err(|error| error.to_string())
            .and_then(Config::check);
        assert!(result.is_err(), "{raw}");
    }
}

#[test]
fn partitions_are_hidden_or_picked() {
    let mounts = ["/", "/boot", "/home", "/mnt/data"];
    let shown = |system: System| {
        system
            .storage
            .shown_partitions(&mounts, |mount| *mount)
            .into_iter()
            .copied()
            .collect::<Vec<_>>()
    };
    assert_eq!(shown(System::default()), ["/", "/home", "/mnt/data"]);
    let picked: Config =
        toml::from_str("[features.system.storage]\npartitions = [\"/home\", \"/\", \"/gone\"]")
            .unwrap();
    assert_eq!(shown(picked.features.system), ["/home", "/"]);
    let everything: Config =
        toml::from_str("[features.system.storage]\nhidden_partitions = []").unwrap();
    assert_eq!(shown(everything.features.system), mounts);
}

#[test]
fn unknown_widgets_list_the_known_ones() {
    let error = toml::from_str::<Bar>("left = [\"headsett\"]")
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown widget 'headsett'"), "{error}");
    assert!(error.contains("headset"), "{error}");
    assert!(error.contains("zowie"), "{error}");
}
