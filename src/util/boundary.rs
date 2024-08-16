use std::time::Duration;

use bevy::prelude::*;

use crate::math::{AdjustPrecision, Vector3, Float};

#[derive(Default)]
pub struct VelocityBoundaryTracker {
    boundary: Option<VelocityBoundary>,
}

impl VelocityBoundaryTracker {
    pub fn update(
        &mut self,
        true_velocity: Vector3,
        disruption_from: Option<Vector3>,
        frame_duration: f32,
        no_push_timeout: f32,
    ) {
        'create_boundary: {
            let Some(disruption_from) = disruption_from else { break 'create_boundary };
            let Ok(disruption_direction) = Dir3::new(true_velocity - disruption_from) else { break 'create_boundary };
            self.boundary = Some(VelocityBoundary {
                base: disruption_from.dot(disruption_direction.adjust_precision()),
                frontier: true_velocity.dot(disruption_direction.adjust_precision()),
                direction: disruption_direction,
                no_push_timer: Timer::from_seconds(no_push_timeout, TimerMode::Once),
            });
            return;
        };
        if let Some(boundary) = self.boundary.as_mut() {
            let new_frontier = true_velocity.dot(boundary.direction.adjust_precision());
            if new_frontier <= boundary.base {
                self.boundary = None;
            } else if new_frontier < boundary.frontier {
                boundary.frontier = new_frontier;
                boundary.no_push_timer = Timer::from_seconds(no_push_timeout, TimerMode::Once);
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

struct VelocityBoundary {
    base: Float,
    frontier: Float,
    direction: Dir3,
    no_push_timer: Timer,
}
