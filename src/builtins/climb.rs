use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::math::{AdjustPrecision, Float};

use crate::util::MotionHelper;
use crate::TnuaActionContext;
use crate::{
    math::Vector3, TnuaAction, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus, TnuaMotor,
};

#[derive(Clone)]
pub struct TnuaBuiltinClimb {
    pub climbable_entity: Option<Entity>,
    pub anchor: Vector3,
    pub desired_vec_to_anchor: Vector3,
    pub anchor_velocity: Float,
    pub anchor_acceleration: Float,

    pub desired_climb_velocity: Vector3,
    pub climb_acceleration: Float,

    /// The direction used to initiate the climb.
    ///
    /// This field is not used by the action itself. It's purpose is to help user controller
    /// systems determine if the player input is a continuation of the motion used to initiate the
    /// climb, or if it's a motion for breaking from the climb.
    pub initiation_direction: Vector3,
}

impl TnuaAction for TnuaBuiltinClimb {
    const NAME: &'static str = "TnuaBuiltinClimb";

    type State = TnuaBuiltinClimbState;

    const VIOLATES_COYOTE_TIME: bool = true;

    fn apply(
        &self,
        _state: &mut Self::State,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        motor
            .lin
            .cancel_on_axis(ctx.up_direction.adjust_precision());
        motor.lin += ctx.negate_gravity();
        motor.lin += ctx.adjust_vertical_velocity(
            self.desired_climb_velocity
                .dot(ctx.up_direction.adjust_precision()),
            self.climb_acceleration,
        );

        let vec_to_anchor = (self.anchor - ctx.tracker.translation)
            .reject_from(ctx.up_direction().adjust_precision());
        let horizontal_displacement = self.desired_vec_to_anchor - vec_to_anchor;

        let desired_horizontal_velocity = -horizontal_displacement / ctx.frame_duration;

        motor.lin +=
            ctx.adjust_horizontal_velocity(desired_horizontal_velocity, self.anchor_acceleration);

        lifecycle_status.directive_simple()
    }

    fn initiation_decision(
        &self,
        _ctx: TnuaActionContext,
        _being_fed_for: &bevy::time::Stopwatch,
    ) -> TnuaActionInitiationDirective {
        TnuaActionInitiationDirective::Allow
    }
}

#[derive(Default, Debug)]
pub struct TnuaBuiltinClimbState {}
