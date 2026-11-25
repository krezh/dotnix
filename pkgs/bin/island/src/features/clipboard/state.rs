use std::sync::Arc;

use super::{
    entry::Entry,
    filter::{FILTERS, Filter},
    picker::Layout,
};

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

/// What the picker lists, the pinned first, each group newest first.
pub(super) struct Matching<'a> {
    pub(super) entries: Vec<&'a Entry>,
    pub(super) layout: Layout,
}

impl State {
    pub(super) fn matching(&self, query: &str, filter: Filter) -> Matching<'_> {
        let query = query.to_lowercase();
        let (pinned, rest): (Vec<&Entry>, Vec<&Entry>) = self
            .entries
            .iter()
            .filter(|entry| filter.admits(entry) && entry.matches(&query))
            .partition(|entry| entry.pinned);
        let layout = Layout {
            count: pinned.len() + rest.len(),
            pinned: pinned.len(),
        };
        let mut entries = pinned;
        entries.extend(rest);
        Matching { entries, layout }
    }

    /// How many each filter keeps of what matches `query`, in the order of [`FILTERS`].
    pub(super) fn counts(&self, query: &str) -> [usize; FILTERS.len()] {
        let query = query.to_lowercase();
        let mut counts = [0; FILTERS.len()];
        for entry in self.entries.iter().filter(|entry| entry.matches(&query)) {
            for (count, filter) in counts.iter_mut().zip(FILTERS) {
                if filter.admits(entry) {
                    *count += 1;
                }
            }
        }
        counts
    }
}
