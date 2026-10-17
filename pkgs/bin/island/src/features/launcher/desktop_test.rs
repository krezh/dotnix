use std::collections::HashMap;

use super::{App, search};

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
    results.into_iter().map(|index| apps[index].name.as_str()).collect()
}

#[test]
fn without_history_apps_are_by_name() {
    let apps = apps();
    assert_eq!(names(&apps, search(&apps, "", &HashMap::new())), ["Files", "Firefox", "Kitty", "Zed"]);
}

#[test]
fn with_no_query_the_most_used_come_first() {
    let apps = apps();
    let ranks = HashMap::from([("zed.desktop".to_owned(), 9.0), ("kitty.desktop".to_owned(), 2.0)]);
    assert_eq!(names(&apps, search(&apps, "", &ranks)), ["Zed", "Kitty", "Files", "Firefox"]);
}

#[test]
fn use_breaks_ties_between_equal_matches() {
    let apps = apps();
    let ranks = HashMap::from([("firefox.desktop".to_owned(), 5.0)]);
    assert_eq!(names(&apps, search(&apps, "fi", &ranks)), ["Firefox", "Files"]);
}

#[test]
fn use_never_beats_a_better_match() {
    let apps = apps();
    let ranks = HashMap::from([("files.desktop".to_owned(), 100.0)]);
    assert_eq!(names(&apps, search(&apps, "fire", &ranks)), ["Firefox"]);
    let ranks = HashMap::from([("zed.desktop".to_owned(), 100.0)]);
    assert_eq!(names(&apps, search(&apps, "kit", &ranks))[0], "Kitty");
}
