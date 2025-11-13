use crate::selection::Rect;

// Spring physics constants
const SPRING_STIFFNESS: f64 = 400.0;
const MAX_TIME_STEP: f64 = 0.016; // 16ms prevents physics instability
const SNAP_ANIMATION_THRESHOLD: f64 = 0.5;
const POSITION_SETTLE_THRESHOLD: f64 = 5.0;
const VELOCITY_SETTLE_THRESHOLD: f64 = 50.0;

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
        let stiffness = SPRING_STIFFNESS;
        let damping: f64 = 2.0 * (stiffness.sqrt()) * 1.1;

        let (x, y, width, height) = initial.as_f64_tuple();

        Self {
            x,
            y,
            width,
            height,
            vx: 0.0,
            vy: 0.0,
            vw: 0.0,
            vh: 0.0,
            target_x: x,
            target_y: y,
            target_width: width,
            target_height: height,
            stiffness,
            damping,
        }
    }

    /// Set a new target rectangle
    pub fn set_target(&mut self, target: Rect) {
        let (x, y, width, height) = target.as_f64_tuple();
        self.target_x = x;
        self.target_y = y;
        self.target_width = width;
        self.target_height = height;
    }

    /// Update the animation by the given time delta (in seconds)
    pub fn update(&mut self, dt: f64) {
        let dt = dt.min(MAX_TIME_STEP);

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

    /// Snap value to target if within threshold (prevents sub-pixel jitter)
    #[inline]
    fn snap_to_target(current: f64, target: f64, threshold: f64) -> f64 {
        if (current - target).abs() < threshold {
            target
        } else {
            current
        }
    }

    /// Get the current animated rectangle
    pub fn current(&self) -> Rect {
        const SNAP_THRESHOLD: f64 = SNAP_ANIMATION_THRESHOLD;

        let x = Self::snap_to_target(self.x, self.target_x, SNAP_THRESHOLD);
        let y = Self::snap_to_target(self.y, self.target_y, SNAP_THRESHOLD);
        let width = Self::snap_to_target(self.width, self.target_width, SNAP_THRESHOLD);
        let height = Self::snap_to_target(self.height, self.target_height, SNAP_THRESHOLD);

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
        let position_threshold = POSITION_SETTLE_THRESHOLD;
        let velocity_threshold = VELOCITY_SETTLE_THRESHOLD;

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
