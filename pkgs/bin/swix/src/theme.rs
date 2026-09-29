pub const CRUST: (f64, f64, f64) = (17.0 / 255.0, 17.0 / 255.0, 27.0 / 255.0);
pub const MANTLE: (f64, f64, f64) = (24.0 / 255.0, 24.0 / 255.0, 37.0 / 255.0);
pub const BASE: (f64, f64, f64) = (30.0 / 255.0, 30.0 / 255.0, 46.0 / 255.0);
pub const SURFACE1: (f64, f64, f64) = (69.0 / 255.0, 71.0 / 255.0, 90.0 / 255.0);
pub const OVERLAY0: (f64, f64, f64) = (108.0 / 255.0, 112.0 / 255.0, 134.0 / 255.0);
pub const TEXT: (f64, f64, f64) = (205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0);

pub const LAVENDER: (f64, f64, f64) = (180.0 / 255.0, 190.0 / 255.0, 254.0 / 255.0);
pub const SAPPHIRE: (f64, f64, f64) = (116.0 / 255.0, 199.0 / 255.0, 236.0 / 255.0);
pub const SKY: (f64, f64, f64) = (137.0 / 255.0, 220.0 / 255.0, 235.0 / 255.0);
pub const TEAL: (f64, f64, f64) = (148.0 / 255.0, 226.0 / 255.0, 213.0 / 255.0);
pub const GREEN: (f64, f64, f64) = (166.0 / 255.0, 227.0 / 255.0, 161.0 / 255.0);
pub const YELLOW: (f64, f64, f64) = (249.0 / 255.0, 226.0 / 255.0, 175.0 / 255.0);
pub const RED: (f64, f64, f64) = (243.0 / 255.0, 139.0 / 255.0, 168.0 / 255.0);

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
