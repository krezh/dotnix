use super::{Control, parse};

fn args(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn a_panel_opens_on_one_of_its_views() {
    assert_eq!(
        parse(&args("open ai chat")),
        Ok(Control::Open("ai".to_owned(), Some("chat".to_owned())))
    );
    assert_eq!(
        parse(&args("toggle ai usage")),
        Ok(Control::Toggle("ai".to_owned(), Some("usage".to_owned())))
    );
    assert_eq!(
        parse(&args("open ai")),
        Ok(Control::Open("ai".to_owned(), None))
    );
}

#[test]
fn an_unknown_view_is_named_with_the_ones_there_are() {
    let error = parse(&args("open ai history")).unwrap_err();
    assert!(error.contains("sessions, chat, usage"), "{error}");
    let error = parse(&args("open calendar week")).unwrap_err();
    assert!(error.contains("no views"), "{error}");
}
