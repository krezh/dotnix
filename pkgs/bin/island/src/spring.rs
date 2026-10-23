use std::time::Instant;

use gpui::{SpringConfig, SpringState};

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
