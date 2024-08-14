use bevy::prelude::*;

use crate::math::{Float, Vector3};

#[derive(Component)]
pub struct TnuaPushover {
    pub update_factor: Float,
    pub(crate) perceived_velocity: Vector3,
}

impl TnuaPushover {
    pub fn new(update_factor: Float) -> Self {
        Self {
            update_factor,
            perceived_velocity: Vector3::ZERO,
        }
    }

    pub fn predict(&mut self, change_in_velocity: Vector3) {
        self.perceived_velocity += change_in_velocity;
    }

    pub(crate) fn update(&mut self, frame_duration: Float, true_velocity: Vector3) {
        let factor = self.update_factor.powf(1.0 / frame_duration);
        self.perceived_velocity = (1.0 - factor) * self.perceived_velocity + factor * true_velocity;
    }
}
