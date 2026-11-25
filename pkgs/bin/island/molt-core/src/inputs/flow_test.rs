use super::*;

fn input(name: &str, last_modified: Option<i64>) -> FlakeInput {
    FlakeInput {
        name: name.to_owned(),
        source: format!("owner/{name}"),
        source_kind: "github".to_owned(),
        revision: format!("{name}0000000000000000"),
        last_modified,
        follows: None,
        flake_ref: Some(format!("github:owner/{name}")),
        remote_url: None,
        remote_ref: None,
    }
}

fn record(revision: &str) -> RevisionRecord {
    RevisionRecord {
        revision: revision.to_owned(),
        flake_ref: format!("github:owner/repo/{revision}"),
        date: String::new(),
        summary: String::new(),
    }
}

fn screen() -> Inputs {
    let mut screen = Inputs {
        directory: PathBuf::from("/nonexistent"),
        home_manager: false,
        inputs: Vec::new(),
        load_error: None,
        query: String::new(),
        oldest_first: false,
        selected: None,
        history: History::None,
        history_generation: 0,
        pin: None,
        busy: None,
        status: None,
        undo: None,
    };
    screen.set_inputs(vec![
        input("zen", Some(5)),
        input("home-manager", Some(9)),
        input("agenix", Some(1)),
        input("nixpkgs", None),
    ]);
    screen
}

#[test]
fn opens_on_the_first_input_and_fetches_its_history() {
    let screen = screen();
    assert_eq!(screen.selected.as_deref(), Some("nixpkgs"));
    assert_eq!(screen.history, History::Loading);
    let names: Vec<&str> = screen.shown().iter().map(|input| input.name.as_str()).collect();
    assert_eq!(names, ["nixpkgs", "home-manager", "agenix", "zen"]);
}

#[test]
fn oldest_first_keeps_the_pinned_inputs_on_top() {
    let mut screen = screen();
    screen.update(Msg::OldestFirst(true));
    let names: Vec<&str> = screen.shown().iter().map(|input| input.name.as_str()).collect();
    assert_eq!(names, ["nixpkgs", "home-manager", "agenix", "zen"]);
}

#[test]
fn a_late_history_answer_for_another_pick_is_dropped() {
    let mut screen = screen();
    let first = screen.history_generation;
    let (effect, _) = screen.update(Msg::Select("zen".to_owned()));
    assert!(matches!(effect, Some(Effect::History { .. })));
    screen.update(Msg::History(first, Ok(vec![record("old")])));
    assert_eq!(screen.history, History::Loading);
    screen.update(Msg::History(screen.history_generation, Ok(vec![record("new")])));
    assert_eq!(screen.history, History::Ready(vec![record("new")]));
}

#[test]
fn filtering_away_the_pick_picks_the_first_match() {
    let mut screen = screen();
    screen.update(Msg::Query("ZE".to_owned()));
    assert_eq!(screen.selected.as_deref(), Some("zen"));
    screen.update(Msg::Query("nothing".to_owned()));
    assert_eq!(screen.selected, None);
    assert_eq!(screen.history, History::None);
}

#[test]
fn picking_another_input_closes_the_pin_field() {
    let mut screen = screen();
    screen.update(Msg::TogglePin);
    screen.update(Msg::PinText("v1".to_owned()));
    assert!(screen.can_apply_pin());
    screen.update(Msg::Select("zen".to_owned()));
    assert_eq!(screen.pin, None);
}

#[test]
fn only_one_lock_change_runs_at_a_time() {
    let mut screen = screen();
    let (effect, _) = screen.update(Msg::UpdateAll);
    assert_eq!(effect, Some(Effect::Update(None)));
    assert_eq!(screen.busy.as_ref().unwrap().message, "Updating every input…");
    assert_eq!(screen.update(Msg::UpdateSelected).0, None);
    assert_eq!(screen.update(Msg::Select("zen".to_owned())).0, None);
}

#[test]
fn a_failed_change_shows_its_first_line() {
    let mut screen = screen();
    screen.update(Msg::UpdateAll);
    screen.update(Msg::Finished(
        Err("nix failed\ndetails".to_owned()),
        Ok(Vec::new()),
    ));
    assert_eq!(screen.busy, None);
    let status = screen.status.unwrap();
    assert_eq!(status.text, "nix failed");
    assert_eq!(status.error.as_deref(), Some("nix failed\ndetails"));
}

#[test]
fn locking_at_a_history_commit_names_it() {
    let mut screen = screen();
    let generation = screen.history_generation;
    screen.update(Msg::History(
        generation,
        Ok(vec![record("aaaaaaaaaaaaaaaaaaaa"), record("bbbbbbbbbbbbbbbbbbbb")]),
    ));
    let (effect, _) = screen.update(Msg::LockAt(1));
    assert_eq!(
        effect,
        Some(Effect::Revision {
            input: "nixpkgs".to_owned(),
            flake_ref: "github:owner/repo/bbbbbbbbbbbbbbbbbbbb".to_owned(),
        })
    );
    assert_eq!(screen.busy.unwrap().message, "Setting nixpkgs to bbbbbbbbbbbb…");
}
