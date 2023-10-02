use bevy::prelude::*;

use crate::basis_action_traits::{
    TnuaActionContext, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus,
};
use crate::control_helpers::TnuaCrouchEnforcedAction;
use crate::{TnuaAction, TnuaMotor, TnuaVelChange};

use super::TnuaBuiltinWalk;

#[derive(Clone)]
pub struct TnuaBuiltinCrouch {
    pub float_offset: f32,
    pub height_change_impulse_for_duration: f32,
    pub height_change_impulse_limit: f32,
    pub uncancellable: bool,
}

impl TnuaAction for TnuaBuiltinCrouch {
    const NAME: &'static str = "TnuaBuiltinCrouch";

    type State = TnuaBuiltinCrouchState;

    fn initiation_decision(
        &self,
        ctx: TnuaActionContext,
        _being_fed_for: &bevy::time::Stopwatch,
    ) -> TnuaActionInitiationDirective {
        if ctx.proximity_sensor.output.is_some() {
            TnuaActionInitiationDirective::Allow
        } else {
            TnuaActionInitiationDirective::Delay
        }
    }

    fn apply(
        &self,
        state: &mut Self::State,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        let Some((walk_basis, walk_state)) = ctx.basis_and_state::<TnuaBuiltinWalk>() else {
            error!("Cannot crouch - basis is not TnuaBuiltinWalk");
            return TnuaActionLifecycleDirective::Finished;
        };
        let Some(sensor_output) = &ctx.proximity_sensor.output else {
            return TnuaActionLifecycleDirective::Reschedule { after_seconds: 0.0 };
        };
        let spring_offset_up = walk_basis.float_height - sensor_output.proximity;
        let spring_offset_down = spring_offset_up + self.float_offset;

        match lifecycle_status {
            TnuaActionLifecycleStatus::Initiated => {}
            TnuaActionLifecycleStatus::CancelledFrom => {}
            TnuaActionLifecycleStatus::StillFed => {}
            TnuaActionLifecycleStatus::NoLongerFed => {
                *state = TnuaBuiltinCrouchState::Rising;
            }
            TnuaActionLifecycleStatus::CancelledInto => {
                if !self.uncancellable {
                    *state = TnuaBuiltinCrouchState::Rising;
                }
            }
        }

        let spring_force_boost = |spring_offset: f32| -> f32 {
            walk_basis.spring_force_boost(walk_state, &ctx.as_basis_context(), spring_offset)
        };

        let impulse_or_spring_force_boost = |spring_offset: f32| -> f32 {
            let spring_force_boost = spring_force_boost(spring_offset);
            let impulse_boost = self.impulse_boost(spring_offset);
            if spring_force_boost.abs() < impulse_boost.abs() {
                impulse_boost
            } else {
                spring_force_boost
            }
        };

        let mut set_impulse = |impulse: f32| {
            motor.lin.cancel_on_axis(walk_basis.up);
            motor.lin += TnuaVelChange::boost(impulse * walk_basis.up);
        };

        match state {
            TnuaBuiltinCrouchState::Sinking => {
                if spring_offset_down < -0.01 {
                    set_impulse(impulse_or_spring_force_boost(spring_offset_down));
                } else {
                    *state = TnuaBuiltinCrouchState::Maintaining;
                    set_impulse(spring_force_boost(spring_offset_down));
                }
                lifecycle_status.directive_simple()
            }
            TnuaBuiltinCrouchState::Maintaining => {
                set_impulse(spring_force_boost(spring_offset_down));
                // If it's finished/cancelled, something else should changed its state
                TnuaActionLifecycleDirective::StillActive
            }
            TnuaBuiltinCrouchState::Rising => {
                if 0.01 < spring_offset_up {
                    set_impulse(impulse_or_spring_force_boost(spring_offset_up));

                    // TODO: maybe this decision should be smarter, and based on
                    // `TnuaKeepCrouchingBelowObstacles`?
                    if matches!(lifecycle_status, TnuaActionLifecycleStatus::CancelledInto) {
                        // Don't finish the rise - just do the other action
                        TnuaActionLifecycleDirective::Reschedule { after_seconds: 0.0 }
                    } else {
                        // Finish the rise
                        TnuaActionLifecycleDirective::StillActive
                    }
                } else {
                    TnuaActionLifecycleDirective::Finished
                }
            }
        }
    }
}

impl TnuaBuiltinCrouch {
    fn impulse_boost(&self, spring_offset: f32) -> f32 {
        let velocity_to_get_to_new_float_height =
            spring_offset / self.height_change_impulse_for_duration;
        velocity_to_get_to_new_float_height.clamp(
            -self.height_change_impulse_limit,
            self.height_change_impulse_limit,
        )
    }
}

#[derive(Default, Debug)]
pub enum TnuaBuiltinCrouchState {
    #[default]
    Sinking,
    Maintaining,
    Rising,
}

impl TnuaCrouchEnforcedAction for TnuaBuiltinCrouch {
    fn range_to_cast_up(&self, _state: &Self::State) -> f32 {
        -self.float_offset
    }

    fn prevent_cancellation(&mut self) {
        self.uncancellable = true;
    }
}
