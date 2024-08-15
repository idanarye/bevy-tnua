use std::time::Duration;

use bevy::prelude::*;

use crate::math::{AdjustPrecision, Vector3};

#[derive(Default)]
pub struct BoundaryTracker {
    boundary: Option<Boundary>,
}

impl BoundaryTracker {
    pub fn update(
        &mut self,
        true_velocity: Vector3,
        disruption_from: Option<Vector3>,
        frame_duration: f32,
        no_push_timeout: f32,
    ) {
        let disruption_direction = disruption_from
            .and_then(|disruption_from| Dir3::new(true_velocity - disruption_from).ok());
        if let Some(disruption_direction) = disruption_direction {
            self.boundary = Some(Boundary {
                point: true_velocity,
                direction: disruption_direction,
                no_push_timer: Timer::from_seconds(no_push_timeout, TimerMode::Once),
            });
        } else if let Some(boundary) = self.boundary.as_mut() {
            let dist = (true_velocity - boundary.point).dot(boundary.direction.adjust_precision());
            if dist < 0.0 {
                boundary.point = true_velocity;
                boundary.no_push_timer = Timer::from_seconds(no_push_timeout, TimerMode::Once);
                info!("Boundary pushed {dist}");
            } else if boundary
                .no_push_timer
                .tick(Duration::from_secs_f32(frame_duration))
                .finished()
            {
                self.boundary = None;
            }
        }
    }
}

struct Boundary {
    point: Vector3,
    direction: Dir3,
    no_push_timer: Timer,
}
