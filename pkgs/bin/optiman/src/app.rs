use libadwaita as adw;
use libadwaita::prelude::*;

use crate::ui::MainWindow;

const APP_ID: &str = "com.github.optiman";

pub struct OptiManApp {
    app: adw::Application,
}

impl OptiManApp {
    pub fn new() -> Self {
        let app = adw::Application::builder().application_id(APP_ID).build();
        app.connect_activate(|app| {
            let window = MainWindow::new(app);
            window.present();
        });
        Self { app }
    }

    pub fn run(&self) -> i32 {
        self.app.run().into()
    }
}
