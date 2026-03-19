use gtk4::prelude::*;
use gtk4::{gio, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::sync::{Arc, Mutex};

use crate::config::TrackedGames;
use crate::steam::{self, SteamGame};
use crate::optiscaler::{GitHubClient, Installer};
use super::game_list::GameListView;
use super::game_details::GameDetailsPanel;
use super::install_dialog::InstallDialog;
use std::path::PathBuf;

pub struct MainWindow {
    window: adw::ApplicationWindow,
    tracked_games: Arc<Mutex<TrackedGames>>,
    game_list_view: GameListView,
    details_panel: GameDetailsPanel,
    toast_overlay: adw::ToastOverlay,
}

impl MainWindow {
    pub fn new(app: &adw::Application, tracked_games: Arc<Mutex<TrackedGames>>) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("OptiMan")
            .default_width(1000)
            .default_height(700)
            .build();

        let game_list_view = GameListView::new();
        let details_panel = GameDetailsPanel::new();
        let toast_overlay = adw::ToastOverlay::new();

        let instance = Self {
            window,
            tracked_games,
            game_list_view,
            details_panel,
            toast_overlay,
        };

        instance.build_ui();
        instance
    }

    fn build_ui(&self) {
        // Create header bar
        let header_bar = adw::HeaderBar::new();

        // Add refresh button
        let refresh_button = gtk4::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh game list")
            .build();

        header_bar.pack_start(&refresh_button);

        // Add menu button
        let menu_button = gtk4::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Main menu")
            .build();

        let menu = gio::Menu::new();
        menu.append(Some("Preferences"), Some("app.preferences"));
        menu.append(Some("About OptiMan"), Some("app.about"));
        menu_button.set_menu_model(Some(&menu));

        header_bar.pack_end(&menu_button);

        // Create main content area
        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        // Add search bar
        let search_entry = gtk4::SearchEntry::builder()
            .placeholder_text("Search games...")
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        content_box.append(&search_entry);

        // Create split view (sidebar + details)
        let split_view = adw::NavigationSplitView::builder()
            .sidebar_width_fraction(0.3)
            .min_sidebar_width(280.0)
            .max_sidebar_width(400.0)
            .build();

        // Create sidebar with game list
        let sidebar_page = adw::NavigationPage::builder()
            .title("Games")
            .child(&self.game_list_view.widget())
            .build();

        split_view.set_sidebar(Some(&sidebar_page));

        // Create details panel
        let details_page = adw::NavigationPage::builder()
            .title("Game Details")
            .child(&self.details_panel.widget())
            .build();

        split_view.set_content(Some(&details_page));

        // Make split view expand to fill available space
        split_view.set_vexpand(true);
        split_view.set_hexpand(true);

        content_box.append(&split_view);

        // Create toolbar view (combines header and content)
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&content_box));

        // Wrap in toast overlay
        self.toast_overlay.set_child(Some(&toolbar_view));
        
        self.window.set_content(Some(&self.toast_overlay));

        // Connect game selection to details panel
        let details_panel_clone = self.details_panel.clone();
        let tracked_games_clone = self.tracked_games.clone();
        self.game_list_view.connect_row_selected(move |game_data| {
            if let Some((name, app_id, installed)) = game_data {
                // Get additional game info from tracked games
                let (version, proxy_dll, install_path, exe_path) = if let Ok(tracked) = tracked_games_clone.lock() {
                    if let Some(entry) = tracked.games.get(&app_id) {
                        (
                            entry.optiscaler_version.clone(),
                            entry.proxy_dll.clone(),
                            Some(entry.install_path.clone()),
                            entry.exe_path.clone(),
                        )
                    } else {
                        (None, None, None, None)
                    }
                } else {
                    (None, None, None, None)
                };
                
                details_panel_clone.update(&name, &app_id, installed, version, proxy_dll, install_path, exe_path);
            } else {
                details_panel_clone.clear();
            }
        });

        // Connect install button
        let window_clone = self.window.clone();
        let details_panel_clone = self.details_panel.clone();
        let tracked_games_clone = self.tracked_games.clone();
        let toast_overlay_clone = self.toast_overlay.clone();
        let game_list_clone = self.game_list_view.clone();
        self.details_panel.install_button().connect_clicked(move |_| {
            let window = window_clone.clone();
            let details_panel = details_panel_clone.clone();
            let tracked_games = tracked_games_clone.clone();
            let toast_overlay = toast_overlay_clone.clone();
            let game_list = game_list_clone.clone();
            
            glib::spawn_future_local(async move {
                match Self::handle_install(&window, &details_panel, tracked_games.clone()).await {
                    Ok((installed, app_id)) => {
                        if installed {
                            let toast = adw::Toast::new("OptiScaler installed successfully");
                            toast_overlay.add_toast(toast);
                            // Update just this game's status in the list
                            game_list.update_game_status(&app_id, true);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Installation failed: {}", e);
                        
                        let error_dialog = adw::AlertDialog::builder()
                            .heading("Installation Failed")
                            .body(&format!("Failed to install OptiScaler: {}", e))
                            .build();
                        error_dialog.add_response("ok", "OK");
                        error_dialog.present(Some(&window));
                    }
                }
            });
        });

        // Connect remove button
        let window_clone = self.window.clone();
        let details_panel_clone = self.details_panel.clone();
        let tracked_games_clone = self.tracked_games.clone();
        let toast_overlay_clone = self.toast_overlay.clone();
        let game_list_clone = self.game_list_view.clone();
        self.details_panel.remove_button().connect_clicked(move |_| {
            let window = window_clone.clone();
            let details_panel = details_panel_clone.clone();
            let tracked_games = tracked_games_clone.clone();
            let toast_overlay = toast_overlay_clone.clone();
            let game_list = game_list_clone.clone();
            
            glib::spawn_future_local(async move {
                match Self::handle_remove(&window, &details_panel, tracked_games.clone()).await {
                    Ok((removed, app_id)) => {
                        if removed {
                            let toast = adw::Toast::new("OptiScaler removed successfully");
                            toast_overlay.add_toast(toast);
                            // Update just this game's status in the list
                            game_list.update_game_status(&app_id, false);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Removal failed: {}", e);
                        
                        let error_dialog = adw::AlertDialog::builder()
                            .heading("Removal Failed")
                            .body(&format!("Failed to remove OptiScaler: {}", e))
                            .build();
                        error_dialog.add_response("ok", "OK");
                        error_dialog.present(Some(&window));
                    }
                }
            });
        });

        // Connect refresh button
        let tracked_games_clone = self.tracked_games.clone();
        let game_list_clone = self.game_list_view.clone();
        refresh_button.connect_clicked(move |_| {
            Self::refresh_games(tracked_games_clone.clone(), game_list_clone.clone());
        });

        // Initial game scan
        let tracked_games_clone = self.tracked_games.clone();
        let game_list_clone = self.game_list_view.clone();
        glib::spawn_future_local(async move {
            Self::refresh_games(tracked_games_clone, game_list_clone);
        });
    }

    fn refresh_games(tracked_games: Arc<Mutex<TrackedGames>>, game_list: GameListView) {
        tracing::info!("Refreshing game list");

        // Clear existing list
        game_list.clear();

        match steam::find_steam_libraries() {
            Ok(libraries) => {
                tracing::info!("Found {} Steam libraries", libraries.len());

                let mut all_games = Vec::new();

                for library in libraries {
                    match steam::scan_library(&library) {
                        Ok(games) => {
                            all_games.extend(games);
                        }
                        Err(e) => {
                            tracing::error!("Failed to scan library {:?}: {}", library, e);
                        }
                    }
                }

                // Update tracked games and UI
                if let Ok(mut tracked) = tracked_games.lock() {
                    for game in &all_games {
                        // Only add if not already tracked
                        if !tracked.games.contains_key(&game.app_id) {
                            use crate::config::GameEntry;
                            tracked.upsert_game(
                                game.app_id.clone(),
                                GameEntry {
                                    name: game.name.clone(),
                                    installed: false,
                                    optiscaler_version: None,
                                    proxy_dll: None,
                                    install_path: game.install_dir.to_string_lossy().to_string(),
                                    exe_path: game.executable_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                                    installed_date: None,
                                    last_verified: None,
                                },
                            );
                        } else {
                            // Update exe_path if it changed
                            if let Some(entry) = tracked.get_game_mut(&game.app_id) {
                                let new_exe_path = game.executable_path.as_ref().map(|p| p.to_string_lossy().to_string());
                                if entry.exe_path != new_exe_path {
                                    entry.exe_path = new_exe_path;
                                }
                            }
                        }
                    }

                    // Populate UI with games
                    for game in all_games {
                        let installed = tracked.games.get(&game.app_id)
                            .map(|e| e.installed)
                            .unwrap_or(false);
                        
                        game_list.add_game(&game.name, &game.app_id, installed);
                    }

                    if let Err(e) = tracked.save() {
                        tracing::error!("Failed to save tracked games: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to find Steam libraries: {}", e);
            }
        }
    }

    async fn handle_install(
        window: &adw::ApplicationWindow,
        details_panel: &GameDetailsPanel,
        tracked_games: Arc<Mutex<TrackedGames>>,
    ) -> anyhow::Result<(bool, String)> {
        let (app_id, game_name) = details_panel.current_game()
            .ok_or_else(|| anyhow::anyhow!("No game selected"))?;

        tracing::info!("Install button clicked for game: {}", game_name);

        // Fetch available versions in a dedicated Tokio runtime
        let releases = std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let github_client = GitHubClient::new();
                github_client.get_releases().await
            })
        })
        .join()
        .map_err(|e| anyhow::anyhow!("Thread panicked: {:?}", e))??;

        if releases.is_empty() {
            anyhow::bail!("No OptiScaler releases available");
        }

        // Get game install path and executable path from tracked games
        let (install_path, exe_path) = if let Ok(tracked) = tracked_games.lock() {
            tracked.games.get(&app_id)
                .map(|e| (PathBuf::from(&e.install_path), e.exe_path.clone().map(PathBuf::from)))
                .ok_or_else(|| anyhow::anyhow!("Game not found in tracked games"))?
        } else {
            anyhow::bail!("Failed to access tracked games");
        };

        // Show install dialog
        let dialog = InstallDialog::new(&game_name, releases);
        
        if let Some((release, proxy_dll)) = dialog.show(window).await {
            tracing::info!("Installing OptiScaler {} with proxy {}", release.tag_name, proxy_dll);
            
            // Create SteamGame object for installer
            let mut game = SteamGame::new(app_id.clone(), game_name.clone(), install_path);
            if let Some(exe) = exe_path {
                game = game.with_executable(exe);
            }
            
            // Create installer
            let installer = Installer::new(tracked_games.clone())?;
            
            // Perform installation in a separate thread with Tokio runtime
            let release_clone = release.clone();
            let proxy_dll_clone = proxy_dll.clone();
            let result = std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    installer.install(&game, &release_clone, &proxy_dll_clone, |downloaded, total| {
                        tracing::debug!("Download progress: {}/{}", downloaded, total);
                    }).await
                })
            })
            .join()
            .map_err(|e| anyhow::anyhow!("Installation thread panicked: {:?}", e))??;
            
            // Update UI
            if let Ok(tracked) = tracked_games.lock() {
                if let Some(entry) = tracked.games.get(&app_id) {
                    details_panel.update(
                        &game_name,
                        &app_id,
                        entry.installed,
                        entry.optiscaler_version.clone(),
                        entry.proxy_dll.clone(),
                        Some(entry.install_path.clone()),
                        entry.exe_path.clone(),
                    );
                }
            }
            
            return Ok((true, app_id));
        }

        Ok((false, app_id))
    }

    async fn handle_remove(
        window: &adw::ApplicationWindow,
        details_panel: &GameDetailsPanel,
        tracked_games: Arc<Mutex<TrackedGames>>,
    ) -> anyhow::Result<(bool, String)> {
        let (app_id, game_name) = details_panel.current_game()
            .ok_or_else(|| anyhow::anyhow!("No game selected"))?;

        tracing::info!("Remove button clicked for game: {}", game_name);

        // Show confirmation dialog
        let confirm_dialog = adw::AlertDialog::builder()
            .heading("Remove OptiScaler")
            .body(&format!("Are you sure you want to remove OptiScaler from {}?", game_name))
            .build();
        
        confirm_dialog.add_response("cancel", "Cancel");
        confirm_dialog.add_response("remove", "Remove");
        confirm_dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        confirm_dialog.set_default_response(Some("cancel"));
        confirm_dialog.set_close_response("cancel");

        let response = confirm_dialog.choose_future(window).await;

        if response == "remove" {
            // Get game install path and executable path from tracked games
            let (install_path, exe_path) = if let Ok(tracked) = tracked_games.lock() {
                tracked.games.get(&app_id)
                    .map(|e| (PathBuf::from(&e.install_path), e.exe_path.clone().map(PathBuf::from)))
                    .ok_or_else(|| anyhow::anyhow!("Game not found in tracked games"))?
            } else {
                anyhow::bail!("Failed to access tracked games");
            };
            
            // Create SteamGame object for installer
            let mut game = SteamGame::new(app_id.clone(), game_name.clone(), install_path);
            if let Some(exe) = exe_path {
                game = game.with_executable(exe);
            }
            
            // Create installer
            let installer = Installer::new(tracked_games.clone())?;
            
            // Perform removal
            installer.remove(&game)?;
            
            // Update UI
            if let Ok(tracked) = tracked_games.lock() {
                if let Some(entry) = tracked.games.get(&app_id) {
                    details_panel.update(
                        &game_name,
                        &app_id,
                        entry.installed,
                        entry.optiscaler_version.clone(),
                        entry.proxy_dll.clone(),
                        Some(entry.install_path.clone()),
                        entry.exe_path.clone(),
                    );
                }
            }
            
            return Ok((true, app_id));
        }

        Ok((false, app_id))
    }

    pub fn present(&self) {
        self.window.present();
    }
}
