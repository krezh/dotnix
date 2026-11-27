use super::*;
use crate::home::repository::RepositoryKind;

fn status(incoming: usize, local: usize) -> RepositoryStatus {
    RepositoryStatus {
        kind: RepositoryKind::Jj,
        upstream: "trunk".to_owned(),
        incoming,
        local,
        commits: Vec::new(),
    }
}

fn home() -> Home {
    Home {
        config: Err("none".to_owned()),
        system: SystemStatus::default(),
        inputs: None,
        cleanup: None,
        watchlist: WatchlistSummary::default(),
        repository: Repository::Checking { pull: false },
        reviewed: Vec::new(),
    }
}

#[test]
fn check_result_settles_the_card() {
    let mut home = home();
    assert_eq!(home.update(Msg::Checked(Ok(status(2, 0)))), None);
    assert_eq!(home.repository, Repository::Ready(status(2, 0)));
}

#[test]
fn check_is_ignored_while_one_runs() {
    let mut home = home();
    assert_eq!(home.update(Msg::Check), None);
    home.update(Msg::Checked(Err("offline".to_owned())));
    assert_eq!(home.update(Msg::Check), Some(Effect::Check));
    assert_eq!(home.repository, Repository::Checking { pull: false });
}

#[test]
fn pull_stays_on_show_while_rechecking() {
    let mut home = home();
    home.update(Msg::Checked(Ok(status(2, 0))));
    home.update(Msg::Check);
    assert_eq!(home.repository, Repository::Checking { pull: true });
    assert_eq!(home.update(Msg::Pull), None);
}

#[test]
fn pull_needs_incoming_changes() {
    let mut home = home();
    home.update(Msg::Checked(Ok(status(0, 1))));
    assert_eq!(home.update(Msg::Pull), None);
    home.update(Msg::Check);
    home.update(Msg::Checked(Ok(status(3, 0))));
    assert_eq!(home.update(Msg::Pull), Some(Effect::Pull));
    assert_eq!(
        home.repository,
        Repository::Updating(Box::new(Repository::Ready(status(3, 0))))
    );
}

#[test]
fn a_failed_pull_can_be_retried() {
    let mut home = home();
    home.update(Msg::Checked(Ok(status(3, 0))));
    home.update(Msg::Pull);
    home.update(Msg::Pulled(Err("conflict".to_owned())));
    assert_eq!(
        home.repository,
        Repository::UpdateFailed("conflict".to_owned())
    );
    assert_eq!(home.update(Msg::Pull), Some(Effect::Pull));
    home.update(Msg::Pulled(Ok(status(0, 0))));
    assert_eq!(home.repository, Repository::Ready(status(0, 0)));
}

#[test]
fn stale_results_are_dropped() {
    let mut home = home();
    home.update(Msg::Checked(Ok(status(1, 0))));
    home.update(Msg::Checked(Ok(status(9, 9))));
    home.update(Msg::Pulled(Ok(status(9, 9))));
    assert_eq!(home.repository, Repository::Ready(status(1, 0)));
}
