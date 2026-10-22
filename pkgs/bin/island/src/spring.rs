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
