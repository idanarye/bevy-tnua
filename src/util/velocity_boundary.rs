use std::time::Duration;

use crate::math::{AdjustPrecision, AsF32, Float, Vector3};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// An indication that a character was knocked back and "struggles" to get back to its original
/// velocity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VelocityBoundary {
    base: Float,
    original_frontier: Float,
    frontier: Float,
    pub direction: Dir3,
    no_push_timer: Timer,
}

impl VelocityBoundary {
    pub fn new(
        disruption_from: Vector3,
        disruption_to: Vector3,
        no_push_timeout: f32,
    ) -> Option<Self> {
        let Ok(disruption_direction) = Dir3::new((disruption_to - disruption_from).f32()) else {
            return None;
        };
        let frontier = disruption_to.dot(disruption_direction.adjust_precision());
        Some(Self {
            base: disruption_from.dot(disruption_direction.adjust_precision()),
            original_frontier: frontier,
            frontier,
            direction: disruption_direction,
            no_push_timer: Timer::from_seconds(no_push_timeout, TimerMode::Once),
        })
    }

    /// Call this every frame to update the velocity boundary.
    ///
    /// This methos takes care of "clearing" the boundary when it gets "pushed" (the character's
    /// actual velocity goes past the boundary).
    ///
    /// This method does not detect when the boundary is cleared - use
    /// [`is_cleared`](Self::is_cleared) for that purpose
    ///
    /// This method does not apply the boundary - it only updates it. To apply the boundary, use
    /// [`calc_boost_part_on_boundary_axis_after_limit`](Self::calc_boost_part_on_boundary_axis_after_limit)
    /// to determine how to alter the acceleration.
    ///
    /// # Arguments:
    ///
    /// * `velocity` - the velocity as reported by the physics backend. This is the data tracked in
    ///   the [`TnuaRigidBodyTracker`](crate::TnuaRigidBodyTracker), so a typical basis or action
    ///   will get it from [`TnuaBasisContext::tracker`](crate::TnuaBasisContext::tracker).
    /// * `frame_duration` - the duration of the current frame, in seconds.
    pub fn update(&mut self, velocity: Vector3, frame_duration: Duration) {
        let new_frontier = velocity.dot(self.direction.adjust_precision());
        if new_frontier < self.frontier {
            self.frontier = new_frontier;
            self.no_push_timer.reset();
        } else {
            self.no_push_timer.tick(frame_duration);
        }
    }

    pub fn is_cleared(&self) -> bool {
        self.no_push_timer.is_finished() || self.frontier <= self.base
    }

    /// Calculate how a boost needs to be adjusted according to the boundary.
    ///
    /// Note that the returned value is the boost limit only on the axis of the returned direction.
    /// The other axes should remain the same (unless the caller has a good reason to modify them).
    /// The reason why this method doesn't simply return the final boost is that the caller may be
    /// using [`TnuaVelChange`](crate::TnuaVelChange) which combines acceleration and impulse, and
    /// if so then it is the caller's responsibility to amend the result of this method to match
    /// that scheme.
    ///
    /// # Arguments:
    ///
    /// * `current_velocity` - the velocity of the character **before the boost**.
    /// * `regular_boost` - the boost that the caller would have applied to the character before
    ///   taking the boundary into account.
    /// * `boost_limit_inside_barrier` - the maximum boost allowed inside a fully strength barrier,
    ///   assuming it goes directly against the direction of the boundary.
    /// * `barrier_strength_diminishing` - an exponent describing how the boundary strength
    ///   diminishes when the barrier gets cleared. For best results, set it to values larger than
    ///   1.0.
    pub fn calc_boost_part_on_boundary_axis_after_limit(
        &self,
        current_velocity: Vector3,
        regular_boost: Vector3,
        boost_limit_inside_barrier: Float,
        barrier_strength_diminishing: Float,
    ) -> Option<(Dir3, Float)> {
        let boost = regular_boost.dot(self.direction.adjust_precision());
        if 0.0 <= boost {
            // Not pushing the barrier
            return None;
        }
        let current = current_velocity.dot(self.direction.adjust_precision());
        let after_boost = current + boost;
        if self.frontier <= after_boost {
            return None;
        }
        let boost_before_barrier = (current - self.frontier).max(0.0);
        let fraction_before_frontier = boost_before_barrier / -boost;
        let fraction_after_frontier = 1.0 - fraction_before_frontier;
        let push_inside_barrier = fraction_after_frontier * boost_limit_inside_barrier;
        let barrier_depth = self.frontier - self.base;
        if barrier_depth <= 0.0 {
            return None;
        }
        let fraction_inside_barrier = if push_inside_barrier <= barrier_depth {
            fraction_after_frontier
        } else {
            barrier_depth / boost_limit_inside_barrier
        }
        .clamp(0.0, 1.0);

        let boost_outside_barrier = (1.0 - fraction_inside_barrier) * boost;
        // Make it negative here, because this is the one that pushes against the barrier
        let boost_inside_barrier = fraction_inside_barrier * -boost_limit_inside_barrier;

        let total_boost = boost_outside_barrier + boost_inside_barrier;

        let barrier_strength = self.percentage_left().powf(barrier_strength_diminishing);
        let total_boost = (1.0 - barrier_strength) * boost + barrier_strength * total_boost;

        Some((-self.direction, -total_boost))
    }

    fn percentage_left(&self) -> Float {
        let current_depth = self.frontier - self.base;
        let original_depth = self.original_frontier - self.base;
        current_depth / original_depth
    }
}
