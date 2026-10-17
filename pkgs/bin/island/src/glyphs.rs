use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::Global;

use crate::{features::GlyphSpec, icons};

/// Every icon island and its features draw, looked up once at startup; cheap to clone. Empty until
/// the lookup finishes; islands observe it to redraw once it has.
#[derive(Clone, Debug, Default)]
pub struct Glyphs(Arc<HashMap<&'static str, PathBuf>>);

impl Global for Glyphs {}

impl Glyphs {
    /// Looks up the features' icons, `specs`.
    pub fn resolve(specs: &[GlyphSpec]) -> Self {
        Self(Arc::new(
            specs
                .iter()
                .filter_map(|(key, names)| Some((*key, names.iter().find_map(|name| icons::lookup(name))?)))
                .collect(),
        ))
    }

    pub fn get(&self, key: &str) -> Option<&Path> {
        self.0.get(key).map(PathBuf::as_path)
    }
}
