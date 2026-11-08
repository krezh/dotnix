//! Surfaces drawn over everything else on the island, such as tooltips and an open dropdown, and
//! how one grows out of what opens it. A surface belongs to the panel it was opened from: it fades
//! with it and closes when that panel stops being the one showing. While open it sits over a
//! [`scrim`], so a click anywhere else only closes it. While closing, or before its rows show, it
//! takes no clicks at all.

/// Room around the surface for its shadow; popovers are laid out as roots, so it is kept as margin.
pub const SHADOW_SPREAD: f32 = 6.0;
/// How far a surface has grown when what opened it has faded and its rows start to show.
pub const ROWS_FROM: f32 = 0.2;
/// How far it has grown when its rows show in full and it has its own color, and it starts taking
/// clicks.
pub const ROWS_AT: f32 = 0.6;

/// How much of a surface's rows show at `look`: none until what opened it has faded, so the same
/// words never show twice.
pub fn rows_shown(look: f32) -> f32 {
    ((look - ROWS_FROM) / (ROWS_AT - ROWS_FROM)).clamp(0.0, 1.0)
}

/// How much of what opened it still shows on the surface at `look`.
pub fn opener_shown(look: f32) -> f32 {
    (1.0 - look / ROWS_FROM).clamp(0.0, 1.0)
}

/// Reaches the surface's own color only as its rows arrive, so it is never dark and empty.
pub fn cover(look: f32) -> f32 {
    (look / ROWS_AT).clamp(0.0, 1.0)
}
