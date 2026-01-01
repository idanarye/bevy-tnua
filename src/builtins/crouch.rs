use bevy::time::Stopwatch;
use serde::{Deserialize, Serialize};

use crate::basis_capabilities::{
    TnuaBasisWithFloating, TnuaBasisWithGround, TnuaBasisWithHeadroom, TnuaBasisWithSpring,
};
use crate::{TnuaAction, TnuaActionContext, TnuaBasis};
use crate::{
    TnuaActionInitiationDirective, TnuaActionLifecycleDirective, TnuaActionLifecycleStatus, math::*,
};
use crate::{TnuaMotor, TnuaVelChange};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TnuaBuiltinCrouch;

#[derive(Clone, Serialize, Deserialize)]
pub struct TnuaBuiltinCrouchConfig {
    /// Controls how low the character will crouch, compared to its regular float offset while
    /// standing.
    ///
    /// This field should typically have a negative value. A positive value will cause the
    /// character to "crouch" upward - which may be an interesting gameplay action, but not
    /// what one would call a "crouch".
    pub float_offset: Float,

    /// A duration, in seconds, that it should take for the character to change its floating height
    /// to start or stop the crouch.
    ///
    /// Set this to more than the expected duration of a single frame, so that the character will
    /// some distance for the
    /// [`spring_dampening`](crate::builtins::TnuaBuiltinWalkConfig::spring_dampening) force to
    /// reduce its vertical velocity.
    pub height_change_impulse_for_duration: Float,

    /// The maximum impulse to apply when starting or stopping the crouch.
    pub height_change_impulse_limit: Float,
}

impl Default for TnuaBuiltinCrouchConfig {
    fn default() -> Self {
        Self {
            float_offset: 0.0,
            height_change_impulse_for_duration: 0.02,
            height_change_impulse_limit: 40.0,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub enum TnuaBuiltinCrouchMemory {
    /// The character is transitioning from standing to crouching.
    #[default]
    Sinking,
    /// The character is currently crouched.
    Maintaining,
    /// The character is transitioning from crouching to standing.
    Rising,
}

impl<B: TnuaBasis> TnuaAction<B> for TnuaBuiltinCrouch
where
    B: TnuaBasisWithFloating,
    B: TnuaBasisWithSpring,
    B: TnuaBasisWithGround,
    B: TnuaBasisWithHeadroom,
{
    type Config = TnuaBuiltinCrouchConfig;
    type Memory = TnuaBuiltinCrouchMemory;

    fn initiation_decision(
        &self,
        _config: &Self::Config,
        sensors: &B::Sensors<'_>,
        _ctx: TnuaActionContext<B>,
        _being_fed_for: &Stopwatch,
    ) -> TnuaActionInitiationDirective {
        if B::ground_sensor(sensors).output.is_some() {
            TnuaActionInitiationDirective::Allow
        } else {
            TnuaActionInitiationDirective::Delay
        }
    }

    fn apply(
        &self,
        config: &Self::Config,
        memory: &mut Self::Memory,
        sensors: &B::Sensors<'_>,
        ctx: TnuaActionContext<B>,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> crate::TnuaActionLifecycleDirective {
        let Some(sensor_output) = &B::ground_sensor(sensors).output else {
            return TnuaActionLifecycleDirective::Reschedule { after_seconds: 0.0 };
        };
        let spring_offset_up =
            B::float_height(ctx.basis) - sensor_output.proximity.adjust_precision();
        let spring_offset_down =
            spring_offset_up.adjust_precision() + config.float_offset.adjust_precision();

        match lifecycle_status {
            TnuaActionLifecycleStatus::Initiated => {}
            TnuaActionLifecycleStatus::CancelledFrom => {}
            TnuaActionLifecycleStatus::StillFed => {}
            TnuaActionLifecycleStatus::NoLongerFed => {
                *memory = Self::Memory::Rising;
            }
            TnuaActionLifecycleStatus::CancelledInto => {
                *memory = TnuaBuiltinCrouchMemory::Rising;
            }
        }

        let can_stand = B::headroom_intrusion(ctx.basis, sensors)
            .map(|headroom_intrusion| {
                spring_offset_up < headroom_intrusion.end - headroom_intrusion.start
            })
            .unwrap_or(true);

        if !can_stand && matches!(memory, TnuaBuiltinCrouchMemory::Rising) {
            *memory = TnuaBuiltinCrouchMemory::Maintaining;
        }

        let spring_force = |spring_offset: Float| -> TnuaVelChange {
            B::spring_force(ctx.basis, &ctx.as_basis_context(), spring_offset)
        };

        let impulse_or_spring_force = |spring_offset: Float| -> TnuaVelChange {
            let spring_force = spring_force(spring_offset);
            let spring_force_boost = crate::util::calc_boost(&spring_force, ctx.frame_duration);
            let impulse_boost = config.impulse_boost(spring_offset);
            if spring_force_boost.length_squared() < impulse_boost.powi(2) {
                TnuaVelChange::boost(impulse_boost * ctx.up_direction.adjust_precision())
            } else {
                spring_force
            }
        };

        let mut set_vel_change = |vel_change: TnuaVelChange| {
            motor
                .lin
                .cancel_on_axis(ctx.up_direction.adjust_precision());
            motor.lin += vel_change;
        };

        match memory {
            TnuaBuiltinCrouchMemory::Sinking => {
                if spring_offset_down < -0.01 {
                    set_vel_change(impulse_or_spring_force(spring_offset_down));
                } else {
                    *memory = TnuaBuiltinCrouchMemory::Maintaining;
                    set_vel_change(spring_force(spring_offset_down));
                }
                lifecycle_status.directive_simple()
            }
            TnuaBuiltinCrouchMemory::Maintaining => {
                set_vel_change(spring_force(spring_offset_down));
                // If it's finished/cancelled, something else should changed its state
                TnuaActionLifecycleDirective::StillActive
            }
            TnuaBuiltinCrouchMemory::Rising => {
                if 0.01 < spring_offset_up {
                    set_vel_change(impulse_or_spring_force(spring_offset_up));

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

    fn influence_basis(
        &self,
        config: &Self::Config,
        _memory: &Self::Memory,
        _ctx: crate::TnuaBasisContext,
        _basis_input: &B,
        _basis_config: &<B as TnuaBasis>::Config,
        basis_memory: &mut <B as TnuaBasis>::Memory,
    ) {
        B::set_extra_headroom(basis_memory, -config.float_offset);
    }
}

impl TnuaBuiltinCrouchConfig {
    fn impulse_boost(&self, spring_offset: Float) -> Float {
        let velocity_to_get_to_new_float_height =
            spring_offset / self.height_change_impulse_for_duration;
        velocity_to_get_to_new_float_height.clamp(
            -self.height_change_impulse_limit,
            self.height_change_impulse_limit,
        )
    }
}

// TOOD: this
// impl TnuaCrouchEnforcedAction for TnuaBuiltinCrouch {
// fn range_to_cast_up(&self, _memory: &Self::Memory) -> Float {
// -self.float_offset
// }

// fn prevent_cancellation(&mut self) {
// self.uncancellable = true;
// }
// }
