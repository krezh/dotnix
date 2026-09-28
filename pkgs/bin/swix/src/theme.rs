#![allow(dead_code)]
pub const CRUST: (f64, f64, f64) = (17.0 / 255.0, 17.0 / 255.0, 27.0 / 255.0);
pub const MANTLE: (f64, f64, f64) = (24.0 / 255.0, 24.0 / 255.0, 37.0 / 255.0);
pub const BASE: (f64, f64, f64) = (30.0 / 255.0, 30.0 / 255.0, 46.0 / 255.0);
pub const SURFACE0: (f64, f64, f64) = (49.0 / 255.0, 50.0 / 255.0, 68.0 / 255.0);
pub const SURFACE1: (f64, f64, f64) = (69.0 / 255.0, 71.0 / 255.0, 90.0 / 255.0);
pub const SURFACE2: (f64, f64, f64) = (88.0 / 255.0, 91.0 / 255.0, 112.0 / 255.0);
pub const OVERLAY0: (f64, f64, f64) = (108.0 / 255.0, 112.0 / 255.0, 134.0 / 255.0);
pub const OVERLAY1: (f64, f64, f64) = (127.0 / 255.0, 132.0 / 255.0, 156.0 / 255.0);
pub const OVERLAY2: (f64, f64, f64) = (147.0 / 255.0, 153.0 / 255.0, 178.0 / 255.0);
pub const SUBTEXT0: (f64, f64, f64) = (166.0 / 255.0, 173.0 / 255.0, 200.0 / 255.0);
pub const SUBTEXT1: (f64, f64, f64) = (186.0 / 255.0, 194.0 / 255.0, 222.0 / 255.0);
pub const TEXT: (f64, f64, f64) = (205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0);

pub const LAVENDER: (f64, f64, f64) = (180.0 / 255.0, 190.0 / 255.0, 254.0 / 255.0);
pub const BLUE: (f64, f64, f64) = (137.0 / 255.0, 180.0 / 255.0, 250.0 / 255.0);
pub const SAPPHIRE: (f64, f64, f64) = (116.0 / 255.0, 199.0 / 255.0, 236.0 / 255.0);
pub const SKY: (f64, f64, f64) = (137.0 / 255.0, 220.0 / 255.0, 235.0 / 255.0);
pub const TEAL: (f64, f64, f64) = (148.0 / 255.0, 226.0 / 255.0, 213.0 / 255.0);
pub const GREEN: (f64, f64, f64) = (166.0 / 255.0, 227.0 / 255.0, 161.0 / 255.0);
pub const YELLOW: (f64, f64, f64) = (249.0 / 255.0, 226.0 / 255.0, 175.0 / 255.0);
pub const PEACH: (f64, f64, f64) = (250.0 / 255.0, 179.0 / 255.0, 135.0 / 255.0);
pub const MAROON: (f64, f64, f64) = (235.0 / 255.0, 160.0 / 255.0, 172.0 / 255.0);
pub const RED: (f64, f64, f64) = (243.0 / 255.0, 139.0 / 255.0, 168.0 / 255.0);
pub const MAUVE: (f64, f64, f64) = (203.0 / 255.0, 166.0 / 255.0, 247.0 / 255.0);
pub const PINK: (f64, f64, f64) = (245.0 / 255.0, 194.0 / 255.0, 231.0 / 255.0);
pub const FLAMINGO: (f64, f64, f64) = (242.0 / 255.0, 205.0 / 255.0, 205.0 / 255.0);
pub const ROSEWATER: (f64, f64, f64) = (245.0 / 255.0, 224.0 / 255.0, 220.0 / 255.0);

pub fn set_source_rgb(context: &gtk::cairo::Context, (r, g, b): (f64, f64, f64)) {
    context.set_source_rgb(r, g, b);
}

pub fn set_source_rgba(context: &gtk::cairo::Context, (r, g, b): (f64, f64, f64), a: f64) {
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

pub fn capsule_point(cap_x: f64, y_cap: f64, w_cap: f64, h_cap: f64, r: f64, u: f64) -> (f64, f64) {
    let u = u.rem_euclid(1.0);
    let l_straight = (w_cap - 2.0 * r).max(0.0);
    let l_arc = std::f64::consts::PI * r;
    let total = 2.0 * l_straight + 2.0 * l_arc;
    if total <= 0.0 {
        return (cap_x, y_cap);
    }
    let d = u * total;

    if d < l_straight {
        (cap_x + r + d, y_cap)
    } else if d < l_straight + l_arc {
        let arc_d = d - l_straight;
        let angle = -std::f64::consts::FRAC_PI_2 + (arc_d / r);
        let cx = cap_x + w_cap - r;
        let cy = y_cap + r;
        (cx + r * angle.cos(), cy + r * angle.sin())
    } else if d < 2.0 * l_straight + l_arc {
        let seg_d = d - (l_straight + l_arc);
        (cap_x + w_cap - r - seg_d, y_cap + h_cap)
    } else {
        let arc_d = d - (2.0 * l_straight + l_arc);
        let angle = std::f64::consts::FRAC_PI_2 + (arc_d / r);
        let cx = cap_x + r;
        let cy = y_cap + r;
        (cx + r * angle.cos(), cy + r * angle.sin())
    }
}
