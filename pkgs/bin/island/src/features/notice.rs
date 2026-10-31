//! Notifications a feature hands the notification center itself, rather than sending island one
//! over D-Bus, and what the center tells the feature back about them.

use std::{
    collections::HashMap,
    sync::{
        Mutex, OnceLock,
        mpsc::{self, Receiver, Sender},
    },
    time::SystemTime,
};

/// Who posted a notification: the feature, the name its source goes by in the panel, and the
/// feature's own name for it, which a later post under the same key replaces.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Origin {
    pub feature: String,
    pub source: String,
    pub key: String,
}

#[derive(Clone, Debug)]
pub struct Notice {
    pub origin: Origin,
    pub app: String,
    pub title: String,
    pub body: String,
    pub critical: bool,
    /// After the time in its overline, such as a priority.
    pub detail: Option<String>,
    /// Names its source in its overline, after the app.
    pub via: bool,
    /// Led by the mark its feature lends, rather than the app's letter.
    pub lead: bool,
    pub actions: Vec<NoticeAction>,
    pub received: SystemTime,
    /// There before island heard of it, such as messages waiting on a server: it waits with what
    /// was missed rather than popping up.
    pub quiet: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeAction {
    /// `default` is clicking the notification.
    pub key: String,
    pub label: String,
    /// A panel, by name, and its view, that it opens, such as the session it is about.
    pub opens: Option<(String, String)>,
}

/// Why a source cannot deliver now: what is wrong, and more about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trouble {
    pub title: String,
    pub detail: String,
}

pub enum Post {
    Show(Notice),
    /// Every one the feature still has; the others go.
    Keep {
        feature: String,
        keys: Vec<String>,
    },
    /// Shown above the list, with a way to retry; `None` once it can deliver again.
    Trouble {
        feature: String,
        trouble: Option<Trouble>,
    },
}

pub enum Reply {
    /// Dismissed or cleared, so the feature can let go of it too.
    Dismissed(String),
    /// Its key and the action's.
    Action(String, String),
    /// The notification center opened, so a feature can check what it posted is still current.
    Looked,
    /// Asked to try again at once after a trouble.
    Retry,
}

struct Inbox {
    posts: Sender<Post>,
    unread: Mutex<Option<Receiver<Post>>>,
    replies: Mutex<HashMap<String, Sender<Reply>>>,
}

fn inbox() -> &'static Inbox {
    static INBOX: OnceLock<Inbox> = OnceLock::new();
    INBOX.get_or_init(|| {
        let (posts, unread) = mpsc::channel();
        Inbox {
            posts,
            unread: Mutex::new(Some(unread)),
            replies: Mutex::default(),
        }
    })
}

/// Kept until the center takes them, so one posted before it starts still arrives.
pub fn post(post: Post) {
    let _ = inbox().posts.send(post);
}

/// The center's, once.
pub fn take() -> Option<Receiver<Post>> {
    inbox().unread.lock().ok()?.take()
}

/// What the center tells `feature`; a later call replaces the earlier one.
pub fn listen(feature: &str) -> Receiver<Reply> {
    let (sender, replies) = mpsc::channel();
    if let Ok(mut listeners) = inbox().replies.lock() {
        listeners.insert(feature.to_owned(), sender);
    }
    replies
}

pub fn reply(feature: &str, reply: Reply) {
    if let Ok(listeners) = inbox().replies.lock()
        && let Some(listener) = listeners.get(feature)
    {
        let _ = listener.send(reply);
    }
}

pub fn looked() {
    if let Ok(listeners) = inbox().replies.lock() {
        for listener in listeners.values() {
            let _ = listener.send(Reply::Looked);
        }
    }
}
