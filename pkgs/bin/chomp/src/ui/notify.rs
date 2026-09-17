//! Desktop notification wrapper using notify-rust

use notify_rust::{Notification, Timeout, Urgency};
use std::process::{Command, Stdio};

pub struct Notifier;

impl Notifier {
    pub fn new() -> Self {
        Self
    }

    /// Sends an error notification with optional details.
    pub fn send_error(&self, message: &str, details: Option<&str>) {
        let full_message = if let Some(d) = details {
            format!("{}: {}", message, d)
        } else {
            message.to_string()
        };
        self.send(crate::APP_NAME, &full_message, Urgency::Critical);
    }

    /// Sends an info notification.
    pub fn send_info(&self, message: &str) {
        self.send(crate::APP_NAME, message, Urgency::Normal);
    }

    /// Sends a notification with an action button that opens a URL.
    ///
    /// Serving the action means outliving chomp itself, so this re-runs chomp as a
    /// detached process that only shows the notification and waits. Forking here
    /// instead would inherit the HTTP and D-Bus threads of the upload that produced
    /// the URL, and the child would deadlock on locks those threads left held.
    ///
    /// Falls back to a plain notification carrying the URL in its body.
    pub fn send_with_action(&self, title: &str, message: &str, url: &str) {
        let spawned = std::env::current_exe().ok().and_then(|exe| {
            Command::new(exe)
                .arg("--await-notification-action")
                .args([title, message, url])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| log::warn!("Failed to spawn notification process: {}", e))
                .ok()
        });

        if spawned.is_none() {
            self.send(title, &format!("{}\n\n{}", message, url), Urgency::Normal);
        }
    }

    /// Shows the action notification and blocks until it is answered or expires.
    ///
    /// This is the whole body of the detached process `send_with_action` spawns.
    pub fn await_action(title: &str, message: &str, url: &str) {
        let notification = Notification::new()
            .appname(crate::APP_NAME)
            .summary(title)
            .body(message)
            .urgency(Urgency::Normal)
            .timeout(Timeout::Milliseconds(60_000))
            .action("open", "Open URL")
            .show();

        match notification {
            Ok(handle) => handle.wait_for_action(|action| {
                if action == "open" {
                    if let Err(e) = open::that(url) {
                        log::warn!("Failed to open {}: {}", url, e);
                    }
                }
            }),
            Err(e) => log::warn!("Failed to send notification: {}", e),
        }
    }

    fn send(&self, app_name: &str, message: &str, urgency: Urgency) {
        if let Err(e) = Notification::new()
            .appname(crate::APP_NAME)
            .summary(app_name)
            .body(message)
            .urgency(urgency)
            .show()
        {
            log::warn!("Failed to send notification: {}", e);
        }
    }
}
