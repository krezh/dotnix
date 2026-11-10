use super::{crt, folded, lift_to_middle};
use crate::gpu::renderer::Sprite;

#[test]
fn switching_off_starts_whole_and_ends_black() {
    let start = crt(0.0);
    assert_eq!(start.fold, 0.0);
    assert_eq!(start.island, 1.0);
    assert_eq!(start.line.2, 0.0);
    assert_eq!(start.black, 0.0);
    let end = crt(crate::stage::SWITCH_OFF.as_secs_f32());
    assert_eq!(end.island, 0.0);
    assert_eq!(end.line.2, 0.0);
    assert_eq!(end.flare.1, 0.0);
    assert_eq!(end.black, 1.0);
}

#[test]
fn the_line_shows_once_folded_then_pinches_to_a_dot() {
    let folded = crt(0.26);
    assert_eq!(folded.fold, 1.0);
    assert!(folded.line.2 > 0.5);
    let pinched = crt(0.7);
    assert!(pinched.line.0 < 0.1, "{pinched:?}");
}

#[test]
fn the_flare_rises_then_fades() {
    assert_eq!(crt(0.5).flare.1, 0.0);
    let rising = crt(0.70).flare.1;
    let peak = crt(0.756).flare.1;
    let fading = crt(1.0).flare.1;
    assert!(rising < peak && fading < peak, "{rising} {peak} {fading}");
}

#[test]
fn folding_squashes_about_the_middle_and_whitens() {
    let sprite = Sprite::shape([0.0, 0.0, 100.0, 40.0], [0.0, 0.0, 0.0, 0.5], 10.0, 2.0);
    let done = folded(sprite, (50.0, 20.0), (1.0, 0.5), 1.0);
    assert_eq!(done.rect, [0.0, 10.0, 100.0, 20.0]);
    assert_eq!(done.color, [1.0; 4]);
    assert_eq!(done.params[0], 5.0);
}

#[test]
fn moves_a_panel_to_the_middle_of_the_screen() {
    assert_eq!(lift_to_middle(1440.0, 400.0, 10.0), 510.0);
    assert_eq!(lift_to_middle(300.0, 400.0, 10.0), 0.0);
}
