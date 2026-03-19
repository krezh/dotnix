use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::sync::{Arc, Mutex};

use crate::config::TrackedGames;
use crate::steam::SteamGame;

#[derive(Clone)]
pub struct GameDetailsPanel {
    container: gtk4::Box,
    game_name_label: gtk4::Label,
    app_id_label: gtk4::Label,
    install_path_label: gtk4::Label,
    exe_path_label: gtk4::Label,
    status_label: gtk4::Label,
    version_label: gtk4::Label,
    install_button: gtk4::Button,
    configure_button: gtk4::Button,
    remove_button: gtk4::Button,
    current_game: Arc<Mutex<Option<(String, String)>>>, // (app_id, game_name)
}

impl GameDetailsPanel {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 24);
        container.set_margin_start(24);
        container.set_margin_end(24);
        container.set_margin_top(24);
        container.set_margin_bottom(24);

        // Game info section
        let info_group = adw::PreferencesGroup::new();
        info_group.set_title("Game Information");

        let game_name_label = gtk4::Label::new(Some("No game selected"));
        game_name_label.add_css_class("title-1");
        game_name_label.set_halign(gtk4::Align::Start);
        game_name_label.set_margin_bottom(12);

        let app_id_label = gtk4::Label::new(Some(""));
        app_id_label.add_css_class("dim-label");
        app_id_label.set_halign(gtk4::Align::Start);

        let install_path_label = gtk4::Label::new(Some(""));
        install_path_label.add_css_class("caption");
        install_path_label.add_css_class("dim-label");
        install_path_label.set_halign(gtk4::Align::Start);
        install_path_label.set_wrap(true);
        install_path_label.set_xalign(0.0);

        let exe_path_label = gtk4::Label::new(Some(""));
        exe_path_label.add_css_class("caption");
        exe_path_label.add_css_class("dim-label");
        exe_path_label.set_halign(gtk4::Align::Start);
        exe_path_label.set_wrap(true);
        exe_path_label.set_xalign(0.0);

        let info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        info_box.append(&game_name_label);
        info_box.append(&app_id_label);
        info_box.append(&install_path_label);
        info_box.append(&exe_path_label);

        // OptiScaler status section
        let status_group = adw::PreferencesGroup::new();
        status_group.set_title("OptiScaler Status");
        status_group.set_margin_top(12);

        let status_label = gtk4::Label::new(Some("Not installed"));
        status_label.set_halign(gtk4::Align::Start);

        let version_label = gtk4::Label::new(Some(""));
        version_label.add_css_class("dim-label");
        version_label.set_halign(gtk4::Align::Start);

        let status_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        status_box.append(&status_label);
        status_box.append(&version_label);

        // Action buttons
        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        button_box.set_margin_top(24);
        button_box.set_halign(gtk4::Align::Start);

        let install_button = gtk4::Button::builder()
            .label("Install OptiScaler")
            .css_classes(vec!["suggested-action", "pill"])
            .build();

        let configure_button = gtk4::Button::builder()
            .label("Configure")
            .css_classes(vec!["pill"])
            .sensitive(false)
            .build();

        let remove_button = gtk4::Button::builder()
            .label("Remove")
            .css_classes(vec!["destructive-action", "pill"])
            .sensitive(false)
            .build();

        button_box.append(&install_button);
        button_box.append(&configure_button);
        button_box.append(&remove_button);

        container.append(&info_box);
        container.append(&status_box);
        container.append(&button_box);

        Self {
            container,
            game_name_label,
            app_id_label,
            install_path_label,
            exe_path_label,
            status_label,
            version_label,
            install_button,
            configure_button,
            remove_button,
            current_game: Arc::new(Mutex::new(None)),
        }
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.container.clone().upcast()
    }

    pub fn show_game(&self, game: &SteamGame, tracked_games: &Arc<Mutex<TrackedGames>>) {
        // Update current game
        if let Ok(mut current) = self.current_game.lock() {
            *current = Some((game.app_id.clone(), game.name.clone()));
        }

        // Update game info
        self.game_name_label.set_text(&game.name);
        self.app_id_label.set_text(&format!("AppID: {}", game.app_id));
        self.install_path_label.set_text(&game.install_dir.to_string_lossy());

        // Update OptiScaler status
        if let Ok(tracked) = tracked_games.lock() {
            if let Some(entry) = tracked.get_game(&game.app_id) {
                if entry.installed {
                    self.status_label.set_text("Installed");
                    
                    if let Some(version) = &entry.optiscaler_version {
                        self.version_label.set_text(&format!("Version: {}", version));
                        self.version_label.set_visible(true);
                    } else {
                        self.version_label.set_visible(false);
                    }

                    // Update button states
                    self.install_button.set_label("Update");
                    self.install_button.remove_css_class("suggested-action");
                    self.configure_button.set_sensitive(true);
                    self.remove_button.set_sensitive(true);
                } else {
                    self.status_label.set_text("Not installed");
                    self.version_label.set_visible(false);

                    // Update button states
                    self.install_button.set_label("Install OptiScaler");
                    self.install_button.add_css_class("suggested-action");
                    self.configure_button.set_sensitive(false);
                    self.remove_button.set_sensitive(false);
                }
            }
        }
    }

    pub fn update(
        &self,
        name: &str,
        app_id: &str,
        installed: bool,
        version: Option<String>,
        proxy_dll: Option<String>,
        install_path: Option<String>,
        exe_path: Option<String>,
    ) {
        // Store current game
        if let Ok(mut current) = self.current_game.lock() {
            *current = Some((app_id.to_string(), name.to_string()));
        }

        // Update game info
        self.game_name_label.set_text(name);
        self.app_id_label.set_text(&format!("AppID: {}", app_id));
        
        if let Some(path) = install_path {
            self.install_path_label.set_text(&format!("Install: {}", path));
            self.install_path_label.set_visible(true);
        } else {
            self.install_path_label.set_visible(false);
        }
        
        if let Some(path) = exe_path {
            self.exe_path_label.set_text(&format!("Executable: {}", path));
            self.exe_path_label.set_visible(true);
        } else {
            self.exe_path_label.set_text("Executable: Not detected");
            self.exe_path_label.set_visible(true);
        }

        // Update OptiScaler status
        if installed {
            self.status_label.set_text("Installed");
            
            let mut version_text = String::new();
            if let Some(v) = version {
                version_text.push_str(&format!("Version: {}", v));
            }
            if let Some(dll) = proxy_dll {
                if !version_text.is_empty() {
                    version_text.push_str(" • ");
                }
                version_text.push_str(&format!("Proxy: {}", dll));
            }
            
            if !version_text.is_empty() {
                self.version_label.set_text(&version_text);
                self.version_label.set_visible(true);
            } else {
                self.version_label.set_visible(false);
            }

            // Update button states
            self.install_button.set_label("Update");
            self.install_button.remove_css_class("suggested-action");
            self.install_button.set_sensitive(true);
            self.configure_button.set_sensitive(true);
            self.remove_button.set_sensitive(true);
        } else {
            self.status_label.set_text("Not installed");
            self.version_label.set_visible(false);

            // Update button states
            self.install_button.set_label("Install OptiScaler");
            self.install_button.add_css_class("suggested-action");
            self.install_button.set_sensitive(true);
            self.configure_button.set_sensitive(false);
            self.remove_button.set_sensitive(false);
        }
    }

    pub fn clear(&self) {
        self.game_name_label.set_text("No game selected");
        self.app_id_label.set_text("");
        self.install_path_label.set_text("");
        self.status_label.set_text("");
        self.version_label.set_visible(false);

        self.install_button.set_sensitive(false);
        self.configure_button.set_sensitive(false);
        self.remove_button.set_sensitive(false);

        if let Ok(mut current) = self.current_game.lock() {
            *current = None;
        }
    }

    pub fn install_button(&self) -> &gtk4::Button {
        &self.install_button
    }

    pub fn configure_button(&self) -> &gtk4::Button {
        &self.configure_button
    }

    pub fn remove_button(&self) -> &gtk4::Button {
        &self.remove_button
    }

    pub fn current_game(&self) -> Option<(String, String)> {
        self.current_game.lock().ok()?.clone()
    }
}
