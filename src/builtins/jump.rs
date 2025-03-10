use crate::math::{AdjustPrecision, Float, Vector3};
use bevy::prelude::*;

use crate::util::SegmentedJumpInitialVelocityCalculator;
use crate::{
    TnuaAction, TnuaActionContext, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus,
};

/// The basic jump [action](TnuaAction).
///
/// This action implements jump physics explained in <https://youtu.be/hG9SzQxaCm8> and
/// <https://youtu.be/eeLPL3Y9jjA>. Most of its fields have sane defaults - the only field that
/// must be set is [`height`](Self::height), which controls the jump height.
///
/// The action must be fed for as long as the player holds the jump button. Once the action stops
/// being fed, it'll apply extra gravity to shorten the jump. If the game desires fixed height
/// jumps instead (where the player cannot make lower jumps by tapping the jump button)
/// [`shorten_extra_gravity`](Self::shorten_extra_gravity) should be set to `0.0`.
#[derive(Clone, Debug)]
pub struct TnuaBuiltinJump {
    /// The height the character will jump to.
    ///
    /// If [`shorten_extra_gravity`](Self::shorten_extra_gravity) is higher than `0.0`, the
    /// character may stop the jump in the middle if the jump action is no longer fed (usually when
    /// the player releases the jump button) and the character may not reach its full jump height.
    ///
    /// The jump height is calculated from the center of the character at float_height to the
    /// center of the character at the top of the jump. It _does not_ mean the height from the
    /// ground. The float height is calculated by the inspecting the character's current position
    /// and the basis' [`displacement`](crate::TnuaBasis::displacement).
    pub height: Float,

    /// Allow this action to start even if the character is not touching ground nor in coyote time.
    pub allow_in_air: bool,

    /// Extra gravity for breaking too fast jump from running up a slope.
    ///
    /// When running up a slope, the character gets more jump strength to avoid slamming into the
    /// slope. This may cause the jump to be too high, so this value is used to brake it.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub upslope_extra_gravity: Float,

    /// Extra gravity for fast takeoff.
    ///
    /// Without this, jumps feel painfully slow. Adding this will apply extra gravity until the
    /// vertical velocity reaches below [`takeoff_above_velocity`](Self::takeoff_above_velocity),
    /// and increase the initial jump boost in order to compensate. This will make the jump feel
    /// more snappy.
    pub takeoff_extra_gravity: Float,

    /// The range of upward velocity during [`takeoff_extra_gravity`](Self::takeoff_extra_gravity)
    /// is applied.
    ///
    /// To disable, set this to [`Float::INFINITY`] rather than zero.
    pub takeoff_above_velocity: Float,

    /// Extra gravity for falling down after reaching the top of the jump.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub fall_extra_gravity: Float,

    /// Extra gravity for shortening a jump when the player releases the jump button.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub shorten_extra_gravity: Float,

    /// Used to decrease the time the character spends "floating" at the peak of the jump.
    ///
    /// When the character's upward velocity is above this value,
    /// [`peak_prevention_extra_gravity`](Self::peak_prevention_extra_gravity) will be added to the
    /// gravity in order to shorten the float time.
    ///
    /// This extra gravity is taken into account when calculating the initial jump speed, so the
    /// character is still supposed to reach its full jump [`height`](Self::height).
    pub peak_prevention_at_upward_velocity: Float,

    /// Extra gravity for decreasing the time the character spends at the peak of the jump.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub peak_prevention_extra_gravity: Float,

    /// A duration, in seconds, after which the character would jump if the jump button was already
    /// pressed when the jump became available.
    ///
    /// The duration is measured from the moment the jump became available - not from the moment
    /// the jump button was pressed.
    ///
    /// When set to `None`, the character will not jump no matter how long the player holds the
    /// jump button.
    ///
    /// If the jump button is held but the jump input is still buffered (see
    /// [`input_buffer_time`](Self::input_buffer_time)), this setting will have no effect because
    /// the character will simply jump immediately.
    pub reschedule_cooldown: Option<Float>,

    /// A duration, in seconds, where a player can press a jump button before a jump becomes
    /// possible (typically when a character is still in the air and about the land) and the jump
    /// action would still get registered and be executed once the jump is possible.
    pub input_buffer_time: Float,
}

impl Default for TnuaBuiltinJump {
    fn default() -> Self {
        Self {
            height: 0.0,
            allow_in_air: false,
            upslope_extra_gravity: 30.0,
            takeoff_extra_gravity: 30.0,
            takeoff_above_velocity: 2.0,
            fall_extra_gravity: 20.0,
            shorten_extra_gravity: 60.0,
            peak_prevention_at_upward_velocity: 1.0,
            peak_prevention_extra_gravity: 20.0,
            reschedule_cooldown: None,
            input_buffer_time: 0.2,
        }
    }
}

impl TnuaAction for TnuaBuiltinJump {
    const NAME: &'static str = "TnuaBuiltinJump";
    type State = TnuaBuiltinJumpState;
    const VIOLATES_COYOTE_TIME: bool = true;

    fn initiation_decision(
        &self,
        ctx: TnuaActionContext,
        being_fed_for: &bevy::time::Stopwatch,
    ) -> crate::basis_action_traits::TnuaActionInitiationDirective {
        if self.allow_in_air || !ctx.basis.is_airborne() {
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
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut crate::TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        let up = ctx.up_direction.adjust_precision();

        if lifecycle_status.just_started() {
            let mut calculator = SegmentedJumpInitialVelocityCalculator::new(self.height);
            let gravity = ctx.tracker.gravity.dot(-up);
            let kinetic_energy = calculator
                .add_segment(
                    gravity + self.peak_prevention_extra_gravity,
                    self.peak_prevention_at_upward_velocity,
                )
                .add_segment(gravity, self.takeoff_above_velocity)
                .add_final_segment(gravity + self.takeoff_extra_gravity)
                .kinetic_energy()
                .expect("`add_final_segment` should have covered remaining height");
            *state = TnuaBuiltinJumpState::StartingJump {
                desired_energy: kinetic_energy,
            };
        }

        let effective_velocity = ctx.basis.effective_velocity();

        // TODO: Once `std::mem::variant_count` gets stabilized, use that instead. The idea is to
        // allow jumping through multiple states but failing if we get into loop.
        for _ in 0..7 {
            return match state {
                TnuaBuiltinJumpState::NoJump => panic!(),
                TnuaBuiltinJumpState::StartingJump { desired_energy } => {
                    let extra_height = if let Some(displacement) = ctx.basis.displacement() {
                        displacement.dot(up)
                    } else if !self.allow_in_air && ctx.basis.is_airborne() {
                        return self.directive_simple_or_reschedule(lifecycle_status);
                    } else {
                        // This means we are at Coyote time, so just jump from place.
                        0.0
                    };
                    let gravity = ctx.tracker.gravity.dot(-up);
                    let energy_from_extra_height = extra_height * gravity;
                    let desired_kinetic_energy = *desired_energy - energy_from_extra_height;
                    let desired_upward_velocity =
                        SegmentedJumpInitialVelocityCalculator::kinetic_energy_to_velocity(
                            desired_kinetic_energy,
                        );

                    let relative_velocity =
                        effective_velocity.dot(up) - ctx.basis.vertical_velocity().max(0.0);

                    motor.lin.cancel_on_axis(up);
                    motor.lin.boost += (desired_upward_velocity - relative_velocity) * up;
                    if 0.0 <= extra_height {
                        *state = TnuaBuiltinJumpState::SlowDownTooFastSlopeJump {
                            desired_energy: *desired_energy,
                            zero_potential_energy_at: ctx.tracker.translation - extra_height * up,
                        };
                    }
                    self.directive_simple_or_reschedule(lifecycle_status)
                }
                TnuaBuiltinJumpState::SlowDownTooFastSlopeJump {
                    desired_energy,
                    zero_potential_energy_at,
                } => {
                    let upward_velocity = up.dot(effective_velocity);
                    if upward_velocity <= ctx.basis.vertical_velocity() {
                        *state = TnuaBuiltinJumpState::FallSection;
                        continue;
                    } else if !lifecycle_status.is_active() {
                        *state = TnuaBuiltinJumpState::StoppedMaintainingJump;
                        continue;
                    }
                    let relative_velocity = effective_velocity.dot(up);
                    let extra_height =
                        (ctx.tracker.translation - *zero_potential_energy_at).dot(up);
                    let gravity = ctx.tracker.gravity.dot(-up);
                    let energy_from_extra_height = extra_height * gravity;
                    let desired_kinetic_energy = *desired_energy - energy_from_extra_height;
                    let desired_upward_velocity =
                        SegmentedJumpInitialVelocityCalculator::kinetic_energy_to_velocity(
                            desired_kinetic_energy,
                        );
                    if relative_velocity <= desired_upward_velocity {
                        *state = TnuaBuiltinJumpState::MaintainingJump;
                        continue;
                    } else {
                        let mut extra_gravity = self.upslope_extra_gravity;
                        if self.takeoff_above_velocity <= relative_velocity {
                            extra_gravity += self.takeoff_extra_gravity;
                        }
                        motor.lin.cancel_on_axis(up);
                        motor.lin.acceleration = -extra_gravity * up;
                        self.directive_simple_or_reschedule(lifecycle_status)
                    }
                }
                TnuaBuiltinJumpState::MaintainingJump => {
                    let relevant_upward_velocity = effective_velocity.dot(up);
                    if relevant_upward_velocity <= 0.0 {
                        *state = TnuaBuiltinJumpState::FallSection;
                        motor.lin.cancel_on_axis(up);
                    } else {
                        motor.lin.cancel_on_axis(up);
                        if relevant_upward_velocity < self.peak_prevention_at_upward_velocity {
                            motor.lin.acceleration -= self.peak_prevention_extra_gravity * up;
                        } else if self.takeoff_above_velocity <= relevant_upward_velocity {
                            motor.lin.acceleration -= self.takeoff_extra_gravity * up;
                        }
                    }
                    match lifecycle_status {
                        TnuaActionLifecycleStatus::Initiated
                        | TnuaActionLifecycleStatus::CancelledFrom
                        | TnuaActionLifecycleStatus::StillFed => {
                            TnuaActionLifecycleDirective::StillActive
                        }
                        TnuaActionLifecycleStatus::CancelledInto => self.finish_or_reschedule(),
                        TnuaActionLifecycleStatus::NoLongerFed => {
                            *state = TnuaBuiltinJumpState::StoppedMaintainingJump;
                            TnuaActionLifecycleDirective::StillActive
                        }
                    }
                }
                TnuaBuiltinJumpState::StoppedMaintainingJump => {
                    if matches!(lifecycle_status, TnuaActionLifecycleStatus::CancelledInto) {
                        self.finish_or_reschedule()
                    } else {
                        let landed = ctx
                            .basis
                            .displacement()
                            .map_or(false, |displacement| displacement.dot(up) <= 0.0);
                        if landed {
                            self.finish_or_reschedule()
                        } else {
                            let upward_velocity = up.dot(effective_velocity);
                            if upward_velocity <= 0.0 {
                                *state = TnuaBuiltinJumpState::FallSection;
                                continue;
                            }

                            let extra_gravity = if self.takeoff_above_velocity <= upward_velocity {
                                self.shorten_extra_gravity + self.takeoff_extra_gravity
                            } else {
                                self.shorten_extra_gravity
                            };

                            motor.lin.cancel_on_axis(up);
                            motor.lin.acceleration -= extra_gravity * up;
                            TnuaActionLifecycleDirective::StillActive
                        }
                    }
                }
                TnuaBuiltinJumpState::FallSection => {
                    let landed = ctx
                        .basis
                        .displacement()
                        .map_or(false, |displacement| displacement.dot(up) <= 0.0);
                    if landed
                        || matches!(lifecycle_status, TnuaActionLifecycleStatus::CancelledInto)
                    {
                        self.finish_or_reschedule()
                    } else {
                        motor.lin.cancel_on_axis(up);
                        motor.lin.acceleration -= self.fall_extra_gravity * up;
                        TnuaActionLifecycleDirective::StillActive
                    }
                }
            };
        }
        error!("Tnua could not decide on jump state");
        TnuaActionLifecycleDirective::Finished
    }
}

impl TnuaBuiltinJump {
    fn finish_or_reschedule(&self) -> TnuaActionLifecycleDirective {
        if let Some(cooldown) = self.reschedule_cooldown {
            TnuaActionLifecycleDirective::Reschedule {
                after_seconds: cooldown,
            }
        } else {
            TnuaActionLifecycleDirective::Finished
        }
    }

    fn directive_simple_or_reschedule(
        &self,
        lifecycle_status: TnuaActionLifecycleStatus,
    ) -> TnuaActionLifecycleDirective {
        if let Some(cooldown) = self.reschedule_cooldown {
            lifecycle_status.directive_simple_reschedule(cooldown)
        } else {
            lifecycle_status.directive_simple()
        }
    }
}

#[derive(Default, Debug, Clone)]
pub enum TnuaBuiltinJumpState {
    #[default]
    NoJump,
    // FreeFall,
    StartingJump {
        /// The potential energy at the top of the jump, when:
        /// * The potential energy at the bottom of the jump is defined as 0
        /// * The mass is 1
        ///
        /// Calculating the desired velocity based on energy is easier than using the ballistic
        /// formulas.
        desired_energy: Float,
    },
    SlowDownTooFastSlopeJump {
        desired_energy: Float,
        zero_potential_energy_at: Vector3,
    },
    MaintainingJump,
    StoppedMaintainingJump,
    FallSection,
}
