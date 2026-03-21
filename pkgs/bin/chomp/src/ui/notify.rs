//! Desktop notification wrapper using notify-send

use std::process::Command;

const APP_NAME: &str = "APP_NAME";

pub struct Notifier;

impl Notifier {
    pub fn new() -> Self {
        Self
    }

    /// Sends a success notification.
    pub fn send_success(&self, message: &str) {
        self.send("APP_NAME", message, "normal");
    }

    /// Sends an error notification with optional details.
    pub fn send_error(&self, message: &str, details: Option<&str>) {
        let full_message = if let Some(d) = details {
            format!("{}: {}", message, d)
        } else {
            message.to_string()
        };
        self.send("APP_NAME", &full_message, "critical");
    }

    /// Sends an info notification.
    pub fn send_info(&self, message: &str) {
        self.send("APP_NAME", message, "normal");
    }

    /// Sends a notification with a clickable action button that opens a URL.
    pub fn send_with_action(&self, message: &str, url: &str) {
        let output = Command::new("notify-send")
            .args([
                "-t", "5000",
                "-u", "normal",
                "--transient",
                "-A", "open=Open URL",
                "APP_NAME",
                message,
            ])
            .output();

        // If user clicks the action button, open the URL
        if let Ok(output) = output {
            let response = String::from_utf8_lossy(&output.stdout);
            if response.trim() == "open" {
                Self::open_url(url);
            }
        }
    }

    /// Opens a URL using the system's default browser.
    fn open_url(url: &str) {
        let commands = ["xdg-open", "open", "firefox", "chromium", "google-chrome"];

        for cmd in commands {
            if let Ok(mut child) = Command::new(cmd).arg(url).spawn() {
                // Detach from the process so browser can outlive the notification
                let _ = child.wait();
                log::info!("Opened URL with {}", cmd);
                return;
            }
        }

        log::warn!("Failed to open URL: no suitable browser found");
    }

    fn send(&self, app_name: &str, message: &str, urgency: &str) {
        let _ = Command::new("notify-send")
            .args(["-a", app_name, "-u", urgency, message])
            .spawn();
    }
}
