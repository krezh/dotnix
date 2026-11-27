use super::{Class, color, hsl, site};
use crate::features::clipboard::entry::Kind;

#[test]
fn reads_one_address_as_a_link() {
    assert_eq!(
        site("https://github.com/catppuccin/catppuccin"),
        Some("github.com")
    );
    assert_eq!(
        site(" https://me@example.org:8443/a?b#c\n"),
        Some("example.org")
    );
    assert_eq!(site("see https://example.org"), None);
    assert_eq!(site("github.com/catppuccin"), None);
    assert_eq!(
        Class::of(Kind::Text, "https://wiki.hypr.land/Configuring/Variables/"),
        Class::Link
    );
}

#[test]
fn reads_colors() {
    assert_eq!(color("#89b4fa"), Some(0x89b4fa));
    assert_eq!(color("#fff"), Some(0xffffff));
    assert_eq!(color("rgb(137, 180, 250)"), Some(0x89b4fa));
    assert_eq!(color("rgb(137 180 250)"), Some(0x89b4fa));
    assert_eq!(color("#89b4f"), None);
    assert_eq!(color("rgb(300, 0, 0)"), None);
    assert_eq!(Class::of(Kind::Text, "#1e1e2e"), Class::Color);
}

#[test]
fn reads_commands_and_source_as_code() {
    assert_eq!(
        Class::of(Kind::Text, "nix build .#island --print-build-logs"),
        Class::Code
    );
    assert_eq!(Class::of(Kind::Text, "$ git status"), Class::Code);
    assert_eq!(
        Class::of(
            Kind::Text,
            "fn matching(&self) -> usize {\n    let a = 1;\n    a\n}"
        ),
        Class::Code
    );
    assert_eq!(Class::of(Kind::Text, "git"), Class::Text);
    assert_eq!(
        Class::of(
            Kind::Text,
            "The meeting moved to Thursday at 14:00.\nBring the notes from last week."
        ),
        Class::Text
    );
    assert_eq!(Class::of(Kind::Text, "681617"), Class::Text);
}

#[test]
fn keeps_pictures_and_files() {
    assert_eq!(Class::of(Kind::Image, ""), Class::Picture);
    assert_eq!(Class::of(Kind::Files, "/tmp/a.png"), Class::Files);
}

#[test]
fn converts_to_hsl() {
    assert_eq!(hsl(0x89b4fa), (217, 92, 76));
    assert_eq!(hsl(0xffffff), (0, 0, 100));
}
