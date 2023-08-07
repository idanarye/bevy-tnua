// use bevy::prelude::*;

use crate::basis_action_traits::{
    TnuaAction, TnuaActionContext, TnuaActionLifecycleDirective, TnuaActionLifecycleStatus,
};
use crate::util::SegmentedJumpInitialVelocityCalculator;

pub struct Jump {
    pub height: f32,
}

impl TnuaAction for Jump {
    type State = JumpState;

    fn apply(
        &self,
        state: &mut Self::State,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut crate::TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        let up = ctx.basis.up_direction();

        // TODO: properly calculate these:
        let effective_velocity = ctx.tracker.velocity;
        let vertical_velocity: f32 = 0.0;

        if lifecycle_status.just_started() {
            let mut calculator = SegmentedJumpInitialVelocityCalculator::new(self.height);
            let gravity = ctx.tracker.gravity.dot(-up);
            // TODO: add all the actual segments
            let kinetic_energy = calculator
                .add_segment(gravity, f32::INFINITY)
                .kinetic_energy();
            if lifecycle_status == TnuaActionLifecycleStatus::Initiated {}
            *state = JumpState::StartingJump {
                desired_energy: kinetic_energy,
            };
        }

        match state {
            JumpState::NoJump => panic!(),
            JumpState::StartingJump { desired_energy } => {
                let extra_height = if let Some(displacement) = ctx.basis.displacement() {
                    displacement.dot(up)
                } else {
                    0.0
                };
                let gravity = ctx.tracker.gravity.dot(-up);
                let energy_from_extra_height = extra_height * gravity;
                let desired_kinetic_energy = *desired_energy - energy_from_extra_height;
                let desired_upward_velocity = (2.0 * desired_kinetic_energy).sqrt();

                let relative_velocity = effective_velocity.dot(up) - vertical_velocity.max(0.0);

                motor.lin.cancel_on_axis(up);
                motor.lin.boost += (desired_upward_velocity - relative_velocity) * up;
                if 0.0 <= extra_height {
                    *state = JumpState::MaintainingJump;
                }
                lifecycle_status.directive_simple()
            }
            JumpState::MaintainingJump => {
                if effective_velocity.dot(up) < 0.0 {
                    *state = JumpState::FallSection;
                } else {
                    motor.lin.cancel_on_axis(up);
                }
                lifecycle_status.directive_simple()
            }
            JumpState::FallSection => {
                let landed = ctx
                    .basis
                    .displacement()
                    .map_or(false, |displacement| displacement.dot(up) <= 0.0);
                if landed {
                    TnuaActionLifecycleDirective::Finished
                } else {
                    motor.lin.cancel_on_axis(up);
                    lifecycle_status.directive_simple()
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub enum JumpState {
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
    // SlowDownTooFastSlopeJump {
    // desired_energy: f32,
    // zero_potential_energy_at: Vec3,
    // },
    MaintainingJump,
    // StoppedMaintainingJump,
    FallSection,
}
