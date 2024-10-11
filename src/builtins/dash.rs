use crate::math::{AdjustPrecision, Float, Vector3};
use bevy::prelude::*;

use crate::util::rotation_arc_around_axis;
use crate::{
    prelude::*, TnuaActionContext, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus, TnuaMotor,
};

/// The basic dash [action](TnuaAction).
#[derive(Clone)]
pub struct TnuaBuiltinDash {
    /// The direction and distance of the dash.
    ///
    /// This input parameter is cached when the action starts. This means that the control system
    /// does not have to make sure the direction reamins the same even if the player changes it
    /// mid-dash.
    pub displacement: Vector3,

    /// Point the negative Z axis of the characetr model in that direction during the dash.
    ///
    /// This input parameter is cached when the action starts. This means that the control system
    /// does not have to make sure the direction reamins the same even if the player changes it
    /// mid-dash.
    pub desired_forward: Option<Dir3>,

    /// Allow this action to start even if the character is not touching ground nor in coyote time.
    pub allow_in_air: bool,

    /// The speed the character will move in during the dash.
    pub speed: Float,

    /// After the dash, the character will brake until its speed is below that number.
    pub brake_to_speed: Float,

    /// The maximum acceleration when starting the jump.
    pub acceleration: Float,

    /// The maximum acceleration when braking after the jump.
    ///
    /// Irrelevant if [`brake_to_speed`](Self::brake_to_speed) is set to infinity.
    pub brake_acceleration: Float,

    /// A duration, in seconds, where a player can press a dash button before a dash becomes
    /// possible (typically when a character is still in the air and about the land) and the dash
    /// action would still get registered and be executed once the dash is possible.
    pub input_buffer_time: Float,
}

impl Default for TnuaBuiltinDash {
    fn default() -> Self {
        Self {
            displacement: Vector3::ZERO,
            desired_forward: None,
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
        if !self.displacement.is_finite() || self.displacement == Vector3::ZERO {
            TnuaActionInitiationDirective::Reject
        } else if self.allow_in_air || !ctx.basis.is_airborne() {
            // Either not airborne, or air jumps are allowed
            TnuaActionInitiationDirective::Allow
        } else if (being_fed_for.elapsed().as_secs_f64() as Float) < self.input_buffer_time {
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
                    if !self.displacement.is_finite() || self.displacement == Vector3::ZERO {
                        return TnuaActionLifecycleDirective::Finished;
                    }
                    *state = TnuaBuiltinDashState::During {
                        direction: self.displacement.normalize(),
                        destination: ctx.tracker.translation + self.displacement,
                        desired_forward: self.desired_forward,
                        consider_blocked_if_speed_is_less_than: Float::NEG_INFINITY,
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

                    if let Some(desired_forward) = desired_forward {
                        let up = ctx.up_direction;
                        let up = up.adjust_precision();
                        let current_forward = ctx.tracker.rotation.mul_vec3(Vector3::NEG_Z);
                        let rotation_along_up_axis = rotation_arc_around_axis(
                            Dir3::Y,
                            current_forward,
                            desired_forward.adjust_precision(),
                        )
                        .unwrap_or(0.0);
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
        direction: Vector3,
        destination: Vector3,
        desired_forward: Option<Dir3>,
        consider_blocked_if_speed_is_less_than: Float,
    },
    Braking {
        direction: Vector3,
    },
}
