use bevy::prelude::*;

use crate::{
    tnua_system_set_for_computing_logic, TnuaDataSynchronizedFromBackend, TnuaMotor,
    TnuaProximitySensor,
};

pub struct TnuaPlatformerPlugin;

impl Plugin for TnuaPlatformerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            tnua_system_set_for_computing_logic().with_system(platformer_control_system),
        );
    }
}

#[derive(Bundle)]
pub struct TnuaPlatformerBundle {
    pub config: TnuaPlatformerConfig,
    pub controls: TnuaPlatformerControls,
    pub motor: TnuaMotor,
    pub proximity_sensor: TnuaProximitySensor,
    pub state: TnuaPlatformerState,
}

impl TnuaPlatformerBundle {
    pub fn new_with_config(config: TnuaPlatformerConfig) -> Self {
        Self {
            config,
            controls: Default::default(),
            motor: Default::default(),
            proximity_sensor: Default::default(),
            state: Default::default(),
        }
    }
}

#[derive(Component)]
pub struct TnuaPlatformerConfig {
    pub float_height: f32,
    pub cling_distance: f32,
    pub spring_strengh: f32,
    pub spring_dampening: f32,
    pub acceleration: f32,
    pub jump_impulse: f32,
    pub jump_height_reached_fall_speed: f32,
    pub jump_height_reached_acceleration: f32,
    pub jump_shorted_fall_speed: f32,
    pub jump_shorted_acceleration: f32,
    pub exponential_jump_stop_until: f32,
    pub exponential_jump_stop_factor: f32,
}

#[derive(Component)]
pub struct TnuaPlatformerControls {
    pub up: Vec3,
    pub move_direction: Vec3,
    pub jump: Option<f32>,
}

#[derive(Component, Default, Debug)]
pub struct TnuaPlatformerState {
    jump_state: JumpState,
}

#[derive(Default, Debug)]
enum JumpState {
    #[default]
    NoJump,
    JumpingFrom(Vec3),
    StoppedJumpingAt(Vec3),
}

impl Default for TnuaPlatformerControls {
    fn default() -> Self {
        Self {
            up: Vec3::Y,
            move_direction: Vec3::ZERO,
            jump: None,
        }
    }
}

fn platformer_control_system(
    time: Res<Time>,
    mut query: Query<(
        &GlobalTransform,
        &TnuaPlatformerControls,
        &TnuaPlatformerConfig,
        &mut TnuaPlatformerState,
        &mut TnuaProximitySensor,
        &mut TnuaMotor,
    )>,
    data_synchronized_from_backend: Res<TnuaDataSynchronizedFromBackend>,
) {
    for (transform, controls, config, mut platformer_state, mut sensor, mut motor) in
        query.iter_mut()
    {
        sensor.cast_range = config.float_height + config.cling_distance;

        let effective_velocity;
        if let (Some(sensor_output), JumpState::NoJump) =
            (&sensor.output, &platformer_state.jump_state)
        {
            let spring_offset = config.float_height - sensor_output.proximity;
            let spring_force = spring_offset * config.spring_strengh /* subtract dumpning */;

            let relative_velocity =
                sensor_output.relative_velocity.dot(sensor.cast_direction) * sensor.cast_direction;

            let dampening_force = relative_velocity * config.spring_dampening;
            let spring_force = spring_force - dampening_force;
            motor.desired_acceleration = time.delta().as_secs_f32() * controls.up * spring_force;
            effective_velocity = sensor_output.relative_velocity;
        } else {
            motor.desired_acceleration = Vec3::ZERO;
            effective_velocity = sensor.velocity;
        }

        let velocity_on_plane =
            effective_velocity - controls.up * controls.up.dot(effective_velocity);

        let desired_velocity = controls.move_direction;
        let exact_acceleration = desired_velocity - velocity_on_plane;

        let safe_direction_coefficient = desired_velocity
            .normalize_or_zero()
            .dot(velocity_on_plane.normalize_or_zero());
        let direction_change_factor = 1.5 - 0.5 * safe_direction_coefficient;

        let acceleration = direction_change_factor * config.acceleration;

        let capped_acceperation =
            exact_acceleration.clamp_length_max(time.delta().as_secs_f32() * acceleration);

        // TODO: Do I need maximum force capping?

        motor.desired_acceleration += capped_acceperation;

        match platformer_state.jump_state {
            JumpState::NoJump => {
                if let (Some(jump_height), Some(sensor_output)) = (controls.jump, &sensor.output) {
                    let jumping_from = transform.translation()
                        + sensor.cast_direction * (sensor_output.proximity - config.float_height);

                    let gravity = data_synchronized_from_backend.gravity.dot(-controls.up);

                    let jump_impulse = (2.0 * gravity * jump_height).sqrt();

                    motor.desired_acceleration += controls.up * jump_impulse;
                    platformer_state.jump_state = JumpState::JumpingFrom(jumping_from);
                }
            }
            JumpState::JumpingFrom(jumping_from) => {
                if let Some(jump_height) = controls.jump {
                    let current_height = (transform.translation() - jumping_from).dot(controls.up);
                    if jump_height <= current_height {
                        platformer_state.jump_state =
                            JumpState::StoppedJumpingAt(jumping_from + jump_height * controls.up);
                    }
                } else {
                    platformer_state.jump_state =
                        JumpState::StoppedJumpingAt(transform.translation());
                }
            }
            JumpState::StoppedJumpingAt(_) => {
                let upward_velocity = effective_velocity.dot(controls.up);

                let downward_boost_capped_exponentially = {
                    let required_exponential_downward_boost =
                        upward_velocity - config.exponential_jump_stop_until;
                    if 0.0 < required_exponential_downward_boost {
                        required_exponential_downward_boost
                            * (config
                                .exponential_jump_stop_factor
                                .powf(0.1 / time.delta().as_secs_f32()))
                    } else {
                        0.0
                    }
                };
                let downward_boost_capped_linearly = {
                    let (fall_speed, acceleration) = if controls.jump.is_some() {
                        (
                            config.jump_height_reached_fall_speed,
                            config.jump_height_reached_acceleration,
                        )
                    } else {
                        (
                            config.jump_shorted_fall_speed,
                            config.jump_shorted_acceleration,
                        )
                    };
                    let required_downward_boost = upward_velocity + fall_speed;
                    if 0.0 < required_downward_boost {
                        required_downward_boost.min(time.delta().as_secs_f32() * acceleration)
                    } else {
                        0.0
                    }
                };
                let downward_boost =
                    downward_boost_capped_exponentially.max(downward_boost_capped_linearly);
                if 0.0 < downward_boost {
                    if false {
                        motor.desired_acceleration -= controls.up * downward_boost;
                    }
                } else {
                    platformer_state.jump_state = JumpState::NoJump;
                }
            }
        }
    }
}
