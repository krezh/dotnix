use super::{Cited, raise, site, split};
use crate::markdown;

#[test]
fn a_trailing_list_of_sources_leaves_the_text() {
    let reply = "It needs 30 Blood.[1]\n\nSources:\n- [Guide (Leprestore)](https://www.leprestore.com/a)\n- [Forum](https://forums.blizzard.com/t/1)";
    let (body, cited) = split(reply);
    assert_eq!(body, "It needs 30 Blood.[1]");
    assert_eq!(
        cited,
        vec![
            Cited {
                site: "leprestore.com".to_owned(),
                url: "https://www.leprestore.com/a".to_owned(),
            },
            Cited {
                site: "forums.blizzard.com".to_owned(),
                url: "https://forums.blizzard.com/t/1".to_owned(),
            },
        ]
    );
}

#[test]
fn sources_written_as_numbered_addresses_leave_too() {
    let reply = "Fixed.[1][2]\n\nSources:\n[1] https://news.blizzard.com/a\n[2] <https://www.wowhead.com/b>.";
    let (body, cited) = split(reply);
    assert_eq!(body, "Fixed.[1][2]");
    let sites: Vec<&str> = cited.iter().map(|cited| cited.site.as_str()).collect();
    assert_eq!(sites, vec!["news.blizzard.com", "wowhead.com"]);
    assert_eq!(cited[1].url, "https://www.wowhead.com/b");
}

#[test]
fn numbers_side_by_side_stay_apart() {
    let (text, raised) = raise(markdown::parse("Fixed [1][2]."));
    assert_eq!(text.text, "Fixed¹\u{2009}².");
    assert_eq!(raised.len(), 2);
    assert_eq!(&text.text[raised[1].0.clone()], "²");
}

#[test]
fn sources_arriving_are_hidden_at_once() {
    let (body, cited) = split("Done.\n\n**Sources:**\n- [Gui");
    assert_eq!(body, "Done.");
    assert!(cited.is_empty());
}

#[test]
fn a_sources_heading_with_text_after_stays() {
    let reply = "Sources:\nI could not find any.";
    assert_eq!(split(reply), (reply, Vec::new()));
}

#[test]
fn sites_drop_the_scheme_and_www() {
    assert_eq!(site("https://www.nixos.org/blog?x=1"), "nixos.org");
}

#[test]
fn markers_rise_outside_code_and_links() {
    let parsed = markdown::parse("**Quests:** a chain [1]. See `a[2]` and [3](https://x.org).[12]");
    let (text, raised) = raise(parsed);
    assert_eq!(text.text, "Quests: a chain¹. See a[2] and 3.¹²");
    let numbers: Vec<usize> = raised.iter().map(|(_, number)| *number).collect();
    assert_eq!(numbers, vec![1, 12]);
    assert_eq!(&text.text[raised[0].0.clone()], "¹");
    let code = text
        .spans
        .iter()
        .find(|(_, style)| style.code)
        .map(|(range, _)| &text.text[range.clone()]);
    assert_eq!(code, Some("a[2]"));
    let bold = text
        .spans
        .iter()
        .find(|(_, style)| style.bold)
        .map(|(range, _)| &text.text[range.clone()]);
    assert_eq!(bold, Some("Quests:"));
}
