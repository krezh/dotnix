use gtk::prelude::*;

use crate::changelog::{ChangelogOutput, markdown_to_text};
use crate::{clear, label};

pub(crate) fn render(content: &gtk::Box, changelog: &ChangelogOutput) {
    clear(content);

    let metadata = gtk::Box::new(gtk::Orientation::Vertical, 8);
    metadata.add_css_class("changelog-metadata");
    let identity = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    identity.append(&label(&changelog.pname, &["changelog-package"], 0.0));
    identity.append(&label(&changelog.version, &["changelog-version"], 0.0));
    metadata.append(&identity);
    if let Some(description) = changelog.description.as_deref() {
        let description = label(description, &["changelog-description"], 0.0);
        description.set_ellipsize(gtk::pango::EllipsizeMode::None);
        description.set_wrap(true);
        description.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        metadata.append(&description);
    }
    content.append(&metadata);

    if changelog.releases.is_empty() {
        content.append(&label(
            "No release notes found for this version.",
            &["no-changes"],
            0.0,
        ));
        return;
    }

    for release in &changelog.releases {
        let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
        section.add_css_class("release-section");
        section.append(&label(&release.tag, &["release-tag"], 0.0));
        let notes = label(&markdown_to_text(&release.body), &["changelog-body"], 0.0);
        notes.set_ellipsize(gtk::pango::EllipsizeMode::None);
        notes.set_selectable(true);
        notes.set_wrap(true);
        notes.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        section.append(&notes);
        content.append(&section);
    }
}
