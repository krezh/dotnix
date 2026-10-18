use std::sync::Arc;

use super::{App, Apps, Local};
use crate::{components::list::ROWS, features::Local as _};

fn apps(ids: &[&str]) -> Apps {
    Apps(Arc::new(
        ids.iter()
            .map(|id| App {
                id: (*id).to_owned(),
                name: (*id).to_owned(),
                generic: String::new(),
                comment: String::new(),
                keywords: Vec::new(),
                exec: (*id).to_owned(),
                terminal: false,
                directory: None,
                icon: None,
            })
            .collect(),
    ))
}

fn selected(local: &Local) -> Option<&str> {
    local.selected().map(|app| app.id.as_str())
}

#[test]
fn keyboard_selection_survives_insertions_and_reordering_by_desktop_id() {
    let mut local = Local::default();
    local.sync(&apps(&["Alpha", "Charlie", "Echo"]));
    local.step(1);
    assert_eq!(selected(&local), Some("Charlie"));

    local.sync(&apps(&["Alpha", "Bravo", "Charlie", "Echo"]));
    assert_eq!(selected(&local), Some("Charlie"));
    assert_eq!(local.selection.selected, 2);

    let mut reordered = apps(&["Echo", "Charlie", "Bravo", "Alpha"]);
    Arc::get_mut(&mut reordered.0).unwrap()[1].name = "Renamed application".to_owned();
    local.sync(&reordered);
    assert_eq!(selected(&local), Some("Charlie"));
    assert_eq!(local.selected().unwrap().name, "Renamed application");
    assert_eq!(local.selection.selected, 1);
}

#[test]
fn hover_and_keyboard_changes_replace_the_preserved_identity() {
    let mut local = Local::default();
    local.sync(&apps(&["Alpha", "Charlie", "Echo"]));
    local.select(2);
    local.sync(&apps(&["Alpha", "Bravo", "Charlie", "Echo"]));
    assert_eq!(selected(&local), Some("Echo"));

    local.step(-1);
    local.sync(&apps(&["Alpha", "Charlie", "Delta", "Echo"]));
    assert_eq!(selected(&local), Some("Charlie"));
    assert_eq!(local.selection.selected, 1);
}

#[test]
fn removal_selects_the_next_neighbor_then_the_previous_at_the_end() {
    let mut local = Local::default();
    local.sync(&apps(&["Alpha", "Bravo", "Charlie", "Delta"]));
    local.select(1);
    local.sync(&apps(&["Alpha", "Charlie", "Delta"]));
    assert_eq!(selected(&local), Some("Charlie"));
    assert_eq!(local.selection.selected, 1);

    local.select(2);
    local.sync(&apps(&["Alpha", "Charlie"]));
    assert_eq!(selected(&local), Some("Charlie"));
    assert_eq!(local.selection.selected, 1);

    local.sync(&apps(&[]));
    assert_eq!(selected(&local), None);
    assert_eq!(local.selection.selected, 0);
    assert_eq!(local.selection.top, 0);

    local.sync(&apps(&["Bravo"]));
    assert_eq!(selected(&local), Some("Bravo"));
}

#[test]
fn snapshots_preserve_the_query_and_selection_in_the_filtered_results() {
    let mut local = Local::default();
    local.sync(&apps(&["Editor A", "Editor C", "Other"]));
    local.search(|query| query.push_str("ed"));
    local.step(1);
    assert_eq!(selected(&local), Some("Editor C"));

    local.sync(&apps(&["Editor A", "Editor B", "Editor C", "Other"]));
    assert_eq!(local.selection.query, "ed");
    assert_eq!(selected(&local), Some("Editor C"));
    assert_eq!(local.selection.selected, 2);
    assert_eq!(
        local
            .matching()
            .map(|app| app.id.as_str())
            .collect::<Vec<_>>(),
        ["Editor A", "Editor B", "Editor C"]
    );

    local.sync(&apps(&["Editor A", "Editor B", "Other"]));
    assert_eq!(local.selection.query, "ed");
    assert_eq!(selected(&local), Some("Editor B"));
    assert_eq!(local.selection.selected, 1);
}

#[test]
fn changing_the_query_resets_selection_instead_of_preserving_the_old_app() {
    let mut local = Local::default();
    local.sync(&apps(&["Editor A", "Editor B", "Other"]));
    local.select(2);
    local.search(|query| query.push_str("ed"));
    assert_eq!(selected(&local), Some("Editor A"));
    assert_eq!(local.selection.selected, 0);
    assert_eq!(local.selection.top, 0);

    local.sync(&apps(&["Editor A", "Editor B", "Editor C", "Other"]));
    assert_eq!(selected(&local), Some("Editor A"));
    local.search(|query| query.push_str(" missing"));
    assert_eq!(selected(&local), None);
    assert_eq!(local.selection.selected, 0);
    assert_eq!(local.selection.top, 0);

    local.search(String::clear);
    assert_eq!(selected(&local), Some("Editor A"));
}

#[test]
fn snapshots_keep_valid_scroll_positions_and_clamp_shorter_lists() {
    let ids: Vec<String> = (0..16).map(|index| format!("App {index:02}")).collect();
    let mut local = Local::default();
    local.sync(&apps(&ids.iter().map(String::as_str).collect::<Vec<_>>()));
    local.select(11);
    local.select(8);
    assert_eq!(local.selection.top, 4);

    let mut inserted = vec!["Added"];
    inserted.extend(ids.iter().map(String::as_str));
    local.sync(&apps(&inserted));
    assert_eq!(selected(&local), Some("App 08"));
    assert_eq!(local.selection.selected, 9);
    assert_eq!(local.selection.top, 4);

    local.sync(&apps(
        &ids[..10].iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(selected(&local), Some("App 08"));
    assert_eq!(local.selection.top, 2);
    assert!(local.selection.selected >= local.selection.top);
    assert!(local.selection.selected < local.selection.top + ROWS);

    local.select(9);
    local.sync(&apps(
        &ids[..3].iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(selected(&local), Some("App 02"));
    assert_eq!(local.selection.top, 0);
}

#[test]
fn closing_the_panel_resets_the_query_selection_and_scroll() {
    let ids: Vec<String> = (0..12).map(|index| format!("Editor {index:02}")).collect();
    let snapshot = apps(&ids.iter().map(String::as_str).collect::<Vec<_>>());
    let mut local = Local::default();
    local.sync(&snapshot);
    local.search(|query| query.push_str("ed"));
    local.select(10);
    assert!(local.selection.top > 0);

    local.panel_closed();
    local.sync(&snapshot);
    assert_eq!(local.selection.query, "");
    assert_eq!(selected(&local), Some("Editor 00"));
    assert_eq!(local.selection.selected, 0);
    assert_eq!(local.selection.top, 0);
}
