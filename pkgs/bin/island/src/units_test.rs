use super::{ago, bytes, link, plural, rate, running, span};

#[test]
fn what_is_under_way_counts_its_first_minute_in_seconds() {
    assert_eq!(running(0), "0s");
    assert_eq!(running(59), "59s");
    assert_eq!(running(65), "1m");
    assert_eq!(running(3700), "1h 1m");
}

#[test]
fn links_read_in_the_largest_whole_unit() {
    assert_eq!(link(866), "866 Mb/s");
    assert_eq!(link(1000), "1 Gb/s");
    assert_eq!(link(2500), "2.5 Gb/s");
}

#[test]
fn sizes_keep_one_decimal_only_under_ten() {
    assert_eq!(bytes(0), "0 B");
    assert_eq!(bytes(1023), "1023 B");
    assert_eq!(bytes(1024), "1.0 kB");
    assert_eq!(bytes(4300), "4.2 kB");
    assert_eq!(bytes(12 * 1024 * 1024), "12 MB");
    assert_eq!(bytes(32 * 1024 * 1024 * 1024), "32 GB");
    assert_eq!(bytes(1536 * 1024 * 1024), "1.5 GB");
    assert_eq!(bytes(5000 * 1024 * 1024 * 1024 * 1024), "5000 TB");
}

#[test]
fn rates_read_as_sizes_each_second() {
    assert_eq!(rate(512.0), "512 B/s");
    assert_eq!(rate(1.5 * 1024.0 * 1024.0), "1.5 MB/s");
    assert_eq!(rate(-1.0), "0 B/s");
}

#[test]
fn spans_show_the_two_largest_units() {
    assert_eq!(span(30), "<1m");
    assert_eq!(span(240), "4m");
    assert_eq!(span(7500), "2h 5m");
    assert_eq!(span(3600), "1h 0m");
    assert_eq!(span(3 * 86_400 + 4 * 3600 + 59), "3d 4h");
}

#[test]
fn ago_counts_back_in_the_largest_unit() {
    assert_eq!(ago(-5), "just now");
    assert_eq!(ago(59), "just now");
    assert_eq!(ago(240), "4m ago");
    assert_eq!(ago(7500), "2h ago");
    assert_eq!(ago(3 * 86_400), "3d ago");
}

#[test]
fn plural_picks_the_word_for_the_count() {
    assert_eq!(plural(1_u32, "file", "files"), "1 file");
    assert_eq!(plural(0_i64, "file", "files"), "0 files");
    assert_eq!(plural(2_usize, "file", "files"), "2 files");
}
