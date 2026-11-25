use molt_core::units::Units;

use super::IslandUnits;

#[test]
fn ages_keep_their_largest_unit() {
    assert_eq!(IslandUnits.age(4 * 86_400 + 3 * 3600), "4d");
    assert_eq!(IslandUnits.age(7 * 3600 + 120), "7h");
    assert_eq!(IslandUnits.age(30), "<1m");
}

#[test]
fn spans_and_sizes_follow_island() {
    assert_eq!(IslandUnits.span(34 * 60), "34m");
    assert!(IslandUnits.size(22 * 1024 * 1024 * 1024).ends_with(" GB"));
}
