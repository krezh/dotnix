use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::Global;

use crate::{features::GlyphSpec, icons};

/// Empty until the lookup finishes; islands observe it to redraw once it has.
#[derive(Clone, Debug, Default)]
pub struct Glyphs(Arc<HashMap<&'static str, PathBuf>>);

impl Global for Glyphs {}

/// Lucide's icons, which island ships for its own interface, set at build time.
const BUNDLED: Option<&str> = option_env!("ISLAND_GLYPHS");

/// One of the icons island ships, by its name.
fn bundled(name: &str) -> Option<PathBuf> {
    let path = Path::new(BUNDLED?).join(format!("{name}.svg"));
    path.exists().then_some(path)
}

impl Glyphs {
    /// Each name is one of the icons island ships, or else one from the icon theme, such as a
    /// provider's logo.
    pub fn resolve(specs: &[GlyphSpec]) -> Self {
        Self(Arc::new(
            specs
                .iter()
                .filter_map(|(key, names)| {
                    let path = names
                        .iter()
                        .find_map(|name| bundled(name).or_else(|| icons::lookup(name)))?;
                    Some((*key, path))
                })
                .collect(),
        ))
    }

    pub fn get(&self, key: &str) -> Option<&Path> {
        self.0.get(key).map(PathBuf::as_path)
    }
}
