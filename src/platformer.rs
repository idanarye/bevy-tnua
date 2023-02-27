use bevy::prelude::*;

use crate::{
    tnua_system_set_for_computing_logic, TnuaMotor, TnuaProximitySensor, TnuaRigidBodyTracker,
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
    pub rigid_body_tracker: TnuaRigidBodyTracker,
    pub proximity_sensor: TnuaProximitySensor,
    pub state: TnuaPlatformerState,
}

impl TnuaPlatformerBundle {
    pub fn new_with_config(config: TnuaPlatformerConfig) -> Self {
        Self {
            config,
            controls: Default::default(),
            motor: Default::default(),
            rigid_body_tracker: Default::default(),
            proximity_sensor: Default::default(),
            state: Default::default(),
        }
    }
}

/// Movement settings for a platformer-like character controlled by Tnua.
#[derive(Component)]
pub struct TnuaPlatformerConfig {
    /// The speed the character will try to reach when
    /// [`desired_velocity`](TnuaPlatformerControls::desired_velocity) is set to a unit vector.
    ///
    /// If `desired_velocity` is not a unit vector, the character will try to reach a speed of
    /// `desired_velocity.length() * `full_speed`. Note that this means that if `desired_velocity`
    /// has a magnitude greater than `1.0`, the character may exceed its full speed.
    pub full_speed: f32,

    /// The height the character will jump to when [`jump`](TnuaPlatformerControls::jump) is set to
    /// `Some(`1.0`)`.
    ///
    /// If `jump` is set to `Some(X)` where `X` is not `1.0`, the character will try to jump to an
    /// height of `X * full_jump_height`. Note that this means that if `X` is greater than `1.0`,
    /// the character may jump higher than its full jump height.
    ///
    /// If [`jump_shorten_extra_gravity`](Self::jump_shorten_extra_gravity) is higher than `0.0`,
    /// the character may stop the jump in the middle if `jump` is set to `None` (usually when the
    /// player releases the jump button) and the character may not reach its full jump height.
    ///
    /// The jump height is calculated from the center of the character at
    /// [`float_height`](Self::float_height) to the center of the character at the top of the jump.
    /// It _does not_ mean the height from the ground.
    pub full_jump_height: f32,

    /// The direction considered as upward.
    ///
    /// Typically `Vec3::Y`.
    pub up: Vec3,

    /// The direction considered as forward.
    ///
    /// This is the direcetion the character is facing when no rotation is applied. Typically
    /// `Vec3::X` for 2D and `Vec3::Z` for 3D.
    pub forward: Vec3,

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

    /// The acceleration for horizontal movement while in the air.
    ///
    /// Set to 0.0 to completely disable air movement.
    pub air_acceleration: f32,

    /// Extra gravity for breaking too fast jump from running up a slope.
    ///
    /// When running up a slope, the character gets more jump strength to avoid slamming into the
    /// slope. This may cause the jump to be too high, so this value is used to brake it.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub jump_start_extra_gravity: f32,

    /// Extra gravity for falling down after reaching the top of the jump.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub jump_fall_extra_gravity: f32,

    /// Extra gravity for shortening a jump when the player releases the jump button.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub jump_shorten_extra_gravity: f32,

    /// What to do when the character is in the air without jumping (e.g. when stepping off a
    /// ledge)
    ///
    /// **NOTE**: Depending on this setting, the character may be able to run up a slope and "jump"
    /// potentially even higher than a regular jump, even without pressing the jump button. See the
    /// documentation for the individual enum variants for information regarding how to prevent
    /// this.
    pub free_fall_behavior: TnuaFreeFallBehavior,

    /// The maximum angular velocity used for keeping the character standing upright.
    ///
    /// NOTE: The character's rotation can also be locked to prevent it from being tilted, in which
    /// case this paramter is redundant and can be set to 0.0.
    pub tilt_offset_angvel: f32,

    /// The maximum angular acceleration used for reaching `tilt_offset_angvel`.
    ///
    /// NOTE: The character's rotation can also be locked to prevent it from being tilted, in which
    /// case this paramter is redundant and can be set to 0.0.
    pub tilt_offset_angacl: f32,

    /// The maximum angular velocity used for turning the character when the direction changes.
    pub turning_angvel: f32,
}

#[derive(Debug)]
pub enum TnuaFreeFallBehavior {
    /// Apply extra gravitiy during free fall.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    ///
    /// **NOTE**: If the parameter set to this option is too low, the character may be able to run
    /// up a slope and "jump" potentially even higher than a regular jump, even without pressing
    /// the jump button.
    ExtraGravity(f32),

    /// Treat free fall like jump shortening.
    ///
    /// This means that as long as the character has an upward velocity
    /// [`jump_shorten_extra_gravity`](TnuaPlatformerConfig::jump_shorten_extra_gravity) will be in
    /// effect, and after the character's vertical velocity turns downward
    /// [`jump_fall_extra_gravity`](TnuaPlatformerConfig::jump_fall_extra_gravity) will take over.
    ///
    /// **NOTE**: If
    /// [`jump_shorten_extra_gravity`](TnuaPlatformerConfig::jump_shorten_extra_gravity) is too
    /// low, the character may be able to run up a slope and "jump" potentially even higher than a
    /// regular jump, even without pressing the jump button.
    LikeJumpShorten,

    /// Treat free fall like falling after a jump.
    ///
    /// This means that ['jump_fall_extra_gravity'](TnuaPlatformerConfig::jump_fall_extra_gravity)
    /// will take effect immediately when the character starts a free fall, even if they have
    /// upward velocity.
    ///
    /// **NOTE**: If [`jump_fall_extra_gravity`](TnuaPlatformerConfig::jump_fall_extra_gravity) is
    /// too low, the character may be able to run up a slope and "jump" potentially even higher
    /// than a regular jump, even without pressing the jump button.
    LikeJumpFall,
}

#[derive(Component)]
pub struct TnuaPlatformerControls {
    pub desired_velocity: Vec3,
    pub desired_forward: Vec3,
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
    FreeFall,
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

impl Default for TnuaPlatformerControls {
    fn default() -> Self {
        Self {
            desired_velocity: Vec3::ZERO,
            desired_forward: Vec3::ZERO,
            jump: None,
        }
    }
}

#[derive(Component, Default)]
pub struct TnuaManualTurningOutput {
    pub forward: Vec3,
}

#[derive(Component, Default)]
pub struct TnuaPlatformerAnimatingOutput {
    pub running_velocity: Vec3,
    pub jumping_velocity: Option<f32>,
}

#[allow(clippy::type_complexity)]
fn platformer_control_system(
    time: Res<Time>,
    mut query: Query<(
        &GlobalTransform,
        &TnuaPlatformerControls,
        &TnuaPlatformerConfig,
        &mut TnuaPlatformerState,
        &TnuaRigidBodyTracker,
        &mut TnuaProximitySensor,
        &mut TnuaMotor,
        Option<&mut TnuaManualTurningOutput>,
        Option<&mut TnuaPlatformerAnimatingOutput>,
    )>,
) {
    let frame_duration = time.delta().as_secs_f32();
    if frame_duration == 0.0 {
        return;
    }
    for (
        transform,
        controls,
        config,
        mut platformer_state,
        tracker,
        mut sensor,
        mut motor,
        manual_turning_output,
        mut animating_output,
    ) in query.iter_mut()
    {
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        sensor.cast_range = config.float_height + config.cling_distance;

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

        let effective_velocity: Vec3;
        let climb_vectors: Option<ClimbVectors>;
        let considered_in_air: bool;

        if let Some(sensor_output) = &sensor.output {
            effective_velocity = tracker.velocity - sensor_output.entity_linvel;
            let sideways_unnormalized = sensor_output.normal.cross(config.up);
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
            considered_in_air = match platformer_state.jump_state {
                JumpState::NoJump => false,
                JumpState::FreeFall => true,
                JumpState::StartingJump { .. } => false,
                JumpState::SlowDownTooFastSlopeJump { .. } => true,
                JumpState::MaintainingJump => true,
                JumpState::StoppedMaintainingJump => true,
                JumpState::FallSection => true,
            };
        } else {
            effective_velocity = tracker.velocity;
            climb_vectors = None;
            considered_in_air = true;
        }

        let upward_velocity = config.up.dot(effective_velocity);

        let velocity_on_plane = effective_velocity - config.up * upward_velocity;

        let desired_velocity = controls.desired_velocity * config.full_speed;
        let exact_acceleration = desired_velocity - velocity_on_plane;

        let safe_direction_coefficient = desired_velocity
            .normalize_or_zero()
            .dot(velocity_on_plane.normalize_or_zero());
        let direction_change_factor = 1.5 - 0.5 * safe_direction_coefficient;

        let relevant_acceleration_limit = if considered_in_air {
            config.air_acceleration
        } else {
            config.acceleration
        };
        let acceleration = direction_change_factor * relevant_acceleration_limit;

        let walk_acceleration = exact_acceleration.clamp_length_max(frame_duration * acceleration);

        let walk_acceleration = if let Some(climb_vectors) = &climb_vectors {
            climb_vectors.project(walk_acceleration)
        } else {
            walk_acceleration
        };

        let vertical_velocity = if let Some(climb_vectors) = &climb_vectors {
            effective_velocity.dot(climb_vectors.direction) * climb_vectors.direction.dot(config.up)
        } else {
            0.0
        };

        // TODO: Do I need maximum force capping?

        let upward_impulse: Vec3 = 'upward_impulse: {
            // TODO: Once `std::mem::variant_count` gets stabilized, use that instead. The idea is
            // to allow jumping through multiple states but failing if we get into loop.
            for _ in 0..7 {
                match platformer_state.jump_state {
                    JumpState::NoJump => {
                        if let Some(sensor_output) = &sensor.output {
                            if let Some(jump_multiplier) = controls.jump {
                                let jump_height = jump_multiplier * config.full_jump_height;
                                let gravity = tracker.gravity.dot(-config.up);
                                platformer_state.jump_state = JumpState::StartingJump {
                                    desired_energy: gravity * jump_height,
                                };
                                continue;
                            } else {
                                let spring_offset = config.float_height - sensor_output.proximity;
                                let spring_force: f32 = spring_offset * config.spring_strengh;

                                let relative_velocity =
                                    effective_velocity.dot(config.up) - vertical_velocity;

                                let dampening_force = relative_velocity * config.spring_dampening;
                                let spring_force = spring_force - dampening_force;

                                let gravity_compensation = -tracker.gravity.dot(config.up);
                                break 'upward_impulse frame_duration
                                    * (spring_force + gravity_compensation)
                                    * config.up;
                            }
                        } else {
                            platformer_state.jump_state = JumpState::FreeFall;
                            continue;
                        }
                    }
                    JumpState::FreeFall => match config.free_fall_behavior {
                        TnuaFreeFallBehavior::ExtraGravity(extra_gravity) => {
                            if sensor.output.is_some() {
                                platformer_state.jump_state = JumpState::NoJump;
                                continue;
                            }
                            break 'upward_impulse -(frame_duration * extra_gravity) * config.up;
                        }
                        TnuaFreeFallBehavior::LikeJumpShorten => {
                            platformer_state.jump_state = JumpState::StoppedMaintainingJump;
                            continue;
                        }
                        TnuaFreeFallBehavior::LikeJumpFall => {
                            platformer_state.jump_state = JumpState::FallSection;
                            continue;
                        }
                    },
                    JumpState::StartingJump { desired_energy } => {
                        if let Some(sensor_output) = &sensor.output {
                            let relative_velocity =
                                effective_velocity.dot(config.up) - vertical_velocity.max(0.0);
                            let extra_height = sensor_output.proximity - config.float_height;
                            let gravity = tracker.gravity.dot(-config.up);
                            let energy_from_extra_height = extra_height * gravity;
                            let desired_kinetic_energy = desired_energy - energy_from_extra_height;
                            let desired_upward_velocity = (2.0 * desired_kinetic_energy).sqrt();

                            if config.float_height < sensor_output.proximity {
                                platformer_state.jump_state = JumpState::SlowDownTooFastSlopeJump {
                                    desired_energy,
                                    zero_potential_energy_at: translation
                                        - extra_height * config.up,
                                };
                            }

                            break 'upward_impulse (desired_upward_velocity - relative_velocity)
                                * config.up;
                        } else {
                            platformer_state.jump_state = JumpState::SlowDownTooFastSlopeJump {
                                desired_energy,
                                zero_potential_energy_at: translation,
                            };
                            continue;
                        }
                    }
                    JumpState::SlowDownTooFastSlopeJump {
                        desired_energy,
                        zero_potential_energy_at,
                    } => {
                        if upward_velocity <= vertical_velocity {
                            platformer_state.jump_state = JumpState::FallSection;
                            continue;
                        } else if controls.jump.is_none() {
                            platformer_state.jump_state = JumpState::StoppedMaintainingJump;
                            continue;
                        }
                        let relative_velocity = effective_velocity.dot(config.up);
                        let extra_height = (translation - zero_potential_energy_at).dot(config.up);
                        let gravity = tracker.gravity.dot(-config.up);
                        let energy_from_extra_height = extra_height * gravity;
                        let desired_kinetic_energy = desired_energy - energy_from_extra_height;
                        let desired_upward_velocity = (2.0 * desired_kinetic_energy).sqrt();
                        if relative_velocity <= desired_upward_velocity {
                            platformer_state.jump_state = JumpState::MaintainingJump;
                            continue;
                        } else {
                            break 'upward_impulse -(frame_duration
                                * config.jump_start_extra_gravity)
                                * config.up;
                        }
                    }
                    JumpState::MaintainingJump => {
                        if upward_velocity <= vertical_velocity {
                            platformer_state.jump_state = JumpState::FallSection;
                            continue;
                        } else if controls.jump.is_none() {
                            platformer_state.jump_state = JumpState::StoppedMaintainingJump;
                            continue;
                        }
                        break 'upward_impulse Vec3::ZERO;
                    }
                    JumpState::StoppedMaintainingJump => {
                        if upward_velocity <= 0.0 {
                            platformer_state.jump_state = JumpState::FallSection;
                            continue;
                        }
                        break 'upward_impulse -(frame_duration
                            * config.jump_shorten_extra_gravity)
                            * config.up;
                    }
                    JumpState::FallSection => {
                        if let Some(sensor_output) = &sensor.output {
                            if sensor_output.proximity <= config.float_height {
                                platformer_state.jump_state = JumpState::NoJump;
                                continue;
                            }
                        }
                        break 'upward_impulse -(frame_duration * config.jump_fall_extra_gravity)
                            * config.up;
                    }
                }
            }
            error!("Tnua could not decide on jump state");
            Vec3::ZERO
        };

        motor.desired_acceleration = walk_acceleration + upward_impulse;

        let torque_to_fix_tilt = {
            let tilted_up = rotation.mul_vec3(config.up);

            let rotation_required_to_fix_tilt = Quat::from_rotation_arc(tilted_up, config.up);

            let desired_angvel = (rotation_required_to_fix_tilt.xyz() / frame_duration)
                .clamp_length_max(config.tilt_offset_angvel);
            let angular_velocity_diff = desired_angvel - tracker.angvel;
            angular_velocity_diff.clamp_length_max(frame_duration * config.tilt_offset_angacl)
        };

        struct ProjectionPlaneForRotation(Vec3, Vec3);

        impl ProjectionPlaneForRotation {
            fn from_config(config: &TnuaPlatformerConfig) -> Self {
                Self(config.forward, config.up.cross(config.forward))
            }

            fn project_and_normalize(&self, vector: Vec3) -> Vec2 {
                Vec2::new(vector.dot(self.0), vector.dot(self.1)).normalize_or_zero()
            }
        }

        if let Some(mut manual_turning_output) = manual_turning_output {
            if manual_turning_output.forward == Vec3::ZERO {
                manual_turning_output.forward = if controls.desired_forward == Vec3::ZERO {
                    config.forward
                } else {
                    controls.desired_forward
                }
            } else if manual_turning_output.forward != Vec3::ZERO {
                let projection = ProjectionPlaneForRotation::from_config(config);

                let rotation_to_set_forward = Quat::from_rotation_arc_2d(
                    projection.project_and_normalize(manual_turning_output.forward),
                    projection.project_and_normalize(controls.desired_forward),
                );
                // NOTE: On this 2D plane we projected into, Z is up.
                let rotation_along_up_axis = rotation_to_set_forward.xyz().z * std::f32::consts::PI;

                let max_rotation_this_frame = frame_duration * config.turning_angvel;
                let angvel_along_up_axis =
                    rotation_along_up_axis.clamp(-max_rotation_this_frame, max_rotation_this_frame);
                let rotation = Quat::from_axis_angle(config.up, angvel_along_up_axis);

                let new_forward = rotation.mul_vec3(manual_turning_output.forward);
                if new_forward.distance_squared(controls.desired_forward) < 0.000_1 {
                    // Because from_rotation_arc_2d is not accurate for small angles
                    manual_turning_output.forward = controls.desired_forward;
                } else {
                    manual_turning_output.forward = new_forward;
                }
            }
        }

        let torque_to_turn = {
            let desired_angvel = if 0.0 < controls.desired_forward.length_squared() {
                let projection = ProjectionPlaneForRotation::from_config(config);

                let current_forward = rotation.mul_vec3(config.forward);
                let rotation_to_set_forward = Quat::from_rotation_arc_2d(
                    projection.project_and_normalize(current_forward),
                    projection.project_and_normalize(controls.desired_forward),
                );
                // NOTE: On this 2D plane we projected into, Z is up.
                let rotation_along_up_axis = rotation_to_set_forward.xyz().z;
                (rotation_along_up_axis / frame_duration)
                    .clamp(-config.turning_angvel, config.turning_angvel)
            } else {
                0.0
            };

            // NOTE: This is the regular axis system so we used the configured up.
            let existing_angvel = tracker.angvel.dot(config.up);

            // This is the torque. Should it be clamped by an acceleration? From experimenting with
            // this I think it's meaningless and only causes bugs.
            desired_angvel - existing_angvel
        };

        let existing_turn_torque = torque_to_fix_tilt.dot(config.up);
        let turn_torque_to_offset = torque_to_turn - existing_turn_torque;

        motor.desired_angacl = torque_to_fix_tilt + turn_torque_to_offset * config.up;

        if let Some(animating_output) = animating_output.as_mut() {
            let new_velocity = effective_velocity + motor.desired_acceleration;
            let new_upward_velocity = config.up.dot(new_velocity);
            animating_output.running_velocity = new_velocity - config.up * new_upward_velocity;
            let is_airborne = if sensor.output.is_some() {
                match platformer_state.jump_state {
                    JumpState::NoJump => false,
                    JumpState::FreeFall => true,
                    JumpState::StartingJump { .. } => true,
                    JumpState::SlowDownTooFastSlopeJump { .. } => true,
                    JumpState::MaintainingJump => true,
                    JumpState::StoppedMaintainingJump => true,
                    JumpState::FallSection => true,
                }
            } else {
                true
            };
            animating_output.jumping_velocity = is_airborne.then_some(new_upward_velocity);
        }
    }
}

#[derive(Default, Debug)]
#[allow(dead_code)]
struct ClimbInfo {
    climb_direction: Vec3,
    climb_per_unit: f32,
}
