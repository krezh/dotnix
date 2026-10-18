use super::parse_exec;

#[test]
fn splits_plain_words_and_drops_field_codes() {
    assert_eq!(
        parse_exec("firefox %U", "Firefox"),
        Some(vec!["firefox".to_owned()])
    );
    assert_eq!(
        parse_exec("code --new-window %F", "Code"),
        Some(vec!["code".to_owned(), "--new-window".to_owned()])
    );
}

#[test]
fn keeps_quoted_arguments_together() {
    assert_eq!(
        parse_exec(r#"sh -c "echo \"hi there\" > \$HOME/x""#, "Sh"),
        Some(vec![
            "sh".to_owned(),
            "-c".to_owned(),
            r#"echo "hi there" > $HOME/x"#.to_owned()
        ])
    );
}

#[test]
fn expands_percent_and_app_name() {
    assert_eq!(
        parse_exec("app --rate 100%% --title %c", "My App"),
        Some(vec![
            "app".to_owned(),
            "--rate".to_owned(),
            "100%".to_owned(),
            "--title".to_owned(),
            "My App".to_owned()
        ])
    );
}

#[test]
fn rejects_unterminated_quotes_and_empty_lines() {
    assert_eq!(parse_exec(r#"app "broken"#, "App"), None);
    assert_eq!(parse_exec("  %U ", "App"), None);
}
