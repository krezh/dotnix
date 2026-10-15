use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::Global;

use crate::{features::GlyphSpec, icons};

const GLYPHS: &[GlyphSpec] = &[("brightness", &["display-brightness-symbolic"])];

/// Every icon island and its features draw, looked up once at startup; cheap to clone. Empty until
/// the lookup finishes; islands observe it to redraw once it has.
#[derive(Clone, Debug, Default)]
pub struct Glyphs(Arc<HashMap<&'static str, PathBuf>>);

impl Global for Glyphs {}

impl Glyphs {
    /// Looks up island's own icons and the features' `extra` ones.
    pub fn resolve(extra: &[GlyphSpec]) -> Self {
        Self(Arc::new(
            GLYPHS
                .iter()
                .chain(extra)
                .filter_map(|(key, names)| Some((*key, names.iter().find_map(|name| icons::lookup(name))?)))
                .collect(),
        ))
    }

    pub fn get(&self, key: &str) -> Option<&Path> {
        self.0.get(key).map(PathBuf::as_path)
    }
}
