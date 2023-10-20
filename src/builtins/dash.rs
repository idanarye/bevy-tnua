use bevy::prelude::*;

use crate::util::ProjectionPlaneForRotation;
use crate::{
    prelude::*, TnuaActionContext, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus, TnuaMotor,
};

#[derive(Clone)]
pub struct TnuaBuiltinDash {
    pub displacement: Vec3,
    pub desired_forward: Vec3,
    pub allow_in_air: bool,
    pub speed: f32,
    pub brake_to_speed: f32,
    pub acceleration: f32,
    pub brake_acceleration: f32,
    pub input_buffer_time: f32,
}

impl Default for TnuaBuiltinDash {
    fn default() -> Self {
        Self {
            displacement: Vec3::ZERO,
            desired_forward: Vec3::ZERO,
            allow_in_air: false,
            speed: 80.0,
            brake_to_speed: 20.0,
            acceleration: 400.0,
            brake_acceleration: 200.0,
            input_buffer_time: 0.2,
        }
    }
}

impl TnuaAction for TnuaBuiltinDash {
    const NAME: &'static str = "TnuaBuiltinStraightDash";
    type State = TnuaBuiltinDashState;
    const VIOLATES_COYOTE_TIME: bool = true;

    fn initiation_decision(
        &self,
        ctx: crate::TnuaActionContext,
        being_fed_for: &bevy::time::Stopwatch,
    ) -> crate::TnuaActionInitiationDirective {
        if !self.displacement.is_finite() || self.displacement == Vec3::ZERO {
            TnuaActionInitiationDirective::Reject
        } else if self.allow_in_air || !ctx.basis.is_airborne() {
            // Either not airborne, or air jumps are allowed
            TnuaActionInitiationDirective::Allow
        } else if being_fed_for.elapsed().as_secs_f32() < self.input_buffer_time {
            TnuaActionInitiationDirective::Delay
        } else {
            TnuaActionInitiationDirective::Reject
        }
    }

    fn apply(
        &self,
        state: &mut Self::State,
        ctx: TnuaActionContext,
        _lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        // TODO: Once `std::mem::variant_count` gets stabilized, use that instead.
        for _ in 0..3 {
            return match state {
                TnuaBuiltinDashState::PreDash => {
                    // Probably unneeded because of the `initiation_decision`, but still
                    if !self.displacement.is_finite() || self.displacement == Vec3::ZERO {
                        return TnuaActionLifecycleDirective::Finished;
                    }
                    *state = TnuaBuiltinDashState::During {
                        direction: self.displacement.normalize(),
                        destination: ctx.tracker.translation + self.displacement,
                        desired_forward: self.desired_forward,
                        consider_blocked_if_speed_is_less_than: f32::NEG_INFINITY,
                    };
                    continue;
                }
                TnuaBuiltinDashState::During {
                    direction,
                    destination,
                    desired_forward,
                    consider_blocked_if_speed_is_less_than,
                } => {
                    let distance_to_destination =
                        direction.dot(*destination - ctx.tracker.translation);
                    if distance_to_destination < 0.0 {
                        *state = TnuaBuiltinDashState::Braking {
                            direction: *direction,
                        };
                        continue;
                    }

                    let current_speed = direction.dot(ctx.tracker.velocity);
                    if current_speed < *consider_blocked_if_speed_is_less_than {
                        return TnuaActionLifecycleDirective::Finished;
                    }

                    motor.lin = Default::default();
                    motor.lin.acceleration = -ctx.tracker.gravity;
                    motor.lin.boost = (*direction * self.speed - ctx.tracker.velocity)
                        .clamp_length_max(ctx.frame_duration * self.acceleration);
                    let expected_speed = direction.dot(ctx.tracker.velocity + motor.lin.boost);
                    *consider_blocked_if_speed_is_less_than = if current_speed < expected_speed {
                        0.5 * (current_speed + expected_speed)
                    } else {
                        0.5 * current_speed
                    };

                    if 0.0 < desired_forward.length_squared() {
                        let up = ctx.basis.up_direction();
                        let projection =
                            ProjectionPlaneForRotation::from_up_using_default_forward(up);
                        let current_forward = ctx.tracker.rotation.mul_vec3(projection.forward);
                        let rotation_along_up_axis = projection
                            .rotation_to_set_forward(current_forward, self.desired_forward);
                        let desired_angvel = rotation_along_up_axis / ctx.frame_duration;
                        let existing_angvel = ctx.tracker.angvel.dot(up);
                        let torque_to_turn = desired_angvel - existing_angvel;
                        motor.ang.cancel_on_axis(up);
                        motor.ang.boost += torque_to_turn * up;
                    }

                    TnuaActionLifecycleDirective::StillActive
                }
                TnuaBuiltinDashState::Braking { direction } => {
                    let remaining_speed = direction.dot(ctx.tracker.velocity);
                    if remaining_speed <= self.brake_to_speed {
                        TnuaActionLifecycleDirective::Finished
                    } else {
                        motor.lin.boost = -*direction
                            * (remaining_speed - self.brake_to_speed).min(self.brake_acceleration);
                        TnuaActionLifecycleDirective::StillActive
                    }
                }
            };
        }
        error!("Tnua could not decide on dash state");
        TnuaActionLifecycleDirective::Finished
    }
}

#[derive(Default)]
pub enum TnuaBuiltinDashState {
    #[default]
    PreDash,
    During {
        direction: Vec3,
        destination: Vec3,
        desired_forward: Vec3,
        consider_blocked_if_speed_is_less_than: f32,
    },
    Braking {
        direction: Vec3,
    },
}
