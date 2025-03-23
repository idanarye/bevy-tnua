use crate::{
    math::{AdjustPrecision, Float, Vector3},
    prelude::*,
    util::{calc_angular_velchange_to_force_forward, VelocityBoundary},
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
#[derive(Clone, Debug)]
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
            TnuaBuiltinKnockbackState::Shove => {
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
                } else if let Some((component_direction, component_limit)) = boundary
                    .calc_boost_part_on_boundary_axis_after_limit(
                        ctx.tracker.velocity,
                        motor.lin.calc_boost(ctx.frame_duration),
                        ctx.frame_duration * self.acceleration_limit,
                        self.barrier_strength_diminishing,
                    )
                {
                    motor.lin.apply_boost_limit(
                        ctx.frame_duration,
                        component_direction,
                        component_limit,
                    );
                }
            }
        }

        if let Some(force_forward) = self.force_forward {
            motor
                .ang
                .cancel_on_axis(ctx.up_direction.adjust_precision());
            motor.ang += calc_angular_velchange_to_force_forward(
                force_forward,
                ctx.tracker.rotation,
                ctx.tracker.angvel,
                ctx.up_direction,
                ctx.frame_duration,
            );
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

#[derive(Default, Clone, Debug)]
pub enum TnuaBuiltinKnockbackState {
    /// Applying the [`shove`](TnuaBuiltinKnockback::shove) impulse to the character.
    #[default]
    Shove,
    /// Hindering the character's ability to overcome the
    /// [`Shove`](TnuaBuiltinKnockbackState::Shove) while waiting for it to overcome it despite the
    /// hindrance.
    Pushback { boundary: VelocityBoundary },
}
