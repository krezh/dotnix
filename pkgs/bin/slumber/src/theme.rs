use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(default = "default_alpha")]
    pub a: u8,
}

const fn default_alpha() -> u8 {
    255
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    #[inline(always)]
    pub const fn to_argb_u32(self) -> u32 {
        let alpha = self.a as u32;
        (alpha << 24)
            | (((self.r as u32 * alpha + 127) / 255) << 16)
            | (((self.g as u32 * alpha + 127) / 255) << 8)
            | ((self.b as u32 * alpha + 127) / 255)
    }

    pub const fn with_alpha(mut self, a: u8) -> Self {
        self.a = a;
        self
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;
        Self {
            r: (self.r as f32 * inv + other.r as f32 * t).round() as u8,
            g: (self.g as f32 * inv + other.g as f32 * t).round() as u8,
            b: (self.b as f32 * inv + other.b as f32 * t).round() as u8,
            a: (self.a as f32 * inv + other.a as f32 * t).round() as u8,
        }
    }

    #[inline(always)]
    pub fn blend_over_pixel(self, background: u32) -> u32 {
        blend_premultiplied(self.to_argb_u32(), background)
    }
}

/// Source-over for Wayland's premultiplied ARGB pixels.
#[inline(always)]
pub fn blend_premultiplied(foreground: u32, background: u32) -> u32 {
    let alpha = foreground >> 24;
    if alpha == 255 {
        return foreground;
    }
    if alpha == 0 {
        return background;
    }
    let inverse = 255 - alpha;
    let channel = |shift: u32| {
        ((foreground >> shift) & 255) + (((background >> shift) & 255) * inverse + 127) / 255
    };
    (channel(24) << 24) | (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub name: String,
    // Accents
    pub rosewater: Color,
    pub flamingo: Color,
    pub pink: Color,
    pub mauve: Color,
    pub red: Color,
    pub maroon: Color,
    pub peach: Color,
    pub yellow: Color,
    pub green: Color,
    pub teal: Color,
    pub sky: Color,
    pub sapphire: Color,
    pub blue: Color,
    pub lavender: Color,
    // Neutrals
    pub text: Color,
    pub subtext1: Color,
    pub subtext0: Color,
    pub overlay2: Color,
    pub overlay1: Color,
    pub overlay0: Color,
    pub surface2: Color,
    pub surface1: Color,
    pub surface0: Color,
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,
}

impl Theme {
    pub fn mocha() -> Self {
        Self {
            name: "mocha".into(),
            rosewater: Color::rgb(0xf5, 0xe0, 0xdc),
            flamingo: Color::rgb(0xf2, 0xcd, 0xcd),
            pink: Color::rgb(0xf5, 0xc2, 0xe7),
            mauve: Color::rgb(0xcb, 0xa6, 0xf7),
            red: Color::rgb(0xf3, 0x8b, 0xa8),
            maroon: Color::rgb(0xeb, 0xa0, 0xac),
            peach: Color::rgb(0xfa, 0xb3, 0x87),
            yellow: Color::rgb(0xf9, 0xe2, 0xaf),
            green: Color::rgb(0xa6, 0xe3, 0xa1),
            teal: Color::rgb(0x94, 0xe2, 0xd5),
            sky: Color::rgb(0x89, 0xdc, 0xeb),
            sapphire: Color::rgb(0x74, 0xc7, 0xec),
            blue: Color::rgb(0x89, 0xb4, 0xfa),
            lavender: Color::rgb(0xb4, 0xbe, 0xfe),

            text: Color::rgb(0xcd, 0xd6, 0xf4),
            subtext1: Color::rgb(0xba, 0xc2, 0xde),
            subtext0: Color::rgb(0xa6, 0xad, 0xc8),
            overlay2: Color::rgb(0x93, 0x99, 0xb2),
            overlay1: Color::rgb(0x7f, 0x84, 0x9c),
            overlay0: Color::rgb(0x6c, 0x70, 0x86),
            surface2: Color::rgb(0x58, 0x5b, 0x70),
            surface1: Color::rgb(0x45, 0x47, 0x5a),
            surface0: Color::rgb(0x31, 0x32, 0x44),
            base: Color::rgb(0x1e, 0x1e, 0x2e),
            mantle: Color::rgb(0x18, 0x18, 0x25),
            crust: Color::rgb(0x11, 0x11, 0x1b),
        }
    }

    pub fn macchiato() -> Self {
        Self {
            name: "macchiato".into(),
            rosewater: Color::rgb(0xf4, 0xdb, 0xd6),
            flamingo: Color::rgb(0xf0, 0xc6, 0xc6),
            pink: Color::rgb(0xf5, 0xbd, 0xe6),
            mauve: Color::rgb(0xc6, 0xa0, 0xf6),
            red: Color::rgb(0xed, 0x87, 0x96),
            maroon: Color::rgb(0xee, 0x99, 0xa0),
            peach: Color::rgb(0xf5, 0xa9, 0x7f),
            yellow: Color::rgb(0xee, 0xd4, 0x9f),
            green: Color::rgb(0xa6, 0xda, 0x95),
            teal: Color::rgb(0x8b, 0xd5, 0xca),
            sky: Color::rgb(0x91, 0xd7, 0xe3),
            sapphire: Color::rgb(0x7d, 0xc4, 0xe4),
            blue: Color::rgb(0x8a, 0xad, 0xf4),
            lavender: Color::rgb(0xb7, 0xbd, 0xf8),

            text: Color::rgb(0xca, 0xd3, 0xf5),
            subtext1: Color::rgb(0xb8, 0xc0, 0xe0),
            subtext0: Color::rgb(0xa5, 0xad, 0xcb),
            overlay2: Color::rgb(0x93, 0x9a, 0xb7),
            overlay1: Color::rgb(0x80, 0x87, 0xa2),
            overlay0: Color::rgb(0x6e, 0x73, 0x8d),
            surface2: Color::rgb(0x5b, 0x60, 0x78),
            surface1: Color::rgb(0x49, 0x4d, 0x64),
            surface0: Color::rgb(0x36, 0x3a, 0x4f),
            base: Color::rgb(0x24, 0x27, 0x3a),
            mantle: Color::rgb(0x1e, 0x20, 0x30),
            crust: Color::rgb(0x18, 0x19, 0x26),
        }
    }

    pub fn frappe() -> Self {
        Self {
            name: "frappe".into(),
            rosewater: Color::rgb(0xf2, 0xd5, 0xcf),
            flamingo: Color::rgb(0xee, 0xbe, 0xbe),
            pink: Color::rgb(0xf4, 0xb8, 0xe4),
            mauve: Color::rgb(0xca, 0x9e, 0xe6),
            red: Color::rgb(0xe7, 0x82, 0x84),
            maroon: Color::rgb(0xea, 0x99, 0x9c),
            peach: Color::rgb(0xef, 0x9f, 0x76),
            yellow: Color::rgb(0xe5, 0xc8, 0x90),
            green: Color::rgb(0xa6, 0xd1, 0x89),
            teal: Color::rgb(0x81, 0xc8, 0xbe),
            sky: Color::rgb(0x99, 0xd1, 0xdb),
            sapphire: Color::rgb(0x85, 0xc1, 0xdc),
            blue: Color::rgb(0x8c, 0xaa, 0xee),
            lavender: Color::rgb(0xba, 0xbb, 0xf1),

            text: Color::rgb(0xc6, 0xd0, 0xf5),
            subtext1: Color::rgb(0xb5, 0xbf, 0xe2),
            subtext0: Color::rgb(0xa5, 0xad, 0xce),
            overlay2: Color::rgb(0x94, 0x9c, 0xbb),
            overlay1: Color::rgb(0x83, 0x8b, 0xa7),
            overlay0: Color::rgb(0x73, 0x79, 0x94),
            surface2: Color::rgb(0x62, 0x68, 0x80),
            surface1: Color::rgb(0x51, 0x57, 0x6d),
            surface0: Color::rgb(0x41, 0x45, 0x59),
            base: Color::rgb(0x30, 0x34, 0x46),
            mantle: Color::rgb(0x29, 0x2c, 0x3c),
            crust: Color::rgb(0x23, 0x26, 0x34),
        }
    }

    pub fn latte() -> Self {
        Self {
            name: "latte".into(),
            rosewater: Color::rgb(0xdc, 0x8a, 0x78),
            flamingo: Color::rgb(0xdd, 0x78, 0x78),
            pink: Color::rgb(0xea, 0x76, 0xcb),
            mauve: Color::rgb(0x88, 0x39, 0xef),
            red: Color::rgb(0xd2, 0x0f, 0x39),
            maroon: Color::rgb(0xe6, 0x45, 0x53),
            peach: Color::rgb(0xfe, 0x64, 0x0b),
            yellow: Color::rgb(0xdf, 0x8e, 0x1d),
            green: Color::rgb(0x40, 0xa0, 0x2b),
            teal: Color::rgb(0x17, 0x92, 0x99),
            sky: Color::rgb(0x04, 0xa5, 0xe5),
            sapphire: Color::rgb(0x20, 0x9f, 0xb5),
            blue: Color::rgb(0x1e, 0x66, 0xf2),
            lavender: Color::rgb(0x72, 0x87, 0xfd),

            text: Color::rgb(0x4c, 0x4f, 0x69),
            subtext1: Color::rgb(0x5c, 0x5f, 0x77),
            subtext0: Color::rgb(0x6c, 0x6f, 0x85),
            overlay2: Color::rgb(0x7c, 0x7f, 0x93),
            overlay1: Color::rgb(0x8c, 0x8f, 0xa1),
            overlay0: Color::rgb(0x9c, 0xa0, 0xb0),
            surface2: Color::rgb(0xac, 0xb0, 0xbe),
            surface1: Color::rgb(0xbc, 0xc0, 0xcc),
            surface0: Color::rgb(0xcc, 0xd0, 0xda),
            base: Color::rgb(0xef, 0xf1, 0xf5),
            mantle: Color::rgb(0xe6, 0xe9, 0xef),
            crust: Color::rgb(0xdc, 0xe0, 0xe8),
        }
    }

    pub fn accents(&self) -> [Color; 14] {
        [
            self.rosewater,
            self.flamingo,
            self.pink,
            self.mauve,
            self.red,
            self.maroon,
            self.peach,
            self.yellow,
            self.green,
            self.teal,
            self.sky,
            self.sapphire,
            self.blue,
            self.lavender,
        ]
    }

    pub fn with_background_opacity(&self, opacity: f32) -> Self {
        let mut t = self.clone();
        let alpha = (255.0 * opacity.clamp(0.0, 1.0)).round() as u8;
        t.crust = t.crust.with_alpha(alpha);
        t.mantle = t.mantle.with_alpha(alpha);
        t.base = t.base.with_alpha(alpha);
        t
    }
}
