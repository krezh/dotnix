use std::time::Instant;

const CRITICAL_TOLERANCE: f32 = 1e-4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringConfig {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpringState {
    pub position: f32,
    pub velocity: f32,
}

impl SpringConfig {
    pub const fn new(stiffness: f32, damping: f32, mass: f32) -> Self {
        Self {
            stiffness,
            damping,
            mass,
        }
    }

    fn canonical(&self) -> (f32, f32) {
        let natural_frequency = (self.stiffness / self.mass).sqrt();
        let damping_ratio = self.damping / (2.0 * (self.stiffness * self.mass).sqrt());
        (natural_frequency, damping_ratio)
    }

    /// Exact for a fixed target, so it does not depend on the frame rate.
    pub fn step(&self, state: SpringState, target: f32, delta_time: f32) -> SpringState {
        let propagator = self.propagator(delta_time);
        let displacement = state.position - target;
        SpringState {
            position: target + propagator[0][0] * displacement + propagator[0][1] * state.velocity,
            velocity: propagator[1][0] * displacement + propagator[1][1] * state.velocity,
        }
    }

    fn propagator(&self, delta_time: f32) -> [[f32; 2]; 2] {
        let (natural_frequency, damping_ratio) = self.canonical();
        if damping_ratio < 1.0 - CRITICAL_TOLERANCE {
            let decay = damping_ratio * natural_frequency;
            let damped_frequency = natural_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();
            let exponential = (-decay * delta_time).exp();
            let (sine, cosine) = (damped_frequency * delta_time).sin_cos();
            let sine_over_frequency = sine / damped_frequency;
            [
                [
                    exponential * (cosine + decay * sine_over_frequency),
                    exponential * sine_over_frequency,
                ],
                [
                    -exponential * natural_frequency * natural_frequency * sine_over_frequency,
                    exponential * (cosine - decay * sine_over_frequency),
                ],
            ]
        } else if damping_ratio > 1.0 + CRITICAL_TOLERANCE {
            let root = (damping_ratio * damping_ratio - 1.0).sqrt();
            let root_sum = damping_ratio + root;
            let slow_root = -natural_frequency / root_sum;
            let fast_root = -natural_frequency * root_sum;
            let denominator = slow_root - fast_root;
            let slow_exponential = (slow_root * delta_time).exp();
            let fast_exponential = (fast_root * delta_time).exp();
            [
                [
                    (-fast_root * slow_exponential + slow_root * fast_exponential) / denominator,
                    (slow_exponential - fast_exponential) / denominator,
                ],
                [
                    slow_root * fast_root * (fast_exponential - slow_exponential) / denominator,
                    (slow_root * slow_exponential - fast_root * fast_exponential) / denominator,
                ],
            ]
        } else {
            let exponential = (-natural_frequency * delta_time).exp();
            [
                [
                    exponential * (1.0 + natural_frequency * delta_time),
                    exponential * delta_time,
                ],
                [
                    -exponential * natural_frequency * natural_frequency * delta_time,
                    exponential * (1.0 - natural_frequency * delta_time),
                ],
            ]
        }
    }

    pub fn is_settled(&self, state: SpringState, target: f32, epsilon: f32) -> bool {
        let (natural_frequency, _) = self.canonical();
        epsilon.is_finite()
            && epsilon >= 0.0
            && (state.position - target).abs() <= epsilon
            && state.velocity.abs() <= epsilon * natural_frequency
    }
}

/// How far something pulled `over` pixels past where it ends stretches: freely at first, then ever
/// less, never as far as `reach`.
pub fn rubber(over: f32, reach: f32) -> f32 {
    over.signum() * reach * (1.0 - 1.0 / (over.abs() * 0.55 / reach + 1.0))
}

/// The pull that stretches it `stretch`, so a pull can carry on from a stretch already showing.
pub fn unrubber(stretch: f32, reach: f32) -> f32 {
    let held = (stretch.abs() / reach).min(0.999);
    stretch.signum() * reach / 0.55 * (1.0 / (1.0 - held) - 1.0)
}

#[derive(Clone, Copy, Debug)]
pub struct Spring {
    state: SpringState,
    target: f32,
    config: SpringConfig,
    epsilon: f32,
    hold_until: Option<Instant>,
}

impl Spring {
    pub fn new(value: f32, config: SpringConfig, epsilon: f32) -> Self {
        Self {
            state: SpringState {
                position: value,
                velocity: 0.0,
            },
            target: value,
            config,
            epsilon,
            hold_until: None,
        }
    }

    pub fn retarget(&mut self, target: f32, config: SpringConfig) {
        self.target = target;
        self.config = config;
        self.hold_until = None;
    }

    pub fn retarget_at(&mut self, target: f32, config: SpringConfig, start: Instant) {
        self.retarget(target, config);
        self.hold_until = Some(start);
    }

    pub fn reset(&mut self, value: f32) {
        self.state = SpringState {
            position: value,
            velocity: 0.0,
        };
        self.target = value;
        self.hold_until = None;
    }

    /// Moves it and where it is heading by the same amount, so it carries on as it was.
    pub fn shift(&mut self, by: f32) {
        self.state.position += by;
        self.target += by;
    }

    /// Adds to its speed, such as a knock that sets it moving.
    pub fn kick(&mut self, velocity: f32) {
        self.state.velocity += velocity;
        self.hold_until = None;
    }

    pub fn value(&self) -> f32 {
        self.state.position
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn is_settled(&self) -> bool {
        self.state.position == self.target && self.state.velocity == 0.0
    }

    /// The fastest it will go on its way from here: its speed and how far it has left to go, so a
    /// swing reads the same at its turn as through its middle.
    pub fn motion(&self) -> f32 {
        let pull = self.config.stiffness / self.config.mass;
        let left = self.target - self.state.position;
        (self.state.velocity.powi(2) + pull * left.powi(2)).sqrt()
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        if self.is_settled() || self.hold_until.is_some_and(|start| now < start) {
            return;
        }
        self.hold_until = None;
        self.state = self.config.step(self.state, self.target, dt);
        if self
            .config
            .is_settled(self.state, self.target, self.epsilon)
        {
            self.state = SpringState {
                position: self.target,
                velocity: 0.0,
            };
        }
    }
}

#[cfg(test)]
#[path = "spring_test.rs"]
mod tests;
