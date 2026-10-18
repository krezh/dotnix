use std::sync::Arc;

use super::entry::Entry;

#[derive(Clone, Default)]
pub struct State {
    pub(super) entries: Arc<Vec<Entry>>,
    pub(super) error: Option<String>,
    /// Counts every change, so comparing states does not compare every entry.
    pub(super) version: u64,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl State {
    pub(super) fn matching(&self, query: &str) -> Vec<&Entry> {
        let query = query.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| entry.matches(&query))
            .collect()
    }
}
