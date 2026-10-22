use super::{Control, parse};

fn args(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_owned).collect()
}

/// The mistake for `words`, with its hints, as plain text.
fn mistake(words: &str) -> String {
    let mistake = parse(&args(words)).unwrap_err();
    crate::cli::render(&mistake.lines(), false)
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
    assert_eq!(parse(&args("toggle system")), Ok(Control::Toggle("system".to_owned(), None)));
}

#[test]
fn an_unknown_view_is_named_with_the_ones_there_are() {
    let error = mistake("open ai histry");
    assert!(error.contains("no view histry"), "{error}");
    assert!(error.contains("Views: sessions chat usage"), "{error}");
    let error = mistake("open calendar week");
    assert!(error.contains("no views"), "{error}");
}

#[test]
fn feature_commands_go_to_their_feature_as_typed() {
    assert_eq!(
        parse(&args("volume up 10")),
        Ok(Control::Feature("audio", args("volume up 10")))
    );
    assert_eq!(
        parse(&args("dnd")),
        Ok(Control::Feature("notifications", args("dnd")))
    );
    assert_eq!(
        parse(&args("session lock")),
        Ok(Control::Feature("session", args("session lock")))
    );
}

#[test]
fn mistakes_say_what_is_wrong_and_suggest_a_fix() {
    let error = mistake("volum up");
    assert!(error.contains("No command volum"), "{error}");
    assert!(error.contains("Did you mean volume?"), "{error}");

    let error = mistake("volume set loud");
    assert!(error.contains("loud is not a percentage"), "{error}");

    let error = mistake("volume set");
    assert!(error.contains("volume set needs a percentage"), "{error}");

    let error = mistake("session");
    assert!(error.contains("session needs one of lock"), "{error}");

    let error = mistake("clear everything");
    assert!(error.contains("Nothing goes after clear, got everything"), "{error}");

    let error = mistake("open laucher");
    assert!(error.contains("No panel laucher"), "{error}");
    assert!(error.contains("Did you mean launcher?"), "{error}");
}
