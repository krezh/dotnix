use super::{Block, Header, PanelView};
use crate::components::text::{Text, Tone};

fn view(blocks: Vec<Block>) -> PanelView {
    PanelView {
        header: Header::None,
        blocks,
        scroll: false,
    }
}

fn caption(words: &str) -> Block {
    Block::Text(Text::new(Tone::Caption(0), words))
}

#[test]
fn longer_wrapping_words_are_measured_again() {
    let short = view(vec![caption("Dim only")]);
    let long = view(vec![caption(
        "I start the new build next to your running island and take screenshots as it runs",
    )]);
    assert_ne!(short.key(), long.key());
}

#[test]
fn a_single_line_keeps_its_shape_as_its_words_change() {
    let held = |words: &str| {
        let mut text = Text::new(Tone::Body, words);
        text.lines = Some(1);
        view(vec![Block::Text(text)])
    };
    assert_eq!(held("4s").key(), held("5s").key());
    assert_eq!(
        view(vec![Block::note("Up 3h")]).key(),
        view(vec![Block::note("Up 4h")]).key()
    );
}
