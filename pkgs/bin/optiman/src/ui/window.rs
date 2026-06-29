use gtk4::prelude::*;
use gtk4::glib;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::optiscaler::{GitHubClient, Manager, Release};

pub struct MainWindow {
    window: adw::ApplicationWindow,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("OptiMan")
            .default_width(480)
            .default_height(320)
            .build();

        let instance = Self { window };
        instance.build_ui();
        instance
    }

    fn build_ui(&self) {
        let toast_overlay = adw::ToastOverlay::new();

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());

        let page = adw::PreferencesPage::new();

        // --- Status group ---
        let status_group = adw::PreferencesGroup::new();
        status_group.set_title("Installed");

        let version_label = gtk4::Label::new(Some(
            &Manager::current_version()
                .unwrap_or_else(|| "Not installed".to_string()),
        ));
        version_label.add_css_class("dim-label");
        version_label.set_halign(gtk4::Align::Start);

        let status_row = adw::ActionRow::builder()
            .title("Version")
            .build();
        status_row.add_suffix(&version_label);

        status_group.add(&status_row);
        page.add(&status_group);

        // --- Update group ---
        let update_group = adw::PreferencesGroup::new();
        update_group.set_title("Update");

        let version_model = gtk4::StringList::new(&[]);
        let combo = adw::ComboRow::builder()
            .title("Release")
            .model(&version_model)
            .build();

        let download_btn = gtk4::Button::builder()
            .label("Download")
            .css_classes(vec!["suggested-action", "pill"])
            .sensitive(false)
            .halign(gtk4::Align::End)
            .build();

        let download_row = adw::ActionRow::new();
        download_row.add_suffix(&download_btn);

        update_group.add(&combo);
        update_group.add(&download_row);
        page.add(&update_group);

        toolbar_view.set_content(Some(&page));
        toast_overlay.set_child(Some(&toolbar_view));
        self.window.set_content(Some(&toast_overlay));

        // Load releases async
        let combo_clone = combo.clone();
        let btn_clone = download_btn.clone();
        let toast_clone = toast_overlay.clone();
        glib::spawn_future_local(async move {
            let releases = match std::thread::spawn(|| {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(GitHubClient::new().get_releases())
            })
            .join()
            {
                Ok(Ok(r)) if !r.is_empty() => r,
                Ok(Ok(_)) => {
                    toast_clone.add_toast(adw::Toast::new("No releases found"));
                    return;
                }
                Ok(Err(e)) => {
                    tracing::error!("Failed to fetch releases: {}", e);
                    let msg = if e.to_string().contains("rate limit") {
                        "GitHub rate limit hit — set GITHUB_TOKEN env var".to_string()
                    } else {
                        format!("Error: {}", e)
                    };
                    toast_clone.add_toast(adw::Toast::builder().title(&msg).timeout(10).build());
                    return;
                }
                Err(_) => {
                    toast_clone.add_toast(adw::Toast::new("Failed to fetch releases"));
                    return;
                }
            };

            for r in &releases {
                version_model.append(&r.tag_name);
            }
            combo_clone.set_selected(0);
            btn_clone.set_sensitive(true);

            // Wire download button
            btn_clone.connect_clicked(move |btn| {
                let idx = combo_clone.selected() as usize;
                let Some(release) = releases.get(idx).cloned() else { return };

                btn.set_sensitive(false);
                btn.set_label("Downloading…");

                let btn2 = btn.clone();
                let toast2 = toast_clone.clone();
                let version_label2 = version_label.clone();
                let version_str = release.tag_name.clone();
                glib::spawn_future_local(async move {
                    let result = std::thread::spawn(move || {
                        tokio::runtime::Runtime::new().unwrap().block_on(async {
                            Manager::new().install(&release, |_, _| {}).await
                        })
                    })
                    .join()
                    .map_err(|_| anyhow::anyhow!("thread panic"))
                    .and_then(|r| r);

                    match result {
                        Ok(()) => {
                            let toast = adw::Toast::new(
                                &format!("Installed OptiScaler {}", version_str)
                            );
                            toast2.add_toast(toast);
                            version_label2.set_text(&version_str);
                        }
                        Err(e) => {
                            let toast = adw::Toast::new(&format!("Failed: {}", e));
                            toast2.add_toast(toast);
                        }
                    }

                    btn2.set_sensitive(true);
                    btn2.set_label("Download");
                });
            });
        });
    }

    pub fn present(&self) {
        self.window.present();
    }
}
