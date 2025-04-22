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

    /// The time, in seconds, the character can still jump after letting go.
    pub coyote_time: Float,

    pub desired_forward: Option<Dir3>,

    pub hard_stop_up: Option<Vector3>,
    pub hard_stop_down: Option<Vector3>,

    /// The direction used to initiate the climb.
    ///
    /// This field is not used by the action itself. It's purpose is to help user controller
    /// systems determine if the player input is a continuation of the motion used to initiate the
    /// climb, or if it's a motion for breaking from the climb.
    pub initiation_direction: Vector3,
}

impl Default for TnuaBuiltinClimb {
    fn default() -> Self {
        Self {
            climbable_entity: None,
            anchor: Vector3::NAN,
            desired_vec_to_anchor: Vector3::ZERO,
            anchor_velocity: 150.0,
            anchor_acceleration: 500.0,
            desired_climb_velocity: Vector3::ZERO,
            climb_acceleration: 30.0,
            coyote_time: 0.15,
            desired_forward: None,
            hard_stop_up: None,
            hard_stop_down: None,
            initiation_direction: Vector3::ZERO,
        }
    }
}

impl TnuaAction for TnuaBuiltinClimb {
    const NAME: &'static str = "TnuaBuiltinClimb";

    type State = TnuaBuiltinClimbState;

    const VIOLATES_COYOTE_TIME: bool = true;

    fn apply(
        &self,
        state: &mut Self::State,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        // TODO: Once `std::mem::variant_count` gets stabilized, use that instead. The idea is to
        // allow jumping through multiple states but failing if we get into loop.
        for _ in 0..2 {
            return match state {
                TnuaBuiltinClimbState::Climbing { climbing_velocity } => {
                    if matches!(lifecycle_status, TnuaActionLifecycleStatus::NoLongerFed) {
                        *state = TnuaBuiltinClimbState::Coyote(Timer::from_seconds(
                            self.coyote_time as f32,
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
                        self.desired_climb_velocity
                            .dot(ctx.up_direction.adjust_precision()),
                        self.climb_acceleration,
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
                        desired_horizontal_velocity.clamp_length_max(self.anchor_velocity),
                        self.anchor_acceleration,
                    );

                    if let Some(desired_forward) = self.desired_forward {
                        motor
                            .ang
                            .cancel_on_axis(ctx.up_direction.adjust_precision());
                        motor.ang += ctx.turn_to_direction(desired_forward, ctx.up_direction);
                    }

                    lifecycle_status.directive_simple()
                }
                TnuaBuiltinClimbState::Coyote(timer) => {
                    if timer.tick(ctx.frame_duration_as_duration()).finished() {
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

    fn initiation_decision(
        &self,
        _ctx: TnuaActionContext,
        _being_fed_for: &bevy::time::Stopwatch,
    ) -> TnuaActionInitiationDirective {
        TnuaActionInitiationDirective::Allow
    }

    fn target_entity(&self, _state: &Self::State) -> Option<Entity> {
        self.climbable_entity
    }
}

#[derive(Debug)]
pub enum TnuaBuiltinClimbState {
    Climbing { climbing_velocity: Vector3 },
    Coyote(Timer),
}

impl Default for TnuaBuiltinClimbState {
    fn default() -> Self {
        Self::Climbing {
            climbing_velocity: Vector3::ZERO,
        }
    }
}
