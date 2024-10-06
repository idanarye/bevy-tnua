#![allow(unused_imports)]
use crate::{
    math::{AdjustPrecision, Float, Vector3},
    prelude::*,
    util::{
        boundary::{VelocityBoundary, VelocityBoundaryTracker},
        rotation_arc_around_axis,
    },
    TnuaActionContext, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus, TnuaMotor, TnuaVelChange,
};
use bevy::prelude::*;

#[derive(Clone)]
pub struct TnuaBuiltinKnockback {
    pub shove: Vector3,

    /// Timeout (in seconds) for abandoning a Pushover boundary that no longer gets pushed.
    ///
    /// Refer to [`VelocityBoundaryTracker`] for more information about the Pushover feature.
    pub no_push_timeout: f32,

    /// An exponent for controlling the shape of the Pushover barrier diminishing.
    ///
    /// For best results, set it to values larger than 1.0.
    ///
    /// Refer to [`VelocityBoundaryTracker`] for more information about the Pushover feature.
    pub barrier_strength_diminishing: Float,

    /// Acceleration cap when pushing against the Pushover barrier.
    ///
    /// In practice this will be averaged with [`acceleration`](Self::acceleration) (weighted by a
    /// function of the pushover boundary penetration percentage and
    /// [`barrier_strength_diminishing`](Self::barrier_strength_diminishing)) so
    /// the actual acceleration limit will higher than that.
    ///
    /// Refer to [`VelocityBoundaryTracker`] for more information about the Pushover feature.
    pub acceleration_limit: Float,

    /// Acceleration cap when pushing against the Pushover barrier while in the air.
    ///
    /// In practice this will be averaged with [`air_acceleration`](Self::air_acceleration)
    /// (weighted by a function of the pushover boundary penetration percentage and
    /// [`barrier_strength_diminishing`](Self::barrier_strength_diminishing)) so
    /// the actual acceleration limit will higher than that.
    ///
    /// Refer to [`VelocityBoundaryTracker`] for more information about the Pushover feature.
    pub air_acceleration_limit: Float,

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
