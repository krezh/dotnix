use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::optiscaler::Release;

pub struct InstallDialog {
    dialog: adw::AlertDialog,
    version_row: adw::ComboRow,
    proxy_row: adw::ComboRow,
    versions: Vec<Release>,
}

impl InstallDialog {
    /// Creates a new install dialog.
    pub fn new(game_name: &str, versions: Vec<Release>) -> Self {
        let dialog = adw::AlertDialog::builder()
            .heading(&format!("Install OptiScaler to {}", game_name))
            .build();

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("install", "Install");
        dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("install"));
        dialog.set_close_response("cancel");

        let preferences_page = adw::PreferencesPage::new();
        preferences_page.set_vexpand(false);

        // Version selection group
        let version_group = adw::PreferencesGroup::new();
        version_group.set_title("Version");
        version_group.set_description(Some("Select the OptiScaler version to install"));

        let version_model = gtk4::StringList::new(&[]);
        for release in &versions {
            version_model.append(&release.tag_name);
        }

        let version_row = adw::ComboRow::builder()
            .title("Release Version")
            .model(&version_model)
            .build();

        if !versions.is_empty() {
            version_row.set_selected(0);
        }

        version_group.add(&version_row);

        // Proxy DLL selection group
        let proxy_group = adw::PreferencesGroup::new();
        proxy_group.set_title("Proxy DLL");
        proxy_group.set_description(Some("Select which DLL to use as proxy (dxgi.dll is recommended for most games)"));

        let proxy_options = ["dxgi.dll", "d3d11.dll", "d3d12.dll", "winmm.dll"];
        let proxy_model = gtk4::StringList::new(&proxy_options);

        let proxy_row = adw::ComboRow::builder()
            .title("Proxy DLL")
            .model(&proxy_model)
            .build();

        proxy_row.set_selected(0);

        proxy_group.add(&proxy_row);

        preferences_page.add(&version_group);
        preferences_page.add(&proxy_group);

        dialog.set_extra_child(Some(&preferences_page));

        Self {
            dialog,
            version_row,
            proxy_row,
            versions,
        }
    }

    /// Shows the dialog and returns the selected version and proxy DLL.
    pub async fn show<W: IsA<gtk4::Widget>>(self, parent: &W) -> Option<(Release, String)> {
        let response = self.dialog.choose_future(parent).await;

        if response == "install" {
            let version_index = self.version_row.selected() as usize;
            let proxy_index = self.proxy_row.selected() as usize;

            let proxy_options = ["dxgi.dll", "d3d11.dll", "d3d12.dll", "winmm.dll"];

            if let (Some(release), Some(&proxy)) = (
                self.versions.get(version_index),
                proxy_options.get(proxy_index),
            ) {
                return Some((release.clone(), proxy.to_string()));
            }
        }

        None
    }
}
