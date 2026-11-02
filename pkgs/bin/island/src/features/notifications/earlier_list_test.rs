use std::time::{Duration, Instant, SystemTime};

use super::{EarlierList, Entry};
use crate::features::notifications::{
    history::Earlier,
    notification::{Expire, Notification},
};

fn kept(id: u32, days_ago: u64) -> Earlier {
    Earlier {
        session: 1,
        notification: Notification {
            id,
            app: "App".to_owned(),
            summary: "Hello".to_owned(),
            body: "Message".to_owned(),
            critical: false,
            icon: None,
            image: None,
            lead: None,
            transient: false,
            resident: false,
            expire: Expire::Default,
            actions: Vec::new(),
            received: SystemTime::now() - Duration::from_secs(days_ago * 86_400),
            posted: None,
        },
    }
}

fn settle(list: &mut EarlierList) {
    let now = Instant::now();
    for frame in 0..=240 {
        list.advance(
            1.0 / 60.0,
            now + Duration::from_secs_f32(frame as f32 / 60.0),
        );
    }
}

fn days(list: &EarlierList) -> usize {
    list.rows()
        .iter()
        .filter(|row| !row.leaving && matches!(row.entry, Entry::Day(_)))
        .count()
}

#[test]
fn a_dismissed_one_slides_out_before_the_room_it_took_closes() {
    let mut list = EarlierList::default();
    list.sync(vec![kept(1, 0), kept(2, 0)], false, Instant::now());
    let full = list.visible_height();
    list.sync(vec![kept(2, 0)], true, Instant::now());
    let leaving = list.rows().iter().find(|row| row.leaving).unwrap();
    assert_eq!(leaving.out(), 0.0, "it starts where it was");
    assert_eq!(
        list.visible_height(),
        full,
        "keeping its room while it slides out"
    );
    settle(&mut list);
    assert!(list.rows().iter().all(|row| !row.leaving));
    assert!(list.visible_height() < full);
}

#[test]
fn a_swap_keeps_what_both_show_and_the_rest_leave_together() {
    let mut list = EarlierList::default();
    list.sync(
        (1..=40).map(|id| kept(id, 0)).collect(),
        false,
        Instant::now(),
    );
    let card = |list: &EarlierList, id: u32| {
        list.rows()
            .iter()
            .position(|row| matches!(&row.entry, Entry::Card(card) if card.notification.id == id))
            .unwrap()
    };
    let near_at = list.drawn_y()[card(&list, 3)];
    let both = vec![kept(3, 0), kept(30, 0), kept(100, 0)];
    list.swap(both.clone(), Instant::now());
    let gone: Vec<_> = list.rows().iter().filter(|row| row.leaving).collect();
    assert_eq!(gone.len(), 38, "every card not in both");
    assert!(
        gone.iter().all(|row| row.fold.target() == 0.0),
        "all on their way out at once, none waiting its turn"
    );
    let mut alone = EarlierList::default();
    alone.sync(both, false, Instant::now());
    assert_eq!(
        list.visible_height(),
        alone.visible_height(),
        "the old rows take no room from the new"
    );
    let near = &list.rows()[card(&list, 3)];
    assert!(!near.leaving, "one in both stays");
    assert_eq!(near.alpha.value(), 1.0, "and never fades");
    assert_eq!(near.y.value(), near_at, "sliding from where it was");
    let far = &list.rows()[card(&list, 30)];
    assert!(
        far.alpha.value() == 0.0 && far.alpha.target() == 1.0,
        "one from far out of sight fades in where it goes instead of sweeping across"
    );
    settle(&mut list);
    assert_eq!(list.rows().len(), 4, "a heading and the three");
}

#[test]
fn a_day_keeps_its_heading_only_while_it_has_notifications() {
    let mut list = EarlierList::default();
    list.sync(vec![kept(1, 0), kept(2, 1)], false, Instant::now());
    assert_eq!(days(&list), 2);
    list.sync(vec![kept(1, 0)], true, Instant::now());
    assert_eq!(days(&list), 1);
    settle(&mut list);
    assert_eq!(list.rows().len(), 2, "a heading and its one notification");
}
