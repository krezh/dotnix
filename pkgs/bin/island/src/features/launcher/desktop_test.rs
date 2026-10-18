use std::collections::HashMap;

use super::super::settings::Settings;
use super::{App, launch, search};

fn app(id: &str, name: &str) -> App {
    App {
        id: id.to_owned(),
        name: name.to_owned(),
        generic: String::new(),
        comment: String::new(),
        keywords: Vec::new(),
        exec: name.to_lowercase(),
        terminal: false,
        directory: None,
        icon: None,
    }
}

/// Sorted by name, as `load` returns them.
fn apps() -> Vec<App> {
    vec![
        app("files.desktop", "Files"),
        app("firefox.desktop", "Firefox"),
        app("kitty.desktop", "Kitty"),
        app("zed.desktop", "Zed"),
    ]
}

fn names(apps: &[App], results: Vec<usize>) -> Vec<&str> {
    results
        .into_iter()
        .map(|index| apps[index].name.as_str())
        .collect()
}

#[test]
fn without_history_apps_are_by_name() {
    let apps = apps();
    assert_eq!(
        names(&apps, search(&apps, "", &HashMap::new())),
        ["Files", "Firefox", "Kitty", "Zed"]
    );
}

#[test]
fn with_no_query_the_most_used_come_first() {
    let apps = apps();
    let ranks = HashMap::from([
        ("zed.desktop".to_owned(), 9.0),
        ("kitty.desktop".to_owned(), 2.0),
    ]);
    assert_eq!(
        names(&apps, search(&apps, "", &ranks)),
        ["Zed", "Kitty", "Files", "Firefox"]
    );
}

#[test]
fn use_breaks_ties_between_equal_matches() {
    let apps = apps();
    let ranks = HashMap::from([("firefox.desktop".to_owned(), 5.0)]);
    assert_eq!(
        names(&apps, search(&apps, "fi", &ranks)),
        ["Firefox", "Files"]
    );
}

#[test]
fn use_never_beats_a_better_match() {
    let apps = apps();
    let ranks = HashMap::from([("files.desktop".to_owned(), 100.0)]);
    assert_eq!(names(&apps, search(&apps, "fire", &ranks)), ["Firefox"]);
    let ranks = HashMap::from([("zed.desktop".to_owned(), 100.0)]);
    assert_eq!(names(&apps, search(&apps, "kit", &ranks))[0], "Kitty");
}

#[test]
fn terminal_apps_use_the_default_terminal_and_keep_launch_metadata() {
    let mut app = app("tool.desktop", "My Tool");
    app.exec = "tool --title %c %f".to_owned();
    app.terminal = true;
    app.directory = Some("/tmp/working directory".into());

    let launch = launch(&app, &Settings::default()).unwrap();
    assert_eq!(
        launch.argv,
        ["xdg-terminal-exec", "tool", "--title", "My Tool"]
    );
    assert_eq!(launch.id, "tool");
    assert_eq!(launch.name, "My Tool");
    assert_eq!(launch.directory, app.directory);
}

#[test]
fn custom_terminal_and_desktop_arguments_keep_their_token_boundaries() {
    let settings: Settings = toml::from_str(
        r#"terminal = ["/opt/My Terminal/bin/foot", "--title", "App Window", "", "-e"]"#,
    )
    .unwrap();
    let mut app = app("tool.desktop", "My Tool");
    app.exec = r#""/opt/App Folder/tool" "two words" "--name=%c""#.to_owned();
    app.terminal = true;

    assert_eq!(
        launch(&app, &settings).unwrap().argv,
        [
            "/opt/My Terminal/bin/foot",
            "--title",
            "App Window",
            "",
            "-e",
            "/opt/App Folder/tool",
            "two words",
            "--name=My Tool"
        ],
    );
}

#[test]
fn ordinary_apps_do_not_get_a_terminal_prefix() {
    let settings: Settings = toml::from_str(r#"terminal = ["foot", "-e"]"#).unwrap();
    let mut app = app("tool.desktop", "My Tool");
    app.exec = r#"tool "two words" %c"#.to_owned();

    for settings in [Settings::default(), settings] {
        assert_eq!(
            launch(&app, &settings).unwrap().argv,
            ["tool", "two words", "My Tool"]
        );
    }
}

#[test]
fn empty_terminal_commands_are_config_errors_and_never_launch_apps_naked() {
    let mut app = app("tool.desktop", "Tool");
    app.terminal = true;
    for raw in [
        r#"terminal = []"#,
        r#"terminal = [""]"#,
        r#"terminal = ["   ", "-e"]"#,
    ] {
        let value = toml::from_str(raw).unwrap();
        let error = crate::features::check_settings("launcher", &value).unwrap_err();
        assert!(error.contains("terminal"), "{error}");
        let settings: Settings = value.try_into().unwrap();
        assert!(launch(&app, &settings).is_none());
    }
}

#[test]
fn unusable_desktop_exec_is_not_turned_into_a_terminal_only_launch() {
    let mut app = app("tool.desktop", "Tool");
    app.terminal = true;
    for exec in ["", "%f", "\"unfinished"] {
        app.exec = exec.to_owned();
        assert!(launch(&app, &Settings::default()).is_none());
    }
}
