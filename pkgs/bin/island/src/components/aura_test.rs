use super::*;

fn playing() -> Aura {
    let aura = Aura::default();
    aura.0.borrow_mut().levels = [0.6; BANDS];
    aura
}

fn alpha(pixels: &[u8], width: u32, x: u32, y: u32) -> u8 {
    pixels[((y * width + x) * 4 + 3) as usize]
}

#[test]
fn light_below_the_bar_fades_on_down() {
    let aura = playing();
    let part = Part {
        field: (600.0, 40.0),
        offset: (200.0, 40.0),
    };
    let (w, h, pixels) = aura.pixels_of(200.0, 120.0, 1.0, part);
    let top = alpha(&pixels, w, w / 2, 1);
    let bottom = alpha(&pixels, w, w / 2, h - 2);
    assert!(top > 0);
    assert!(top > bottom);
}
