use std::time::{Duration, Instant, SystemTime};

use super::*;
use crate::features::notifications::{missed_list::Body, notification::Expire};

fn notification(id: u32) -> Notification {
    Notification {
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
        received: SystemTime::UNIX_EPOCH,
        posted: None,
    }
}

fn layout(rows: &MissedList) -> Placed {
    let looks = rows
        .rows()
        .iter()
        .map(|row| match row.notification() {
            Some(_) => RowLook::Card(Look {
                lead: None,
                corner: None,
                hidden: false,
                activate: panel::on(|_, _| {}),
                dismiss: panel::on(|_, _| {}),
                invoke: Vec::new(),
                countdown: None,
            }),
            None => RowLook::Head(HeadLook {
                toggle: panel::on(|_, _| {}),
                clear: panel::on(|_, _| {}),
                more: None,
                close: None,
            }),
        })
        .collect();
    let Block::Placed(placed) = missed(rows, looks, "Nothing missed") else {
        panic!("missed list must keep its empty-state viewport");
    };
    placed
}

fn from(app: &str, id: u32) -> Notification {
    Notification {
        app: app.to_owned(),
        ..notification(id)
    }
}

fn settle(rows: &mut MissedList) {
    let now = Instant::now();
    for frame in 0..=240 {
        rows.advance(
            1.0 / 60.0,
            now + Duration::from_secs_f32(frame as f32 / 60.0),
        );
    }
}

#[test]
fn an_apps_cards_fold_under_its_heading_until_opened() {
    let now = Instant::now();
    let mut rows = MissedList::default();
    let items = [from("Sonarr", 1), from("Firefox", 2), from("Sonarr", 3), from("Sonarr", 4)];
    rows.sync(&items, false, now);
    let order: Vec<_> = rows
        .rows()
        .iter()
        .map(|row| match &row.body {
            Body::Head { app, count, .. } => format!("{app} ({count})"),
            Body::Card(card) => card.id.to_string(),
        })
        .collect();
    assert_eq!(order, ["Sonarr (3)", "1", "3", "4", "2"], "grouped, newest app first");
    let depths: Vec<_> = rows.rows().iter().map(|row| row.depth).collect();
    assert_eq!(depths, [0, 0, 1, 2, 0]);
    let front = rows.rows()[1].y.target();
    assert!(
        rows.rows()[2..4]
            .iter()
            .all(|row| row.y.target() == front && row.alpha.target() == 0.0),
        "tucked out of sight behind the front card"
    );
    assert_eq!(rows.rows()[1].peeks, 2, "two edges show below the front card");
    assert_eq!(rows.rows()[1].peek.target(), 1.0);
    let folded = rows.visible_height();
    let shown = |rows: &MissedList| {
        let placed = layout(rows);
        placed
            .blocks
            .iter()
            .filter(|(place, _)| place.drawn.is_some())
            .count()
    };
    assert_eq!(shown(&rows), 2);

    rows.toggle("Sonarr");
    settle(&mut rows);
    assert!(rows.rows().iter().all(|row| row.depth == 0 && row.clip.is_none()));
    assert!(rows.rows().iter().all(|row| row.alpha.target() == 1.0));
    assert_eq!(shown(&rows), 0, "no edges once spread out");
    assert!(rows.visible_height() > folded + 2.0 * missed_height(&from("Sonarr", 3), 1) - 1.0);

    rows.toggle("Sonarr");
    settle(&mut rows);
    assert_eq!(rows.visible_height(), folded);
}

fn apps(rows: &MissedList) -> Vec<String> {
    let mut apps: Vec<String> = Vec::new();
    for row in rows.rows().iter().filter(|row| !row.leaving) {
        if let Some(card) = row.notification()
            && apps.last() != Some(&card.app)
        {
            apps.push(card.app.clone());
        }
    }
    apps
}

#[test]
fn a_group_keeps_its_place_until_something_new_arrives_for_another() {
    let now = Instant::now();
    let mut rows = MissedList::default();
    rows.sync(
        &[from("Radarr", 5), from("Firefox", 4), from("Radarr", 3)],
        false,
        now,
    );
    assert_eq!(apps(&rows), ["Radarr", "Firefox"]);
    rows.sync(&[from("Firefox", 4), from("Radarr", 3)], true, now);
    assert_eq!(apps(&rows), ["Radarr", "Firefox"], "dismissing its newest moves nothing");
    rows.sync(
        &[from("Firefox", 6), from("Firefox", 4), from("Radarr", 3)],
        true,
        now,
    );
    assert_eq!(apps(&rows), ["Firefox", "Radarr"], "a new arrival brings its group up");
}

#[test]
fn a_lone_card_has_no_heading_and_a_dissolved_group_loses_it() {
    let now = Instant::now();
    let mut rows = MissedList::default();
    rows.sync(&[from("Sonarr", 1), from("Sonarr", 2)], false, now);
    assert!(rows.rows().iter().any(|row| row.notification().is_none()));
    rows.sync(&[from("Sonarr", 1)], true, now);
    settle(&mut rows);
    assert_eq!(rows.rows().len(), 1);
    assert_eq!(rows.rows()[0].depth, 0);
}

#[test]
fn a_line_opening_over_the_list_moves_the_list_down_as_it_opens() {
    let mut rows = MissedList::default();
    rows.sync(&[from("Sonarr", 1)], false, Instant::now());
    let top = |blocks: Vec<Block>| {
        let [Block::Placed(placed)] = blocks.as_slice() else {
            panic!("one list");
        };
        let card = placed
            .blocks
            .iter()
            .find(|(_, block)| matches!(block, Block::Frame(..)))
            .map(|(place, _)| place.y)
            .unwrap();
        (card, placed.visible.unwrap(), placed.blocks.len())
    };
    let shut = top(opening(layout_block(&rows), vec![Block::note("Trouble")], 0.0));
    let half = top(opening(layout_block(&rows), vec![Block::note("Trouble")], 0.5));
    let open = top(opening(layout_block(&rows), vec![Block::note("Trouble")], 1.0));
    assert_eq!(shut.2 + 1, open.2, "the line joins the list once opening");
    assert!(shut.0 < half.0 && half.0 < open.0);
    assert!(shut.1 < half.1 && half.1 < open.1);
}

fn layout_block(rows: &MissedList) -> Block {
    Block::Placed(layout(rows))
}

#[test]
fn dismissing_last_or_clearing_keeps_the_final_empty_height_through_the_fade() {
    for count in [1, 3] {
        let now = Instant::now();
        let mut rows = MissedList::default();
        let items: Vec<_> = (1..=count).map(notification).collect();
        rows.sync(&items, false, now);
        let occupied = layout(&rows).visible.unwrap();
        rows.sync(&[], true, now);
        let empty_height = layout(&MissedList::default()).visible.unwrap();
        assert!(empty_height > 0.0 && empty_height < occupied);
        assert!(rows.rows().iter().any(|row| row.leaving));
        for frame in 0..=120 {
            let view = layout(&rows);
            assert_eq!(view.visible, Some(empty_height), "frame {frame}");
            rows.advance(
                1.0 / 60.0,
                now + Duration::from_secs_f32(frame as f32 / 60.0),
            );
        }
        assert!(rows.rows().is_empty());
        let settled = layout(&rows);
        assert!(
            settled
                .blocks
                .iter()
                .any(|(place, block)| { matches!(block, Block::Text(_)) && place.opacity == 1.0 })
        );
    }
}
