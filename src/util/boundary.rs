use std::time::Duration;

use bevy::prelude::*;

use crate::math::{AdjustPrecision, AsF32, Float, Vector3};

/// Use this to create velocity boundaries for implementing the _Pushover_ feature.
///
/// A Pushover happens when the character receives an external impulse or force other than gravity.
/// The typical acceleration required for tight and responsive control can usually neutralize such
/// impulses too quickly, ruining the knockback effect the game is trying to achieve.
///
/// To work around this, the basis/action can use a `VelocityBoundaryTracker`. It needs to use the
/// [`update`](Self::update) method to detect these external impulses by comparing the velocity the
/// basis/action was trying to achieve with the actual velocity of the character. Then it can check
/// for a velocity [`boundary`](Self::boundary()) and if one exists - use its
/// [`calc_boost_part_on_boundary_axis_after_limit`](VelocityBoundary::calc_boost_part_on_boundary_axis_after_limit)
/// method to determine how to limit the character acceleration according to that boundary.
#[derive(Default)]
pub struct VelocityBoundaryTracker {
    boundary: Option<VelocityBoundary>,
}

impl VelocityBoundaryTracker {
    /// Call this every frame to detect disruptions and track boundary clearing.
    ///
    /// Call this every frame to update the velocity boundary. This method will create a velocity
    /// boundary when there is a "disruption" in the velocity (which indicates an external impulse)
    /// and also take care of "clearing" the boundary when it gets "pushed" (the character's actual
    /// velocity goes past the boundary). It also removes the boundary when it does not get pushed
    /// for too long.
    ///
    /// This method does not apply the boundary - it only updates it. To apply the boundary,
    /// retrieve it with the [`boundary`](Self::boundary) method and use the [`VelocityBoundary`]
    /// object to alter the acceleration.
    ///
    /// # Arguments:
    ///
    /// * `true_velocity` - the velocity as reported by the physics backend. This is the data
    ///   tracked in the [`TnuaRigidBodyTracker`](crate::TnuaRigidBodyTracker), so a typical basis
    ///   or action will get it from
    ///   [`TnuaBasisContext::tracker`](crate::TnuaBasisContext::tracker).
    /// * `disruption_from` - the velocity the entity was _supposed_ to be in, according to the
    ///   calculations of the basis or the action.
    ///   It is up to the caller to determine whether or not there was a disruption. If there was
    ///
    ///   no disruption, this argument should be `None` (but `update` should still be called!).
    ///   `VelocityBoundaryTracker` will make no attempt to determine if the difference in velocity
    ///   really is a disruption.
    /// * `frame_duration` - the duration of the current frame, in seconds.
    /// * `no_push_timeout` - a duration in seconds to keep the boundary alive when it is not
    ///   actively pushed.
    pub fn update(
        &mut self,
        true_velocity: Vector3,
        disruption_from: Option<Vector3>,
        frame_duration: Float,
        no_push_timeout: f32,
    ) {
        'create_boundary: {
            let Some(disruption_from) = disruption_from else {
                break 'create_boundary;
            };
            let Ok(disruption_direction) = Dir3::new((true_velocity - disruption_from).f32())
            else {
                break 'create_boundary;
            };
            let frontier = true_velocity.dot(disruption_direction.adjust_precision());
            self.boundary = Some(VelocityBoundary {
                base: disruption_from.dot(disruption_direction.adjust_precision()),
                original_frontier: frontier,
                frontier,
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
                .tick(Duration::from_secs_f32(frame_duration.f32()))
                .finished()
            {
                self.boundary = None;
            }
        }
    }

    /// Retrieve the current velocity boundary if there is one.
    ///
    /// Make sure to call [`update`](Self::update) every frame to keep the bounady up-to-date.
    pub fn boundary(&self) -> Option<&VelocityBoundary> {
        self.boundary.as_ref()
    }
}

/// An indication that a character was knocked back and "struggles" to get back to its original
/// velocity.
///
/// Typically created in [`VelocityBoundaryTracker`].
pub struct VelocityBoundary {
    base: Float,
    original_frontier: Float,
    frontier: Float,
    direction: Dir3,
    no_push_timer: Timer,
}

impl VelocityBoundary {
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
