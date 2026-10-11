pub mod clock;
pub mod starfield;
pub mod waves;

use crate::canvas::Canvas;
use crate::rng::Rng;
use crate::theme::Theme;

pub trait Animation: Send {
    fn update(&mut self, dt: f32, width: u32, height: u32);
    fn render(&mut self, canvas: &mut Canvas, theme: &Theme);
}

pub const ANIMATION_NAMES: &[&str] = &[
    "waves",
    "starfield",
    "clock",
];

pub fn create_animation(name: &str, rng: &mut Rng, font_family: &str) -> Box<dyn Animation> {
    match name.to_lowercase().as_str() {
        "waves" => Box::new(waves::AuroraWaves::new(rng)),
        "starfield" => Box::new(starfield::Starfield::new(rng)),
        "clock" => Box::new(clock::MinimalClock::new(rng, font_family)),
        "random" | _ => {
            let choice = *rng.choice(ANIMATION_NAMES);
            create_animation(choice, rng, font_family)
        }
    }
}
