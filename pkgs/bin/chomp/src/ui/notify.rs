//! Desktop notification wrapper using notify-rust

use notify_rust::{Notification, Timeout, Urgency};

pub struct Notifier;

impl Notifier {
    pub fn new() -> Self {
        Self
    }

    /// Sends a success notification.
    pub fn send_success(&self, message: &str) {
        self.send(crate::APP_NAME, message, Urgency::Normal);
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
    /// Forks a background process to wait for the button click, allowing the main
    /// chomp process to exit immediately.
    pub fn send_with_action(&self, message: &str, url: &str) {
        let url_owned = url.to_string();
        let message_owned = message.to_string();

        // Fork a background process to handle the notification action
        // This allows chomp to exit while the notification stays active
        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { .. }) => {
                // Parent process continues and exits normally
            }
            Ok(nix::unistd::ForkResult::Child) => {
                // Child process handles the notification
                match Notification::new()
                    .summary(crate::APP_NAME)
                    .body(&message_owned)
                    .urgency(Urgency::Normal)
                    .timeout(Timeout::Never)
                    .action("open", "Open URL")
                    .show()
                {
                    Ok(handle) => {
                        handle.wait_for_action(|action| {
                            if action == "open" {
                                if open::that(&url_owned).is_ok() {
                                    log::info!("Opened URL: {}", url_owned);
                                }
                            }
                        });
                    }
                    Err(_) => {}
                }
                // Exit the child process
                std::process::exit(0);
            }
            Err(_) => {
                // Fork failed, fall back to simple notification
                let body = format!("{}\n\n{}", message, url);
                let _ = Notification::new()
                    .summary(crate::APP_NAME)
                    .body(&body)
                    .urgency(Urgency::Normal)
                    .timeout(Timeout::Never)
                    .show();
            }
        }
    }

    fn send(&self, app_name: &str, message: &str, urgency: Urgency) {
        if let Err(e) = Notification::new()
            .summary(app_name)
            .body(message)
            .urgency(urgency)
            .show()
        {
            log::warn!("Failed to send notification: {}", e);
        }
    }
}
