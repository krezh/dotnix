pub const CRUST: u32 = 0x11111b;
pub const BASE: u32 = 0x1e1e2e;
pub const SURFACE1: u32 = 0x45475a;
pub const OVERLAY1: u32 = 0x7f849c;
pub const OVERLAY2: u32 = 0x9399b2;
pub const SUBTEXT1: u32 = 0xbac2de;
pub const TEXT: u32 = 0xcdd6f4;
pub const LAVENDER: u32 = 0xb4befe;
pub const BLUE: u32 = 0x89b4fa;
pub const SAPPHIRE: u32 = 0x74c7ec;
pub const SKY: u32 = 0x89dceb;
pub const TEAL: u32 = 0x94e2d5;
pub const GREEN: u32 = 0xa6e3a1;
pub const YELLOW: u32 = 0xf9e2af;
pub const PEACH: u32 = 0xfab387;
pub const MAROON: u32 = 0xeba0ac;
pub const RED: u32 = 0xf38ba8;
pub const MAUVE: u32 = 0xcba6f7;
pub const PINK: u32 = 0xf5c2e7;
pub const FLAMINGO: u32 = 0xf2cdcd;

pub const ACCENT: u32 = BLUE;
pub const ON_ACCENT: u32 = CRUST;
pub const CRITICAL: u32 = RED;

pub fn app_color(app: &str) -> u32 {
    const COLORS: [u32; 12] = [
        BLUE, MAUVE, PEACH, GREEN, TEAL, PINK, SKY, YELLOW, LAVENDER, SAPPHIRE, MAROON, FLAMINGO,
    ];
    let hash = app
        .bytes()
        .fold(0u32, |hash, byte| hash.wrapping_mul(31).wrapping_add(byte.into()));
    COLORS[hash as usize % COLORS.len()]
}

pub const DEFAULT_OPACITY: f32 = 0.45;
pub const CARD_ALPHA: u8 = 0x26;
pub const NOTIFICATION_ALPHA: u8 = 0x1f;
pub const CARD_BORDER_ALPHA: u8 = 0x59;
pub const CONTROL_ALPHA: u8 = 0x59;
pub const TINT: u32 = TEXT;
pub const HOVER_ALPHA: u8 = 0x26;
pub const SELECTED_ALPHA: u8 = 0x33;
pub const TRACK_ALPHA: u8 = 0x40;
pub const ACCENT_TINT_ALPHA: u8 = 0x40;
pub const ACCENT_HOVER_ALPHA: u8 = 0x66;
pub const SURFACE: u32 = BASE;
pub const SURFACE_BORDER_ALPHA: u8 = 0x66;
pub const POPOVER_OPACITY: f32 = 0.95;
pub const SHADOW_ALPHA: f32 = 0.2;

pub const FONT: &str = "Rubik";

pub const fn with_alpha(rgb: u32, alpha: u8) -> u32 {
    (rgb << 8) | alpha as u32
}
