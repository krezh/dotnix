use super::*;

#[test]
fn percent_reads_full_only_once_spent() {
    assert_eq!(percent(99.6), "99%");
    assert_eq!(percent(100.0), "100%");
    assert_eq!(percent(0.96), "0.9%");
    assert_eq!(percent(0.0), "0%");
}
