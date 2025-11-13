use wayland_client::protocol::wl_surface;

use crate::animation::SpringAnimation;

/// Manages input state for pointer and selection tracking
pub(crate) struct InputState {
    pub(super) pointer_position: (f64, f64),
    pub(super) mouse_pressed: bool,
    pub(super) selection_start: Option<(i32, i32)>,
    pub(super) current_surface: Option<wl_surface::WlSurface>,
    pub(super) snap_animation: Option<SpringAnimation>,
    pub(super) snap_initialized: bool,
}

impl InputState {
    pub(super) fn new() -> Self {
        Self {
            pointer_position: (0.0, 0.0),
            mouse_pressed: false,
            selection_start: None,
            current_surface: None,
            snap_animation: None,
            snap_initialized: false,
        }
    }
}
