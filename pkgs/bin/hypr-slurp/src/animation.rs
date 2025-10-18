use crate::selection::Rect;

/// Critically damped spring animation (smooth motion, no overshoot/bounce)
#[derive(Debug, Clone)]
pub struct SpringAnimation {
    // Current animated values
    x: f64,
    y: f64,
    width: f64,
    height: f64,

    // Velocities
    vx: f64,
    vy: f64,
    vw: f64,
    vh: f64,

    // Target values
    target_x: f64,
    target_y: f64,
    target_width: f64,
    target_height: f64,

    // Spring physics parameters
    stiffness: f64,
    damping: f64,
}

impl SpringAnimation {
    /// Create a new spring animation starting at the given rectangle
    pub fn new(initial: Rect) -> Self {
        // Higher stiffness = faster settling time
        let stiffness: f64 = 400.0;
        // Critical damping = 2 * sqrt(stiffness)
        // Using slightly more than critical to ensure no overshoot
        let damping: f64 = 2.0 * (stiffness.sqrt()) * 1.1;

        Self {
            x: initial.x as f64,
            y: initial.y as f64,
            width: initial.width as f64,
            height: initial.height as f64,
            vx: 0.0,
            vy: 0.0,
            vw: 0.0,
            vh: 0.0,
            target_x: initial.x as f64,
            target_y: initial.y as f64,
            target_width: initial.width as f64,
            target_height: initial.height as f64,
            stiffness,
            damping,
        }
    }

    /// Set a new target rectangle
    pub fn set_target(&mut self, target: Rect) {
        self.target_x = target.x as f64;
        self.target_y = target.y as f64;
        self.target_width = target.width as f64;
        self.target_height = target.height as f64;
    }

    /// Update the animation by the given time delta (in seconds)
    pub fn update(&mut self, dt: f64) {
        // Clamp dt to prevent instability from large time steps
        let dt = dt.min(0.016); // Max 16ms per step

        // Spring physics: F = -k * x - d * v
        // Using semi-implicit Euler integration for stability

        // Position springs
        let fx = -self.stiffness * (self.x - self.target_x) - self.damping * self.vx;
        let fy = -self.stiffness * (self.y - self.target_y) - self.damping * self.vy;

        // Size springs
        let fw = -self.stiffness * (self.width - self.target_width) - self.damping * self.vw;
        let fh = -self.stiffness * (self.height - self.target_height) - self.damping * self.vh;

        // Update velocities
        self.vx += fx * dt;
        self.vy += fy * dt;
        self.vw += fw * dt;
        self.vh += fh * dt;

        // Update positions
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.width += self.vw * dt;
        self.height += self.vh * dt;
    }

    /// Get the current animated rectangle
    pub fn current(&self) -> Rect {
        // Snap to target if very close (prevents sub-pixel jitter)
        let snap_threshold = 0.5;

        let x = if (self.x - self.target_x).abs() < snap_threshold {
            self.target_x
        } else {
            self.x
        };

        let y = if (self.y - self.target_y).abs() < snap_threshold {
            self.target_y
        } else {
            self.y
        };

        let width = if (self.width - self.target_width).abs() < snap_threshold {
            self.target_width
        } else {
            self.width
        };

        let height = if (self.height - self.target_height).abs() < snap_threshold {
            self.target_height
        } else {
            self.height
        };

        Rect::new(
            x.round() as i32,
            y.round() as i32,
            width.round() as i32,
            height.round() as i32,
        )
    }

    /// Check if the animation has settled (for FPS optimization)
    pub fn is_settled(&self) -> bool {
        // Conservative thresholds for FPS reduction - we want smooth animation
        // but also want to save CPU when mostly idle
        let position_threshold = 5.0; // Within 5 pixels (still animating smoothly)
        let velocity_threshold = 50.0; // Slowing down significantly

        let dx = (self.x - self.target_x).abs();
        let dy = (self.y - self.target_y).abs();
        let dw = (self.width - self.target_width).abs();
        let dh = (self.height - self.target_height).abs();

        dx < position_threshold
            && dy < position_threshold
            && dw < position_threshold
            && dh < position_threshold
            && self.vx.abs() < velocity_threshold
            && self.vy.abs() < velocity_threshold
            && self.vw.abs() < velocity_threshold
            && self.vh.abs() < velocity_threshold
    }
}
