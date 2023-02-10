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
    /// The height at which the character will float above ground at rest.
    ///
    /// Note that this is the height of the character's center of mass - not the distance from its
    /// collision mesh.
    pub float_height: f32,

    /// Extra distance above the `float_height` where the spring is still in effect.
    ///
    /// When the character is at at most this distance above the `float_height`, the spring force
    /// will kick in and move it to the float height - even if that means pushing it down. If the
    /// character is above that distance above the `float_height`, Tnua will consider it to be in
    /// the air.
    pub cling_distance: f32,

    /// The force that pushes the character to the float height.
    ///
    /// The actual force applied is in direct linear relationship to the displacement from the
    /// `float_height`.
    pub spring_strengh: f32,

    /// A force that slows down the characters vertical spring motion.
    ///
    /// The actual dampening is in direct linear relationship to the vertical velocity it tries to
    /// dampen.
    pub spring_dampening: f32,

    /// The acceleration for horizontal movement.
    ///
    /// Note that this is the acceleration for starting the horizontal motion and for reaching the
    /// top speed. When braking or changing direction the acceleration is greater, up to 2 times
    /// `acceleration` when doing a 180 turn.
    pub acceleration: f32,

    /// Extra gravity for falling down after reaching the top of the jump.
    pub jump_fall_extra_gravity: f32,

    /// Extra gravity for shortening a jump when the player releases the jump button.
    pub jump_shorten_extra_gravity: f32,
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
    StartingJump {
        /// The potential energy at the top of the jump, when:
        /// * The potential energy at the bottom of the jump is defined as 0
        /// * The mass is 1
        /// Calculating the desired velocity based on energy is easier than using the ballistic
        /// formulas.
        desired_energy: f32,
    },
    MaintainingJump,
    StoppedMaintainingJump,
    FallSection,
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
    for (_transform, controls, config, mut platformer_state, mut sensor, mut motor) in
        query.iter_mut()
    {
        sensor.cast_range = config.float_height + config.cling_distance;

        let effective_velocity = if let Some(sensor_output) = &sensor.output {
            sensor_output.relative_velocity
        } else {
            sensor.velocity
        };

        let upward_velocity = controls.up.dot(effective_velocity);

        let velocity_on_plane = effective_velocity - controls.up * upward_velocity;

        let desired_velocity = controls.move_direction;
        let exact_acceleration = desired_velocity - velocity_on_plane;

        let safe_direction_coefficient = desired_velocity
            .normalize_or_zero()
            .dot(velocity_on_plane.normalize_or_zero());
        let direction_change_factor = 1.5 - 0.5 * safe_direction_coefficient;

        let acceleration = direction_change_factor * config.acceleration;

        let walk_acceleration =
            exact_acceleration.clamp_length_max(time.delta().as_secs_f32() * acceleration);

        // TODO: Do I need maximum force capping?

        let upward_impulse = 'upward_impulse: {
            for _ in 0..4 {
                match platformer_state.jump_state {
                    JumpState::NoJump => {
                        if let Some(sensor_output) = &sensor.output {
                            if let Some(jump_height) = controls.jump {
                                let gravity =
                                    data_synchronized_from_backend.gravity.dot(-controls.up);
                                //let upward_velocity_at_float_height =
                                //(2.0 * gravity * jump_height).sqrt();
                                platformer_state.jump_state = JumpState::StartingJump {
                                    desired_energy: gravity * jump_height,
                                };
                                continue;
                            } else {
                                let spring_offset = config.float_height - sensor_output.proximity;
                                let spring_force: f32 = spring_offset * config.spring_strengh;

                                let relative_velocity =
                                    sensor_output.relative_velocity.dot(sensor.cast_direction);

                                let dampening_force = relative_velocity * config.spring_dampening;
                                let spring_force = spring_force + dampening_force;

                                let gravity_compensation =
                                    -data_synchronized_from_backend.gravity.dot(controls.up);
                                break 'upward_impulse time.delta().as_secs_f32()
                                    * (spring_force + gravity_compensation);
                            }
                        } else {
                            break 'upward_impulse 0.0;
                        }
                    }
                    JumpState::StartingJump { desired_energy } => {
                        if let Some(sensor_output) = &sensor.output {
                            let relative_velocity =
                                -sensor_output.relative_velocity.dot(sensor.cast_direction);
                            let extra_height = sensor_output.proximity - config.float_height;
                            let gravity = data_synchronized_from_backend.gravity.dot(-controls.up);
                            let energy_from_extra_height = extra_height * gravity;
                            let desired_kinetic_energy = desired_energy - energy_from_extra_height;
                            let desired_upward_velocity = (2.0 * desired_kinetic_energy).sqrt();
                            break 'upward_impulse desired_upward_velocity - relative_velocity;
                        } else {
                            platformer_state.jump_state = JumpState::MaintainingJump;
                            continue;
                        }
                    }
                    JumpState::MaintainingJump => {
                        if upward_velocity <= 0.0 {
                            platformer_state.jump_state = JumpState::FallSection;
                            continue;
                        } else if controls.jump.is_none() {
                            platformer_state.jump_state = JumpState::StoppedMaintainingJump;
                            continue;
                        }
                        break 'upward_impulse 0.0;
                    }
                    JumpState::StoppedMaintainingJump => {
                        if upward_velocity <= 0.0 {
                            platformer_state.jump_state = JumpState::FallSection;
                            continue;
                        }
                        break 'upward_impulse -(time.delta().as_secs_f32()
                            * config.jump_shorten_extra_gravity);
                    }
                    JumpState::FallSection => {
                        if sensor.output.is_some() {
                            platformer_state.jump_state = JumpState::NoJump;
                            continue;
                        }
                        break 'upward_impulse -(time.delta().as_secs_f32()
                            * config.jump_fall_extra_gravity);
                    }
                }
            }
            error!("Tnua could not decide on jump state");
            0.0
        };

        motor.desired_acceleration = walk_acceleration + controls.up * upward_impulse;
    }
}
