use gtk4::prelude::*;
use libadwaita as adw;
use std::sync::{Arc, Mutex};

use crate::config::TrackedGames;
use crate::ui::MainWindow;

const APP_ID: &str = "com.github.optiman";

pub struct OptiManApp {
    app: adw::Application,
    tracked_games: Arc<Mutex<TrackedGames>>,
}

impl OptiManApp {
    pub fn new() -> Self {
        let app = adw::Application::builder()
            .application_id(APP_ID)
            .build();

        // Load or create tracked games configuration
        let tracked_games = Arc::new(Mutex::new(
            TrackedGames::load().unwrap_or_else(|e| {
                tracing::warn!("Failed to load tracked games: {}", e);
                TrackedGames::default()
            })
        ));

        let tracked_games_clone = tracked_games.clone();
        app.connect_activate(move |app| {
            Self::on_activate(app, tracked_games_clone.clone());
        });

        Self {
            app,
            tracked_games,
        }
    }

    fn on_activate(app: &adw::Application, tracked_games: Arc<Mutex<TrackedGames>>) {
        tracing::info!("Activating OptiMan application");

        let window = MainWindow::new(app, tracked_games);
        window.present();
    }

    pub fn run(&self) -> i32 {
        self.app.run().into()
    }
}
