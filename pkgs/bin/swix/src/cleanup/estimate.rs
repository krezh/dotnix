use std::time::{SystemTime, UNIX_EPOCH};

use crate::cache;

const CLEANUP_ESTIMATE: &str = "cleanup-estimate";

/// Reclaimable bytes found by the last completed cleanup scan and when it finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CleanupEstimate {
    pub(crate) bytes: u64,
    /// Unix seconds.
    pub(crate) scanned_at: i64,
}

impl CleanupEstimate {
    pub(crate) fn load() -> Option<Self> {
        Self::parse(&cache::read(CLEANUP_ESTIMATE)?)
    }

    pub(crate) fn save(bytes: u64) {
        let scanned_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs() as i64);
        cache::write(CLEANUP_ESTIMATE, &format!("{bytes} {scanned_at}\n"));
    }

    /// Forgets the estimate once a cleanup has run, since it no longer describes the disk.
    pub(crate) fn clear() {
        cache::remove(CLEANUP_ESTIMATE);
    }

    fn parse(contents: &str) -> Option<Self> {
        let mut fields = contents.split_whitespace();
        let estimate = Self {
            bytes: fields.next()?.parse().ok()?,
            scanned_at: fields.next()?.parse().ok()?,
        };
        fields.next().is_none().then_some(estimate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_estimate_parses_only_well_formed_entries() {
        assert_eq!(
            CleanupEstimate::parse("9448928460 1791820512\n"),
            Some(CleanupEstimate {
                bytes: 9_448_928_460,
                scanned_at: 1_791_820_512,
            })
        );
        assert_eq!(CleanupEstimate::parse(""), None);
        assert_eq!(CleanupEstimate::parse("12"), None);
        assert_eq!(CleanupEstimate::parse("x 1"), None);
        assert_eq!(CleanupEstimate::parse("1 2 3"), None);
    }
}
