use std::time::Duration;

use bevy::prelude::*;

use crate::basis_action_traits::TnuaBasisContext;
use crate::util::ProjectionPlaneForRotation;
use crate::{TnuaAirborneStatus, TnuaBasis, TnuaVelChange};

pub struct TnuaBuiltinWalk {
    pub desired_velocity: Vec3,
    pub desired_forward: Vec3,
    pub float_height: f32,
    pub cling_distance: f32,
    pub up: Vec3,
    pub spring_strengh: f32,
    pub spring_dampening: f32,
    pub acceleration: f32,
    pub air_acceleration: f32,
    pub free_fall_extra_gravity: f32,
    pub tilt_offset_angvel: f32,
    pub tilt_offset_angacl: f32,
    pub turning_angvel: f32,
}

impl TnuaBasis for TnuaBuiltinWalk {
    const NAME: &'static str = "TnuaBuiltinWalk";
    type State = TnuaBuiltinWalkState;

    fn apply(&self, state: &mut Self::State, ctx: TnuaBasisContext, motor: &mut crate::TnuaMotor) {
        match &mut state.airborne_status {
            TnuaAirborneStatus::Grounded => {}
            TnuaAirborneStatus::Coyote { duration }
            | TnuaAirborneStatus::AirAction { name: _, duration } => {
                *duration += Duration::from_secs_f32(ctx.frame_duration);
            }
            TnuaAirborneStatus::PostAction { .. } => {}
        }

        let climb_vectors: Option<ClimbVectors>;
        let considered_in_air: bool;
        let impulse_to_offset: Vec3;

        if let Some(sensor_output) = &ctx.proximity_sensor.output {
            state.effective_velocity = ctx.tracker.velocity - sensor_output.entity_linvel;
            let sideways_unnormalized = sensor_output.normal.cross(self.up);
            if sideways_unnormalized == Vec3::ZERO {
                climb_vectors = None;
            } else {
                climb_vectors = Some(ClimbVectors {
                    direction: sideways_unnormalized
                        .cross(sensor_output.normal)
                        .normalize_or_zero(),
                    sideways: sideways_unnormalized.normalize_or_zero(),
                });
            }
            considered_in_air = state.airborne_status.is_in_air();
            if considered_in_air {
                impulse_to_offset = Vec3::ZERO;
                state.standing_on = None;
            } else {
                if let Some(standing_on_state) = &state.standing_on {
                    if standing_on_state.entity != sensor_output.entity {
                        impulse_to_offset = Vec3::ZERO;
                    } else {
                        impulse_to_offset =
                            sensor_output.entity_linvel - standing_on_state.entity_linvel;
                    }
                } else {
                    impulse_to_offset = Vec3::ZERO;
                }
                state.standing_on = Some(StandingOnState {
                    entity: sensor_output.entity,
                    entity_linvel: sensor_output.entity_linvel,
                });
            }
        } else {
            state.effective_velocity = ctx.tracker.velocity;
            climb_vectors = None;
            considered_in_air = true;
            impulse_to_offset = Vec3::ZERO;
            state.standing_on = None;
        }
        state.effective_velocity += impulse_to_offset;

        let velocity_on_plane = state.effective_velocity.reject_from(self.up);

        let exact_acceleration = self.desired_velocity - velocity_on_plane;

        let safe_direction_coefficient = self
            .desired_velocity
            .normalize_or_zero()
            .dot(velocity_on_plane.normalize_or_zero());
        let direction_change_factor = 1.5 - 0.5 * safe_direction_coefficient;

        let relevant_acceleration_limit = if considered_in_air {
            self.air_acceleration
        } else {
            self.acceleration
        };
        let acceleration = direction_change_factor * relevant_acceleration_limit;

        let walk_acceleration =
            exact_acceleration.clamp_length_max(ctx.frame_duration * acceleration);
        let walk_acceleration = if let Some(climb_vectors) = &climb_vectors {
            climb_vectors.project(walk_acceleration)
        } else {
            walk_acceleration
        };

        state.vertical_velocity = if let Some(climb_vectors) = &climb_vectors {
            state.effective_velocity.dot(climb_vectors.direction)
                * climb_vectors.direction.dot(self.up)
        } else {
            0.0
        };

        let upward_impulse: TnuaVelChange = 'upward_impulse: {
            for _ in 0..2 {
                if state.airborne_status.is_in_air() {
                    if let Some(sensor_output) = &ctx.proximity_sensor.output {
                        if sensor_output.proximity <= self.float_height {
                            match state.airborne_status {
                                TnuaAirborneStatus::Grounded
                                | TnuaAirborneStatus::Coyote { .. }
                                | TnuaAirborneStatus::PostAction { .. } => {
                                    state.airborne_status = TnuaAirborneStatus::Grounded;
                                }
                                TnuaAirborneStatus::AirAction { .. } => {}
                            }
                            continue;
                        }
                    }
                    if state.vertical_velocity <= 0.0 {
                        break 'upward_impulse TnuaVelChange::acceleration(
                            -self.free_fall_extra_gravity * self.up,
                        );
                    } else {
                        break 'upward_impulse TnuaVelChange::ZERO;
                    }
                } else {
                    if let Some(sensor_output) = &ctx.proximity_sensor.output {
                        // not doing the jump calculation here
                        let spring_offset = self.float_height - sensor_output.proximity;
                        state.standing_offset = -spring_offset;
                        let boost = self.spring_force_boost(state, &ctx, spring_offset);
                        break 'upward_impulse TnuaVelChange::boost(boost * self.up);
                    } else {
                        // match state.airborne_status {
                                state.airborne_status = TnuaAirborneStatus::Coyote {
                                    duration: Default::default(),
                                };
                            // TnuaAirborneStatus::Grounded => {
                            // }
                            // TnuaAirborneStatus::Coyote { .. }
                            // | TnuaAirborneStatus::AirAction { .. } 
                            // | TnuaAirborneStatus::PostAction { .. } => {
                            // }
                        // }
                        continue;
                    }
                }
            }
            error!("Tnua could not decide on jump state");
            TnuaVelChange::ZERO
        };
        motor.lin = TnuaVelChange::boost(walk_acceleration + impulse_to_offset) + upward_impulse;
        let new_velocity = state.effective_velocity + motor.lin.boost - impulse_to_offset;
        state.running_velocity = new_velocity.reject_from(self.up);

        // Tilt

        let torque_to_fix_tilt = {
            let tilted_up = ctx.tracker.rotation.mul_vec3(self.up);

            let rotation_required_to_fix_tilt = Quat::from_rotation_arc(tilted_up, self.up);

            let desired_angvel = (rotation_required_to_fix_tilt.xyz() / ctx.frame_duration)
                .clamp_length_max(self.tilt_offset_angvel);
            let angular_velocity_diff = desired_angvel - ctx.tracker.angvel;
            angular_velocity_diff.clamp_length_max(ctx.frame_duration * self.tilt_offset_angacl)
        };

        // Turning

        let desired_angvel = if 0.0 < self.desired_forward.length_squared() {
            let projection = ProjectionPlaneForRotation::from_up_using_default_forward(self.up);
            let current_forward = ctx.tracker.rotation.mul_vec3(projection.forward);
            let rotation_along_up_axis =
                projection.rotation_to_set_forward(current_forward, self.desired_forward);
            (rotation_along_up_axis / ctx.frame_duration)
                .clamp(-self.turning_angvel, self.turning_angvel)
        } else {
            0.0
        };

        // NOTE: This is the regular axis system so we used the configured up.
        let existing_angvel = ctx.tracker.angvel.dot(self.up);

        // This is the torque. Should it be clamped by an acceleration? From experimenting with
        // this I think it's meaningless and only causes bugs.
        let torque_to_turn = desired_angvel - existing_angvel;

        let existing_turn_torque = torque_to_fix_tilt.dot(self.up);
        let torque_to_turn = torque_to_turn - existing_turn_torque;

        motor.ang = TnuaVelChange::boost(torque_to_fix_tilt + torque_to_turn * self.up);
    }

    fn proximity_sensor_cast_range(&self) -> f32 {
        self.float_height + self.cling_distance
    }

    fn up_direction(&self, _state: &Self::State) -> Vec3 {
        self.up
    }

    fn displacement(&self, state: &Self::State) -> Option<Vec3> {
        match state.airborne_status {
            TnuaAirborneStatus::Grounded => Some(self.up * state.standing_offset),
            TnuaAirborneStatus::Coyote { .. } => None,
            TnuaAirborneStatus::AirAction { .. } => None,
            TnuaAirborneStatus::PostAction { .. } => None,
        }
    }

    fn effective_velocity(&self, state: &Self::State) -> Vec3 {
        state.effective_velocity
    }

    fn vertical_velocity(&self, state: &Self::State) -> f32 {
        state.vertical_velocity
    }

    fn neutralize(&mut self) {
        self.desired_velocity = Vec3::ZERO;
        self.desired_forward = Vec3::ZERO;
    }

    fn airborne_status(&self, state: &Self::State) -> TnuaAirborneStatus {
        state.airborne_status.clone()
    }

    fn update_air_action(&self, state: &mut Self::State, name: Option<&'static str>) {
        state.airborne_status = match state.airborne_status {
            TnuaAirborneStatus::Grounded
            | TnuaAirborneStatus::Coyote { .. }
            | TnuaAirborneStatus::PostAction { .. } => {
                let Some(name) = name else { return };
                TnuaAirborneStatus::AirAction {
                    name,
                    duration: Duration::ZERO,
                }
            }
            TnuaAirborneStatus::AirAction { name: old_name, .. } => {
                if let Some(name) = name {
                    TnuaAirborneStatus::AirAction {
                        name,
                        duration: Duration::ZERO,
                    }
                } else {
                    TnuaAirborneStatus::PostAction { name: old_name }
                }
            }
        };
    }
}

impl TnuaBuiltinWalk {
    // TODO: maybe this needs to be an acceleration rather than an
    // impulse? The problem is the comparison between `spring_impulse`
    // and `offset_change_impulse`...
    pub fn spring_force_boost(
        &self,
        state: &TnuaBuiltinWalkState,
        ctx: &TnuaBasisContext,
        spring_offset: f32,
    ) -> f32 {
        let spring_force: f32 = spring_offset * self.spring_strengh;

        let relative_velocity = state.effective_velocity.dot(self.up) - state.vertical_velocity;

        let dampening_force = relative_velocity * self.spring_dampening / ctx.frame_duration;
        let spring_force = spring_force - dampening_force;

        let gravity_compensation = -ctx.tracker.gravity.dot(self.up);

        ctx.frame_duration * (spring_force + gravity_compensation)
    }
}

#[derive(Debug)]
struct StandingOnState {
    entity: Entity,
    entity_linvel: Vec3,
}

#[derive(Default)]
pub struct TnuaBuiltinWalkState {
    airborne_status: TnuaAirborneStatus,
    pub standing_offset: f32,
    standing_on: Option<StandingOnState>,
    effective_velocity: Vec3,
    vertical_velocity: f32,
    pub running_velocity: Vec3,
}

impl TnuaBuiltinWalkState {
    pub fn standing_on_entity(&self) -> Option<Entity> {
        Some(self.standing_on.as_ref()?.entity)
    }
}

struct ClimbVectors {
    direction: Vec3,
    sideways: Vec3,
}

impl ClimbVectors {
    fn project(&self, vector: Vec3) -> Vec3 {
        let axis_direction = vector.dot(self.direction) * self.direction;
        let axis_sideways = vector.dot(self.sideways) * self.sideways;
        axis_direction + axis_sideways
    }
}
