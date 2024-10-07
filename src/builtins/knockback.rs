use std::time::Duration;

use crate::{
    math::{AdjustPrecision, AsF32, Float, Vector3},
    prelude::*,
    util::rotation_arc_around_axis,
    TnuaActionContext, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus, TnuaMotor, TnuaVelChange,
};
use bevy::prelude::*;

/// Apply this [action](TnuaAction) to shove the character in a way the [basis](crate::TnuaBasis)
/// cannot easily nullify.
///
/// Note that this action cannot be cancelled or stopped. Once it starts, it'll resume until the
/// Pushover boundary is cleared (which means the character overcame the knockback). Unless the
/// parameters are seriously skewed. The main parameters that can mess it up and unreasonably
/// prolong the knockback duration are:
/// * [`no_push_timer`](Self::no_push_timeout). Setting it too high will allow the character to
///   "move along" with the shove, prolonging the knockback action because the boundary does not
///   get cleared. The action will not affect the velocity during that time, but it can still
///   prolong the animation, apply [`force_forward`](Self::force_forward), and prevent other
///   actions from happening.
/// * [`barrier_strength_diminishing`](Self::barrier_strength_diminishing). Setting it too low
///   makes it very hard for the character to push through the boundary. It starts getting slightly
///   weird below 1.0, and really weird below 0.5. Better keep it at above - 1.0 levels.
#[derive(Clone)]
pub struct TnuaBuiltinKnockback {
    /// Initial impulse to apply to the character before the Pushover stage starts.
    ///
    /// It is important that the impulse will be applied using the action (by setting this field)
    /// and not directly via the physics backend so that Tnua can properly calculate the Pushover
    /// boundary based on it.
    pub shove: Vector3,

    /// Timeout (in seconds) for abandoning a Pushover boundary that no longer gets pushed.
    pub no_push_timeout: f32,

    /// An exponent for controlling the shape of the Pushover barrier diminishing.
    ///
    /// For best results, set it to values larger than 1.0.
    pub barrier_strength_diminishing: Float,

    /// Acceleration cap when pushing against the Pushover barrier.
    ///
    /// In practice this will be averaged with the acceleration the basis tries to apply (weighted
    /// by a function of the Pushover boundary penetration percentage and
    /// [`barrier_strength_diminishing`](Self::barrier_strength_diminishing)) so the actual
    /// acceleration limit will higher than that.
    pub acceleration_limit: Float,

    /// Acceleration cap when pushing against the Pushover barrier while in the air.
    ///
    /// In practice this will be averaged with the acceleration the basis tries to apply (weighted
    /// by a function of the Pushover boundary penetration percentage and
    /// [`barrier_strength_diminishing`](Self::barrier_strength_diminishing)) so the actual
    /// acceleration limit will higher than that.
    pub air_acceleration_limit: Float,

    /// Force the character to face in a particular direction.
    ///
    /// Note that there are no acceleration limits because unlike
    /// [TnuaBuiltinWalk::desired_forward] this field will attempt to force the direction during a
    /// single frame. It is useful for when the knockback animation needs to be aligned with the
    /// knockback direction.
    pub force_forward: Option<Dir3>,
}

impl Default for TnuaBuiltinKnockback {
    fn default() -> Self {
        Self {
            shove: Vector3::ZERO,
            no_push_timeout: 0.2,
            barrier_strength_diminishing: 2.0,
            acceleration_limit: 3.0,
            air_acceleration_limit: 1.0,
            force_forward: None,
        }
    }
}

impl TnuaAction for TnuaBuiltinKnockback {
    const NAME: &'static str = "TnuaBuiltinKnockback";
    type State = TnuaBuiltinKnockbackState;
    const VIOLATES_COYOTE_TIME: bool = true;

    fn apply(
        &self,
        state: &mut Self::State,
        ctx: TnuaActionContext,
        _lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        match state {
            TnuaBuiltinKnockbackState::Boost => {
                let Some(boundary) = VelocityBoundary::new(
                    ctx.tracker.velocity,
                    ctx.tracker.velocity + self.shove,
                    self.no_push_timeout,
                ) else {
                    return TnuaActionLifecycleDirective::Finished;
                };
                motor.lin += TnuaVelChange::boost(self.shove);
                *state = TnuaBuiltinKnockbackState::Pushback { boundary };
            }
            TnuaBuiltinKnockbackState::Pushback { boundary } => {
                boundary.update(ctx.tracker.velocity, ctx.frame_duration_as_duration());
                if boundary.is_cleared() {
                    return TnuaActionLifecycleDirective::Finished;
                } else {
                    let regular_boost = motor.lin.calc_boost(ctx.frame_duration);
                    if let Some((component_direction, component_limit)) = boundary
                        .calc_boost_part_on_boundary_axis_after_limit(
                            ctx.tracker.velocity,
                            regular_boost,
                            ctx.frame_duration * self.acceleration_limit,
                            self.barrier_strength_diminishing,
                        )
                    {
                        'limit_vel_change: {
                            let regular = regular_boost.dot(component_direction.adjust_precision());
                            let to_cut = regular - component_limit;
                            if to_cut <= 0.0 {
                                break 'limit_vel_change;
                            }
                            let boost_part =
                                motor.lin.boost.dot(component_direction.adjust_precision());
                            if to_cut <= boost_part {
                                // Can do the entire cut by just reducing the boost
                                motor.lin.boost -= to_cut * component_direction.adjust_precision();
                                break 'limit_vel_change;
                            }
                            // Even nullifying the boost is not enough, and we don't want to
                            // reverse it, so we're going to cut the acceleration as well.
                            motor.lin.boost = motor
                                .lin
                                .boost
                                .reject_from(component_direction.adjust_precision());
                            let to_cut_from_acceleration = to_cut - boost_part;
                            let acceleration_to_cut = to_cut_from_acceleration / ctx.frame_duration;
                            motor.lin.acceleration -=
                                acceleration_to_cut * component_direction.adjust_precision();
                        }
                    }
                }
            }
        }

        if let Some(force_forward) = self.force_forward {
            let current_forward = ctx.tracker.rotation.mul_vec3(Vector3::NEG_Z);
            let rotation_along_up_axis = rotation_arc_around_axis(
                ctx.up_direction(),
                current_forward,
                force_forward.adjust_precision(),
            )
            .unwrap_or(0.0);
            let desired_angvel = rotation_along_up_axis / ctx.frame_duration;

            let existing_angvel = ctx
                .tracker
                .angvel
                .dot(ctx.up_direction().adjust_precision());

            let torque_to_turn = desired_angvel - existing_angvel;

            motor
                .ang
                .cancel_on_axis(ctx.up_direction().adjust_precision());
            motor.ang +=
                TnuaVelChange::boost(torque_to_turn * ctx.up_direction().adjust_precision());
        }

        TnuaActionLifecycleDirective::StillActive
    }

    fn initiation_decision(
        &self,
        _ctx: crate::TnuaActionContext,
        _being_fed_for: &bevy::time::Stopwatch,
    ) -> TnuaActionInitiationDirective {
        TnuaActionInitiationDirective::Allow
    }
}

#[derive(Default)]
pub enum TnuaBuiltinKnockbackState {
    #[default]
    Boost,
    Pushback {
        boundary: VelocityBoundary,
    },
}

/// An indication that a character was knocked back and "struggles" to get back to its original
/// velocity.
pub struct VelocityBoundary {
    base: Float,
    original_frontier: Float,
    frontier: Float,
    pub direction: Dir3,
    no_push_timer: Timer,
}

impl VelocityBoundary {
    fn new(disruption_from: Vector3, disruption_to: Vector3, no_push_timeout: f32) -> Option<Self> {
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
    fn update(&mut self, velocity: Vector3, frame_duration: Duration) {
        let new_frontier = velocity.dot(self.direction.adjust_precision());
        if new_frontier < self.frontier {
            self.frontier = new_frontier;
            self.no_push_timer.reset();
        } else {
            self.no_push_timer.tick(frame_duration);
        }
    }

    fn is_cleared(&self) -> bool {
        self.no_push_timer.finished() || self.frontier <= self.base
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
    fn calc_boost_part_on_boundary_axis_after_limit(
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
