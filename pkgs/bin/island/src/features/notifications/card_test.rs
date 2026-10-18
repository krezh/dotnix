use std::time::{Duration, Instant, SystemTime};

use super::*;
use crate::features::notifications::notification::Expire;

fn notification(id: u32) -> Notification {
    Notification {
        id,
        app: "App".to_owned(),
        summary: "Hello".to_owned(),
        body: "Message".to_owned(),
        critical: false,
        icon: None,
        image: None,
        transient: false,
        resident: false,
        expire: Expire::Default,
        actions: Vec::new(),
        received: SystemTime::UNIX_EPOCH,
    }
}

fn layout(rows: &MissedList) -> Placed {
    let looks = rows
        .rows()
        .iter()
        .map(|_| Look {
            hidden: false,
            activate: panel::on(|_, _| {}),
            dismiss: panel::on(|_, _| {}),
            invoke: Vec::new(),
            countdown: None,
        })
        .collect();
    let Block::Placed(placed) = missed(rows, looks, "Nothing missed") else {
        panic!("missed list must keep its empty-state viewport");
    };
    placed
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
            rows.advance(1.0 / 60.0, now + Duration::from_secs_f32(frame as f32 / 60.0));
        }
        assert!(rows.rows().is_empty());
        let settled = layout(&rows);
        assert!(settled.blocks.iter().any(|(place, block)| {
            matches!(block, Block::Text(_)) && place.opacity == 1.0
        }));
    }
}
