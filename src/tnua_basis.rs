use std::time::Duration;

use bevy::prelude::*;

use crate::{TnuaBasis, TnuaVelChange};

pub struct Walk {
    pub desired_velocity: Vec3,
    pub float_height: f32,
    pub cling_distance: f32,
    pub up: Vec3,
    pub spring_strengh: f32,
    pub spring_dampening: f32,
    pub height_change_impulse_for_duration: f32,
    pub height_change_impulse_limit: f32,
    pub acceleration: f32,
    pub air_acceleration: f32,
    pub coyote_time: f32,
    pub free_fall_extra_gravity: f32,
}

impl TnuaBasis for Walk {
    const NAME: &'static str = "Walk";
    type State = WalkState;

    fn apply(
        &self,
        state: &mut Self::State,
        ctx: crate::basis_action_traits::TnuaBasisContext,
        motor: &mut crate::TnuaMotor,
    ) {
        match &mut state.airborne_state {
            AirborneState::NoJump => {}
            AirborneState::FreeFall { coyote_time } => {
                coyote_time.tick(Duration::from_secs_f32(ctx.frame_duration));
            }
        }
        // TODO: calc the climb vectors

        let float_height_offset = 0.0; // is this needed?
        let prev_float_height_offset = 0.0; // maybe this should be prev_float_height instead of an
                                            // offset?

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
            considered_in_air = match state.airborne_state {
                AirborneState::NoJump => false,
                AirborneState::FreeFall { .. } => true,
            };
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
                match &mut state.airborne_state {
                    AirborneState::NoJump => {
                        if let Some(sensor_output) = &ctx.proximity_sensor.output {
                            // not doing the jump calculation here
                            let spring_offset = self.float_height - sensor_output.proximity;
                            state.standing_offset = -spring_offset;
                            let spring_offset = spring_offset + float_height_offset;
                            let spring_force: f32 = spring_offset * self.spring_strengh;
                            let offset_change_impulse: f32 =
                                if 0.01 <= (float_height_offset - prev_float_height_offset).abs() {
                                    let velocity_to_get_to_new_float_height =
                                        spring_offset / self.height_change_impulse_for_duration;
                                    velocity_to_get_to_new_float_height.clamp(
                                        -self.height_change_impulse_limit,
                                        self.height_change_impulse_limit,
                                    )
                                } else {
                                    0.0
                                };

                            let relative_velocity =
                                state.effective_velocity.dot(self.up) - state.vertical_velocity;

                            let dampening_force =
                                relative_velocity * self.spring_dampening / ctx.frame_duration;
                            let spring_force = spring_force - dampening_force;

                            let gravity_compensation = -ctx.tracker.gravity.dot(self.up);

                            let spring_impulse =
                                ctx.frame_duration * (spring_force + gravity_compensation);

                            let impulse_to_use =
                                if spring_impulse.abs() < offset_change_impulse.abs() {
                                    offset_change_impulse
                                } else {
                                    spring_impulse
                                };

                            // TODO: maybe this needs to be an acceleration rather than an
                            // impulse? The problem is the comparison between `spring_impulse`
                            // and `offset_change_impulse`...
                            break 'upward_impulse TnuaVelChange::boost(impulse_to_use * self.up);
                        } else {
                            state.airborne_state = AirborneState::FreeFall {
                                coyote_time: Timer::new(
                                    Duration::from_secs_f32(self.coyote_time),
                                    TimerMode::Once,
                                ),
                            };
                            continue;
                        }
                    }
                    AirborneState::FreeFall { coyote_time: _ } => {
                        if let Some(sensor_output) = &ctx.proximity_sensor.output {
                            if sensor_output.proximity <= self.float_height {
                                state.airborne_state = AirborneState::NoJump;
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
                    }
                }
            }
            error!("Tnua could not decide on jump state");
            TnuaVelChange::ZERO
        };
        motor.lin = TnuaVelChange::boost(walk_acceleration + impulse_to_offset) + upward_impulse;
        let new_velocity = state.effective_velocity + motor.lin.boost - impulse_to_offset;
        state.running_velocity = new_velocity.reject_from(self.up);
    }

    fn proximity_sensor_cast_range(&self) -> f32 {
        // TODO - also need to consider float_height_offset? Or maybe it should be united,
        // or converted into an action?
        self.float_height + self.cling_distance
    }

    fn up_direction(&self, _state: &Self::State) -> Vec3 {
        self.up
    }

    fn displacement(&self, state: &Self::State) -> Option<Vec3> {
        match state.airborne_state {
            AirborneState::NoJump => Some(self.up * state.standing_offset),
            AirborneState::FreeFall { .. } => None,
        }
    }

    fn effective_velocity(&self, state: &Self::State) -> Vec3 {
        state.effective_velocity
    }

    fn vertical_velocity(&self, state: &Self::State) -> f32 {
        state.vertical_velocity
    }
}

#[derive(Debug)]
struct StandingOnState {
    entity: Entity,
    entity_linvel: Vec3,
}

#[derive(Default)]
pub struct WalkState {
    airborne_state: AirborneState,
    pub standing_offset: f32,
    standing_on: Option<StandingOnState>,
    effective_velocity: Vec3,
    vertical_velocity: f32,
    pub running_velocity: Vec3,
}

impl WalkState {
    pub fn standing_on_entity(&self) -> Option<Entity> {
        Some(self.standing_on.as_ref()?.entity)
    }
}

// TODO: does this need to be an `enum`? Without all the jump-specific fields, maybe it can be an
// `Option`?
#[derive(Default)]
enum AirborneState {
    #[default]
    NoJump,
    FreeFall {
        // Maybe move the coyote time setting to the jump, and make this a Stopwatch?
        coyote_time: Timer,
    },
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
