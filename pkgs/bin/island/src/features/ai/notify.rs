use std::time::SystemTime;

use super::{Ai, model::Notice};
use crate::features::{
    Feature,
    notice::{self, NoticeAction, Origin, Post},
};

/// What the notification center calls notices from here.
const SOURCE: &str = "AI";
/// The AI panel's view a notice about a session opens it on.
pub(super) const SESSION_VIEW: &str = "sessions";

/// Led by Jelly, which this feature lends under its name.
pub(super) fn send(notices: Vec<Notice>) {
    for notice in notices {
        let actions = notice
            .session
            .iter()
            .map(|session| NoticeAction {
                key: "show".to_owned(),
                label: "Show session".to_owned(),
                opens: Some((
                    Ai::PANEL.unwrap_or(Ai::NAME).to_owned(),
                    format!("{SESSION_VIEW} {session}"),
                )),
            })
            .collect();
        notice::post(Post::Show(notice::Notice {
            origin: Origin {
                feature: Ai::NAME.to_owned(),
                source: SOURCE.to_owned(),
                key: notice.key,
            },
            app: notice.app.to_owned(),
            title: notice.title,
            body: notice.body,
            critical: notice.critical,
            detail: None,
            via: false,
            lead: true,
            actions,
            received: SystemTime::now(),
            quiet: false,
        }));
    }
}
