use super::*;

fn at(x: f32, y: f32) -> Sprite {
    Sprite::shape([x, y, 10.0, 10.0], [1.0; 4], 0.0, 2.0)
}

#[test]
fn a_pull_moves_only_what_is_in_the_area() {
    let mut sprites = [at(20.0, 20.0), at(20.0, 200.0), at(200.0, 20.0)];
    pulled_along(&mut sprites, [0.0, 0.0, 100.0, 100.0], 12.0);
    assert_eq!(sprites[0].rect[1], 32.0);
    assert_eq!(sprites[1].rect[1], 200.0);
    assert_eq!(sprites[2].rect[1], 20.0);
}

#[test]
fn a_pull_up_takes_things_the_other_way() {
    let mut sprites = [at(20.0, 20.0)];
    pulled_along(&mut sprites, [0.0, 0.0, 100.0, 100.0], -12.0);
    assert_eq!(sprites[0].rect[1], 8.0);
}
