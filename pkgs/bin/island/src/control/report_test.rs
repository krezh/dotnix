use super::*;

#[test]
fn json_names_each_row_by_its_label() {
    let report = Report::shows(vec![
        Row::new("Do not disturb", Shown::Switch(false)),
        Row::new("Waiting", Shown::Count(3)),
        Row::new(
            "Volume",
            Shown::Level {
                value: 0.691,
                muted: false,
            },
        )
        .detail("Speakers"),
        Row::new("Brightness", Shown::Unknown),
    ]);
    assert_eq!(
        report.json().to_string(),
        r#"{"do_not_disturb":false,"waiting":3,"volume":{"value":0.69,"muted":false,"device":"Speakers"},"brightness":null}"#
    );
}
