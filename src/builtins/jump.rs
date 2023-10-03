use bevy::prelude::*;

use crate::basis_action_traits::{
    TnuaAction, TnuaActionContext, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus,
};
use crate::util::SegmentedJumpInitialVelocityCalculator;

pub struct TnuaBuiltinJump {
    pub height: f32,
    pub upslope_extra_gravity: f32,
    pub takeoff_extra_gravity: f32,
    pub takeoff_above_velocity: f32,
    pub peak_prevention_at_upward_velocity: f32,
    pub peak_prevention_extra_gravity: f32,
    pub shorten_extra_gravity: f32,
    pub coyote_time: f32,
    pub fall_extra_gravity: f32,
    pub reschedule_cooldown: Option<f32>,
    pub input_buffer_time: f32,
}

impl TnuaAction for TnuaBuiltinJump {
    const NAME: &'static str = "TnuaBuiltinJump";
    type State = TnuaBuiltinJumpState;

    fn initiation_decision(
        &self,
        ctx: TnuaActionContext,
        being_fed_for: &bevy::time::Stopwatch,
    ) -> crate::basis_action_traits::TnuaActionInitiationDirective {
        if let Some(airborne_duration) = ctx.basis.airborne_duration() {
            if airborne_duration.as_secs_f32() < self.coyote_time {
                TnuaActionInitiationDirective::Allow
            } else if being_fed_for.elapsed().as_secs_f32() < self.input_buffer_time {
                TnuaActionInitiationDirective::Delay
            } else {
                TnuaActionInitiationDirective::Reject
            }
        } else {
            // Not airborne - can jump
            TnuaActionInitiationDirective::Allow
        }
    }

    fn apply(
        &self,
        state: &mut Self::State,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut crate::TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        let up = ctx.basis.up_direction();

        if lifecycle_status.just_started() {
            let mut calculator = SegmentedJumpInitialVelocityCalculator::new(self.height);
            let gravity = ctx.tracker.gravity.dot(-up);
            let kinetic_energy = calculator
                .add_segment(
                    gravity + self.peak_prevention_extra_gravity,
                    self.peak_prevention_at_upward_velocity,
                )
                .add_segment(gravity, self.takeoff_above_velocity)
                .add_segment(gravity + self.takeoff_extra_gravity, f32::INFINITY)
                .kinetic_energy();
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
                    } else if let Some(airborne_duration) = ctx.basis.airborne_duration() {
                        if airborne_duration.as_secs_f32() < self.coyote_time {
                            // FIXME: Long coyote time allows for double jump. This needs to be
                            // fixed.
                            0.0
                        } else {
                            // TODO: air jumps?
                            return self.directive_simple_or_reschedule(lifecycle_status);
                        }
                    } else {
                        // Can this state even be reached?
                        return self.directive_simple_or_reschedule(lifecycle_status);
                    };
                    let gravity = ctx.tracker.gravity.dot(-up);
                    let energy_from_extra_height = extra_height * gravity;
                    let desired_kinetic_energy = *desired_energy - energy_from_extra_height;
                    let desired_upward_velocity = (2.0 * desired_kinetic_energy).sqrt();

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
                    let desired_upward_velocity = (2.0 * desired_kinetic_energy).sqrt();
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

                            // TODO: the rest of the StoppedMaintainingJump calculation from
                            // platformer.rs?

                            motor.lin.cancel_on_axis(up);
                            motor.lin.acceleration -= self.shorten_extra_gravity * up;
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

#[derive(Default, Debug)]
pub enum TnuaBuiltinJumpState {
    #[default]
    NoJump,
    // FreeFall,
    StartingJump {
        /// The potential energy at the top of the jump, when:
        /// * The potential energy at the bottom of the jump is defined as 0
        /// * The mass is 1
        /// Calculating the desired velocity based on energy is easier than using the ballistic
        /// formulas.
        desired_energy: f32,
    },
    SlowDownTooFastSlopeJump {
        desired_energy: f32,
        zero_potential_energy_at: Vec3,
    },
    MaintainingJump,
    StoppedMaintainingJump,
    FallSection,
}
