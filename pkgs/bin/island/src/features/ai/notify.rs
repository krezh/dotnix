use super::{Ai, model::Notice};
use crate::features::{
    Feature,
    notice::{self, Origin, Post},
};

/// What the notification center calls notices from here.
const SOURCE: &str = "AI";

/// Led by Jelly, which this feature lends under its name.
pub(super) fn send(notices: Vec<Notice>) {
    for notice in notices {
        let origin = Origin {
            feature: Ai::NAME.to_owned(),
            source: SOURCE.to_owned(),
            key: notice.key,
        };
        notice::post(Post::Show(notice::Notice {
            critical: notice.critical,
            lead: true,
            mood: Some(notice.feeling.name().to_owned()),
            ..notice::Notice::new(origin, notice.app, notice.title, notice.body)
        }));
    }
}
