//! Easing, blending and colour conversion shared by everything that animates or paints.

pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 0 at `from` or before, 1 at `to` or past, easing in and out between; either way round.
pub fn smooth(from: f32, to: f32, at: f32) -> f32 {
    smoothstep((at - from) / (to - from))
}

pub fn mix(from: f32, to: f32, at: f32) -> f32 {
    from + (to - from) * at
}

/// From a color packed as `0xrrggbb`.
pub fn rgb(rgb: u32) -> [f32; 3] {
    let [_, r, g, b] = rgb.to_be_bytes().map(|channel| f32::from(channel) / 255.0);
    [r, g, b]
}

/// From a color packed as `0xrrggbb`, opaque.
pub fn color(rgb: u32) -> [f32; 4] {
    let [r, g, b] = self::rgb(rgb);
    [r, g, b, 1.0]
}
