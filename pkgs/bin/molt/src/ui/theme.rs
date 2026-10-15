//! Molt's colours: the Catppuccin Mocha palette and the roles colours play, defined once.
//!
//! CSS gets them as custom properties (`var(--green)`, `var(--version)`) via `css_variables`;
//! Pango markup prints them as `#rrggbb`; cairo drawing takes their `rgb` channels.

use std::fmt;

/// An opaque sRGB colour, `0xrrggbb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(u32);

impl Color {
    /// Channels in `0.0..=1.0`, as cairo takes them.
    pub const fn rgb(self) -> (f64, f64, f64) {
        let (r, g, b) = ((self.0 >> 16) & 0xff, (self.0 >> 8) & 0xff, self.0 & 0xff);
        (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
    }

    /// This colour at `alpha` opacity, for GTK APIs that take a `gdk::RGBA`.
    pub fn with_alpha(self, alpha: f32) -> gtk::gdk::RGBA {
        let (r, g, b) = self.rgb();
        gtk::gdk::RGBA::new(r as f32, g as f32, b as f32, alpha)
    }
}

/// `#rrggbb`, for Pango markup and GTK colour strings.
impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "#{:06x}", self.0)
    }
}

pub const ROSEWATER: Color = Color(0xf5e0dc);
pub const FLAMINGO: Color = Color(0xf2cdcd);
pub const PINK: Color = Color(0xf5c2e7);
pub const MAUVE: Color = Color(0xcba6f7);
pub const RED: Color = Color(0xf38ba8);
pub const MAROON: Color = Color(0xeba0ac);
pub const PEACH: Color = Color(0xfab387);
pub const YELLOW: Color = Color(0xf9e2af);
pub const GREEN: Color = Color(0xa6e3a1);
pub const TEAL: Color = Color(0x94e2d5);
pub const SKY: Color = Color(0x89dceb);
pub const SAPPHIRE: Color = Color(0x74c7ec);
pub const BLUE: Color = Color(0x89b4fa);
pub const LAVENDER: Color = Color(0xb4befe);
pub const TEXT: Color = Color(0xcdd6f4);
pub const SUBTEXT1: Color = Color(0xbac2de);
pub const SUBTEXT0: Color = Color(0xa6adc8);
pub const OVERLAY2: Color = Color(0x9399b2);
pub const OVERLAY1: Color = Color(0x7f849c);
pub const OVERLAY0: Color = Color(0x6c7086);
pub const SURFACE2: Color = Color(0x585b70);
pub const SURFACE1: Color = Color(0x45475a);
pub const SURFACE0: Color = Color(0x313244);
pub const BASE: Color = Color(0x1e1e2e);
pub const MANTLE: Color = Color(0x181825);
pub const CRUST: Color = Color(0x11111b);

/// Each palette colour: its CSS name and the `.tone-<name>` class that selects it as `--tone`.
const PALETTE: [(&str, &str, Color); 26] = [
    ("rosewater", "tone-rosewater", ROSEWATER),
    ("flamingo", "tone-flamingo", FLAMINGO),
    ("pink", "tone-pink", PINK),
    ("mauve", "tone-mauve", MAUVE),
    ("red", "tone-red", RED),
    ("maroon", "tone-maroon", MAROON),
    ("peach", "tone-peach", PEACH),
    ("yellow", "tone-yellow", YELLOW),
    ("green", "tone-green", GREEN),
    ("teal", "tone-teal", TEAL),
    ("sky", "tone-sky", SKY),
    ("sapphire", "tone-sapphire", SAPPHIRE),
    ("blue", "tone-blue", BLUE),
    ("lavender", "tone-lavender", LAVENDER),
    ("text", "tone-text", TEXT),
    ("subtext1", "tone-subtext1", SUBTEXT1),
    ("subtext0", "tone-subtext0", SUBTEXT0),
    ("overlay2", "tone-overlay2", OVERLAY2),
    ("overlay1", "tone-overlay1", OVERLAY1),
    ("overlay0", "tone-overlay0", OVERLAY0),
    ("surface2", "tone-surface2", SURFACE2),
    ("surface1", "tone-surface1", SURFACE1),
    ("surface0", "tone-surface0", SURFACE0),
    ("base", "tone-base", BASE),
    ("mantle", "tone-mantle", MANTLE),
    ("crust", "tone-crust", CRUST),
];

/// Primary actions and links.
pub const ACCENT: Color = BLUE;
/// Focus rings, hovered primaries.
pub const FOCUS: Color = LAVENDER;
/// Versions and revisions, on every screen.
pub const VERSION: Color = MAUVE;
/// Code, and the system's `/etc`.
pub const CODE: Color = PEACH;
/// Home Manager.
pub const HOME: Color = TEAL;

/// Roles by CSS name.
const ROLES: [(&str, Color); 5] = [
    ("accent", ACCENT),
    ("focus", FOCUS),
    ("version", VERSION),
    ("code", CODE),
    ("home", HOME),
];

fn entry(color: Color) -> (&'static str, &'static str, Color) {
    PALETTE
        .into_iter()
        .find(|(_, _, candidate)| *candidate == color)
        .expect("every Color constant is in the palette")
}

/// The `.tone-<name>` class that makes `.tinted`, `.toned`, and `.notice` elements (and their
/// descendants) draw with `color`.
pub fn tone(color: Color) -> &'static str {
    entry(color).1
}

/// The translucent layers content rests on, by CSS name: a palette colour at an opacity. The
/// window itself is base at 75%; panels sit on it, cards and selected rows are raised above.
const SURFACES: [(&str, Color, u8); 3] = [
    ("panel", MANTLE, 60),
    ("raised", SURFACE0, 70),
    ("hover", SURFACE0, 60),
];

/// Palette, roles, and surfaces as CSS custom properties plus the tone classes, prepended to
/// every Molt stylesheet.
pub fn css_variables() -> String {
    let mut css = String::from(":root {\n");
    for (name, _, color) in PALETTE {
        css.push_str(&format!("  --{name}: {color};\n"));
    }
    for (role, color) in ROLES {
        css.push_str(&format!("  --{role}: var(--{});\n", entry(color).0));
    }
    for (surface, color, percent) in SURFACES {
        css.push_str(&format!(
            "  --{surface}: color-mix(in srgb, var(--{}) {percent}%, transparent);\n",
            entry(color).0
        ));
    }
    css.push_str("}\n");
    for (name, class, _) in PALETTE {
        css.push_str(&format!(".{class} {{ --tone: var(--{name}); }}\n"));
    }
    css
}

pub fn set_source_rgb(context: &gtk::cairo::Context, color: Color) {
    let (r, g, b) = color.rgb();
    context.set_source_rgb(r, g, b);
}

pub fn set_source_rgba(context: &gtk::cairo::Context, color: Color, a: f64) {
    let (r, g, b) = color.rgb();
    context.set_source_rgba(r, g, b, a);
}

pub fn rounded_rectangle(
    context: &gtk::cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
) {
    let r = radius.min(width / 2.0).min(height / 2.0);
    if r <= 0.0 {
        context.rectangle(x, y, width, height);
        return;
    }
    context.new_sub_path();
    context.arc(x + width - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    context.arc(
        x + width - r,
        y + height - r,
        r,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    context.arc(
        x + r,
        y + height - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    context.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    context.close_path();
}

/// Length of the rounded-rectangle outline traced by [`capsule_point`].
pub fn capsule_perimeter(w_cap: f64, h_cap: f64, r: f64) -> f64 {
    let r = r.min(w_cap / 2.0).min(h_cap / 2.0);
    2.0 * (w_cap - 2.0 * r).max(0.0) + 2.0 * (h_cap - 2.0 * r).max(0.0) + std::f64::consts::TAU * r
}

pub fn capsule_point(cap_x: f64, y_cap: f64, w_cap: f64, h_cap: f64, r: f64, u: f64) -> (f64, f64) {
    let u = u.rem_euclid(1.0);
    let r = r.min(w_cap / 2.0).min(h_cap / 2.0);
    let l_horiz = (w_cap - 2.0 * r).max(0.0);
    let l_vert = (h_cap - 2.0 * r).max(0.0);
    let l_arc = std::f64::consts::FRAC_PI_2 * r;
    let total = 2.0 * l_horiz + 2.0 * l_vert + 4.0 * l_arc;
    if total <= 0.0 {
        return (cap_x, y_cap);
    }
    let d = u * total;

    // 1. Top horizontal edge (left to right)
    if d < l_horiz {
        return (cap_x + r + d, y_cap);
    }
    let d = d - l_horiz;

    // 2. Top-right corner arc (-PI/2 to 0)
    if d < l_arc {
        let angle = -std::f64::consts::FRAC_PI_2 + (d / r);
        let cx = cap_x + w_cap - r;
        let cy = y_cap + r;
        return (cx + r * angle.cos(), cy + r * angle.sin());
    }
    let d = d - l_arc;

    // 3. Right vertical edge (top to bottom)
    if d < l_vert {
        return (cap_x + w_cap, y_cap + r + d);
    }
    let d = d - l_vert;

    // 4. Bottom-right corner arc (0 to PI/2)
    if d < l_arc {
        let angle = d / r;
        let cx = cap_x + w_cap - r;
        let cy = y_cap + h_cap - r;
        return (cx + r * angle.cos(), cy + r * angle.sin());
    }
    let d = d - l_arc;

    // 5. Bottom horizontal edge (right to left)
    if d < l_horiz {
        return (cap_x + w_cap - r - d, y_cap + h_cap);
    }
    let d = d - l_horiz;

    // 6. Bottom-left corner arc (PI/2 to PI)
    if d < l_arc {
        let angle = std::f64::consts::FRAC_PI_2 + (d / r);
        let cx = cap_x + r;
        let cy = y_cap + h_cap - r;
        return (cx + r * angle.cos(), cy + r * angle.sin());
    }
    let d = d - l_arc;

    // 7. Left vertical edge (bottom to top)
    if d < l_vert {
        return (cap_x, y_cap + h_cap - r - d);
    }
    let d = d - l_vert;

    // 8. Top-left corner arc (PI to 3*PI/2)
    let angle = std::f64::consts::PI + (d / r);
    let cx = cap_x + r;
    let cy = y_cap + r;
    (cx + r * angle.cos(), cy + r * angle.sin())
}
