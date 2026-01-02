use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::math::{AdjustPrecision, AsF32, Float};
use serde::{Deserialize, Serialize};

use crate::basis_capabilities::TnuaBasisWithGround;
use crate::util::MotionHelper;
use crate::{
    TnuaAction, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus, TnuaMotor, math::Vector3,
};
use crate::{TnuaActionContext, TnuaBasis};

/// An [action](TnuaAction) for climbing on things.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TnuaBuiltinClimb {
    /// A point on the climbed entity where the character touches it.
    ///
    /// Note that this does not actually have to be on any actual collider. It can be a point
    /// in the middle of the air, and the action will cause the character to pretend there is something there and climb on it.
    pub anchor: Vector3,

    /// The position of the [`anchor`](Self::anchor) compared to the character.
    ///
    /// The action will try to maintain this horizontal relative position.
    pub desired_vec_to_anchor: Vector3,

    /// The direction (in the world space) and speed to climb at (move up/down the entity)
    pub desired_climb_motion: Vector3,

    /// Force the character to face in a particular direction.
    pub desired_forward: Option<Dir3>,

    /// Prevent the character from climbing above this point.
    ///
    /// Tip: use
    /// [`probe_extent_from_closest_point`](crate::radar_lens::TnuaRadarBlipLens::probe_extent_from_closest_point)
    /// to find this point.
    pub hard_stop_up: Option<Vector3>,

    /// Prevent the character from climbing below this point.
    ///
    /// Tip: use
    /// [`probe_extent_from_closest_point`](crate::radar_lens::TnuaRadarBlipLens::probe_extent_from_closest_point)
    /// to find this point.
    pub hard_stop_down: Option<Vector3>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TnuaBuiltinClimbConfig {
    /// Speed for maintaining [`desired_vec_to_anchor`](TnuaBuiltinClimb::desired_vec_to_anchor).
    pub anchor_speed: Float,

    /// Acceleration for maintaining
    /// [`desired_vec_to_anchor`](TnuaBuiltinClimb::desired_vec_to_anchor).
    pub anchor_acceleration: Float,

    // How fast the character will climb.
    //
    // Note that this will be the speed when
    // [`desired_climb_motion`](TnuaBuiltinClimb::desired_climb_motion) is a unit vector - meaning
    // that its length is 1.0. If its not 1.0, the speed will be a multiply of that length.
    pub climb_speed: Float,

    /// The acceleration to climb at.
    pub climb_acceleration: Float,

    /// The time, in seconds, the character can still jump after letting go.
    pub coyote_time: Float,
}

impl Default for TnuaBuiltinClimbConfig {
    fn default() -> Self {
        Self {
            anchor_speed: 150.0,
            anchor_acceleration: 500.0,
            climb_speed: 10.0,
            climb_acceleration: 30.0,
            coyote_time: 0.15,
        }
    }
}

impl<B: TnuaBasis> TnuaAction<B> for TnuaBuiltinClimb
where
    B: TnuaBasisWithGround,
{
    type Config = TnuaBuiltinClimbConfig;
    type Memory = TnuaBuiltinClimbMemory;

    // const VIOLATES_COYOTE_TIME: bool = true;

    fn initiation_decision(
        &self,
        _config: &Self::Config,
        _sensors: &B::Sensors<'_>,
        _ctx: TnuaActionContext<B>,
        _being_fed_for: &bevy::time::Stopwatch,
    ) -> TnuaActionInitiationDirective {
        TnuaActionInitiationDirective::Allow
    }

    fn apply(
        &self,
        config: &Self::Config,
        memory: &mut Self::Memory,
        _sensors: &B::Sensors<'_>,
        ctx: TnuaActionContext<B>,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        // TODO: Once `std::mem::variant_count` gets stabilized, use that instead. The idea is to
        // allow jumping through multiple states but failing if we get into loop.
        for _ in 0..2 {
            return match memory {
                TnuaBuiltinClimbMemory::Climbing { climbing_velocity } => {
                    if matches!(lifecycle_status, TnuaActionLifecycleStatus::NoLongerFed) {
                        *memory = TnuaBuiltinClimbMemory::Coyote(Timer::from_seconds(
                            config.coyote_time.f32(),
                            TimerMode::Once,
                        ));
                        continue;
                    }

                    // TODO: maybe this should try to predict the next-frame velocity? Is there a
                    // point?
                    *climbing_velocity = ctx
                        .tracker
                        .velocity
                        .project_onto(ctx.up_direction.adjust_precision());

                    motor
                        .lin
                        .cancel_on_axis(ctx.up_direction.adjust_precision());
                    motor.lin += ctx.negate_gravity();
                    motor.lin += ctx.adjust_vertical_velocity(
                        config.climb_speed
                            * self
                                .desired_climb_motion
                                .dot(ctx.up_direction.adjust_precision()),
                        config.climb_acceleration,
                    );

                    if let Some(stop_at) = self.hard_stop_up {
                        motor.lin += ctx.hard_stop(ctx.up_direction, stop_at, &motor.lin);
                    }
                    if let Some(stop_at) = self.hard_stop_down {
                        motor.lin += ctx.hard_stop(-ctx.up_direction, stop_at, &motor.lin);
                    }

                    let vec_to_anchor = (self.anchor - ctx.tracker.translation)
                        .reject_from(ctx.up_direction().adjust_precision());
                    let horizontal_displacement = self.desired_vec_to_anchor - vec_to_anchor;

                    let desired_horizontal_velocity = -horizontal_displacement / ctx.frame_duration;

                    motor.lin += ctx.adjust_horizontal_velocity(
                        desired_horizontal_velocity.clamp_length_max(config.anchor_speed),
                        config.anchor_acceleration,
                    );

                    if let Some(desired_forward) = self.desired_forward {
                        motor
                            .ang
                            .cancel_on_axis(ctx.up_direction.adjust_precision());
                        motor.ang += ctx.turn_to_direction(desired_forward, ctx.up_direction);
                    }

                    lifecycle_status.directive_simple()
                }
                TnuaBuiltinClimbMemory::Coyote(timer) => {
                    if timer.tick(ctx.frame_duration_as_duration()).is_finished() {
                        TnuaActionLifecycleDirective::Finished
                    } else {
                        lifecycle_status.directive_linger()
                    }
                }
            };
        }
        error!("Tnua could not decide on climb state");
        TnuaActionLifecycleDirective::Finished
    }

    fn influence_basis(
        &self,
        _config: &Self::Config,
        _memory: &Self::Memory,
        _ctx: crate::TnuaBasisContext,
        _basis_input: &B,
        _basis_config: &<B as TnuaBasis>::Config,
        basis_memory: &mut <B as TnuaBasis>::Memory,
    ) {
        B::violate_coyote_time(basis_memory);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TnuaBuiltinClimbMemory {
    Climbing { climbing_velocity: Vector3 },
    Coyote(Timer),
}

impl Default for TnuaBuiltinClimbMemory {
    fn default() -> Self {
        Self::Climbing {
            climbing_velocity: Vector3::ZERO,
        }
    }
}
