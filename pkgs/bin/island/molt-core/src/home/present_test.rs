use super::*;
use crate::home::flow::Repository;
use crate::home::repository::RepositoryKind;

fn status_of(incoming: usize, local: usize) -> RepositoryStatus {
    RepositoryStatus {
        kind: RepositoryKind::Jj,
        upstream: "trunk".to_owned(),
        incoming,
        local,
        commits: Vec::new(),
    }
}

fn labels(card: &Card) -> Vec<&'static str> {
    card.actions.iter().map(|action| action.label).collect()
}

#[test]
fn incoming_changes_offer_a_pull() {
    let card = repository_card(&Repository::Ready(status_of(3, 1)));
    assert_eq!(card.detail, "jj · trunk");
    let status = card.status.unwrap();
    assert_eq!(status.text, "3 incoming · 1 local");
    assert_eq!(status.tone, Tone::Attention);
    assert_eq!(card.actions.iter().map(|a| a.label).collect::<Vec<_>>(), ["Check", "Pull"]);
}

#[test]
fn nothing_incoming_hides_pull() {
    let card = repository_card(&Repository::Ready(status_of(0, 0)));
    assert_eq!(card.status.as_ref().unwrap().text, "Up to date");
    assert_eq!(labels(&card), ["Check"]);
    let card = repository_card(&Repository::Ready(status_of(0, 2)));
    assert_eq!(card.status.as_ref().unwrap().text, "2 local");
}

#[test]
fn a_failed_check_offers_retry_on_its_own_button() {
    let card = repository_card(&Repository::CheckFailed("offline\nmore".to_owned()));
    assert_eq!(card.detail, "offline");
    assert_eq!(card.status.as_ref().unwrap().text, "Check failed");
    assert_eq!(labels(&card), ["Retry"]);
}

#[test]
fn a_failed_pull_retries_the_pull() {
    let card = repository_card(&Repository::UpdateFailed("conflict".to_owned()));
    assert_eq!(card.status.as_ref().unwrap().text, "Update failed");
    assert_eq!(labels(&card), ["Check", "Retry"]);
    assert!(card.actions[1].failed);
}

#[test]
fn pulling_keeps_the_card_and_runs_the_button() {
    let card = repository_card(&Repository::Updating(Box::new(Repository::Ready(
        status_of(3, 0),
    ))));
    assert_eq!(card.status.as_ref().unwrap().text, "3 incoming");
    assert_eq!(labels(&card), ["Check", "Applying"]);
    assert!(card.actions[1].busy && !card.actions[1].enabled);
}

#[test]
fn checking_runs_the_badge() {
    let card = repository_card(&Repository::Checking { pull: false });
    let status = card.status.unwrap();
    assert!(status.busy);
    assert_eq!(status.text, "Checking");
    assert_eq!(card.actions.iter().map(|a| a.label).collect::<Vec<_>>(), ["Checking…"]);
}
