use gtk4::prelude::*;

#[derive(Clone)]
pub struct GameListView {
    scrolled_window: gtk4::ScrolledWindow,
    list_box: gtk4::ListBox,
}

impl GameListView {
    pub fn new() -> Self {
        let list_box = gtk4::ListBox::new();
        list_box.add_css_class("navigation-sidebar");
        list_box.set_selection_mode(gtk4::SelectionMode::Single);

        let scrolled_window = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&list_box)
            .vexpand(true)
            .build();

        Self {
            scrolled_window,
            list_box,
        }
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.scrolled_window.clone().upcast()
    }

    pub fn add_game(&self, name: &str, app_id: &str, installed: bool) {
        let row = gtk4::ListBoxRow::new();
        
        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);

        // Status indicator
        let status_icon_str = if installed { "●" } else { "○" };
        let status_label = gtk4::Label::new(Some(status_icon_str));
        if installed {
            status_label.add_css_class("success");
        } else {
            status_label.add_css_class("dim-label");
        }

        // Game info
        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        
        let name_label = gtk4::Label::new(Some(name));
        name_label.set_halign(gtk4::Align::Start);
        name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        name_label.set_widget_name("game_name");
        
        let id_label = gtk4::Label::new(Some(&format!("AppID: {}", app_id)));
        id_label.set_halign(gtk4::Align::Start);
        id_label.add_css_class("dim-label");
        id_label.add_css_class("caption");
        id_label.set_widget_name("app_id");

        vbox.append(&name_label);
        vbox.append(&id_label);

        hbox.append(&status_label);
        hbox.append(&vbox);
        hbox.set_widget_name(if installed { "installed" } else { "not_installed" });

        row.set_child(Some(&hbox));
        self.list_box.append(&row);
    }
    
    pub fn connect_row_selected<F>(&self, callback: F)
    where
        F: Fn(Option<(String, String, bool)>) + 'static,
    {
        self.list_box.connect_row_selected(move |_, row| {
            let game_data = row.and_then(|r| {
                let child = r.child()?;
                let hbox = child.clone().downcast::<gtk4::Box>().ok()?;
                
                let installed = hbox.widget_name() == "installed";
                
                let mut first_child = hbox.first_child();
                while let Some(child) = first_child.clone() {
                    if let Ok(vbox) = child.clone().downcast::<gtk4::Box>() {
                        let mut name = None;
                        let mut app_id = None;
                        
                        let mut vbox_child = vbox.first_child();
                        while let Some(label_widget) = vbox_child.clone() {
                            if let Ok(label) = label_widget.clone().downcast::<gtk4::Label>() {
                                if label.widget_name() == "game_name" {
                                    name = Some(label.text().to_string());
                                } else if label.widget_name() == "app_id" {
                                    let text = label.text();
                                    app_id = text.strip_prefix("AppID: ").map(|s| s.to_string());
                                }
                            }
                            vbox_child = label_widget.next_sibling();
                        }
                        
                        if let (Some(n), Some(a)) = (name, app_id) {
                            return Some((n, a, installed));
                        }
                    }
                    first_child = child.next_sibling();
                }
                
                None
            });
            
            callback(game_data);
        });
    }

    pub fn update_game_status(&self, app_id: &str, installed: bool) {
        // Find the row with matching app_id
        let mut child = self.list_box.first_child();
        while let Some(row_widget) = child.clone() {
            if let Ok(row) = row_widget.clone().downcast::<gtk4::ListBoxRow>() {
                if let Some(row_child) = row.child() {
                    if let Ok(hbox) = row_child.downcast::<gtk4::Box>() {
                        // Check if this is the right row by finding the app_id label
                        let mut hbox_child = hbox.first_child();
                        let mut found_match = false;
                        
                        while let Some(child) = hbox_child.clone() {
                            if let Ok(vbox) = child.clone().downcast::<gtk4::Box>() {
                                let mut vbox_child = vbox.first_child();
                                while let Some(label_widget) = vbox_child.clone() {
                                    if let Ok(label) = label_widget.clone().downcast::<gtk4::Label>() {
                                        if label.widget_name() == "app_id" {
                                            let text = label.text();
                                            if let Some(id) = text.strip_prefix("AppID: ") {
                                                if id == app_id {
                                                    found_match = true;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    vbox_child = label_widget.next_sibling();
                                }
                            }
                            hbox_child = child.next_sibling();
                        }
                        
                        if found_match {
                            // Update the status indicator
                            let mut hbox_child = hbox.first_child();
                            while let Some(child) = hbox_child.clone() {
                                if let Ok(status_label) = child.clone().downcast::<gtk4::Label>() {
                                    // This is the status indicator (first label child)
                                    status_label.set_text(if installed { "●" } else { "○" });
                                    status_label.remove_css_class("success");
                                    status_label.remove_css_class("dim-label");
                                    if installed {
                                        status_label.add_css_class("success");
                                    } else {
                                        status_label.add_css_class("dim-label");
                                    }
                                    
                                    // Update hbox widget name
                                    hbox.set_widget_name(if installed { "installed" } else { "not_installed" });
                                    return;
                                }
                                hbox_child = child.next_sibling();
                            }
                        }
                    }
                }
            }
            child = row_widget.next_sibling();
        }
    }

    pub fn clear(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
    }
}
