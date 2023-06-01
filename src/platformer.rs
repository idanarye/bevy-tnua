use std::time::Duration;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use crate::subservient_sensors::TnuaSubservientSensor;
use crate::util::SegmentedJumpInitialVelocityCalculator;
use crate::{
    TnuaMotor, TnuaPipelineStages, TnuaProximitySensor, TnuaRigidBodyTracker, TnuaSystemSet,
    TnuaVelChange,
};

pub struct TnuaPlatformerPlugin;

/// The main for supporting platformer-like controls.
///
/// It's called "platformer", but it can also be used for other types of games, like shooters. It
/// won't work very well for vehicle controls though.
impl Plugin for TnuaPlatformerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TnuaPlatformerConfig>();
        app.register_type::<TnuaFreeFallBehavior>();

        app.configure_sets(
            (
                TnuaPipelineStages::Sensors,
                TnuaPipelineStages::SubservientSensors,
                TnuaPipelineStages::Logic,
                TnuaPipelineStages::Motors,
            )
                .chain()
                .in_set(TnuaSystemSet),
        );
        app.add_system(platformer_control_system.in_set(TnuaPipelineStages::Logic));
        app.add_system(
            handle_keep_crouching_below_obstacles.in_set(TnuaPipelineStages::SubservientSensors),
        );
    }
}

/// All the Tnua components needed for a platformer-like character controller.
///
/// Note that:
///
/// * While this bundle has a default which provides a workable setting for
///   [`TnuaPlatformerConfig`], this is only done so that this bundle can be created with the
///   `..Default::default()` syntax. Users are expected to use provide their own
///   [`TnuaPlatformerConfig`], customized for the specific game they are making.
///
/// * This bundle only contains components defined by Tnua. Rapier controllers need to be added
///   manually.
///
/// * That this does not include optional components like [`TnuaManualTurningOutput`] or
///   [`TnuaPlatformerAnimatingOutput`].
#[derive(Bundle)]
pub struct TnuaPlatformerBundle {
    pub config: TnuaPlatformerConfig,
    pub controls: TnuaPlatformerControls,
    pub motor: TnuaMotor,
    pub rigid_body_tracker: TnuaRigidBodyTracker,
    pub proximity_sensor: TnuaProximitySensor,
    pub state: TnuaPlatformerState,
}

impl Default for TnuaPlatformerBundle {
    fn default() -> Self {
        Self {
            config: TnuaPlatformerConfig {
                full_speed: 20.0,
                full_jump_height: 4.0,
                up: Vec3::Y,
                forward: -Vec3::Z,
                float_height: 2.0,
                cling_distance: 1.0,
                spring_strengh: 400.0,
                spring_dampening: 1.2,
                acceleration: 60.0,
                air_acceleration: 20.0,
                coyote_time: 0.15,
                jump_input_buffer_time: 0.2,
                held_jump_cooldown: None,
                jump_start_extra_gravity: 30.0,
                jump_takeoff_extra_gravity: 30.0,
                jump_takeoff_above_velocity: 3.0,
                jump_fall_extra_gravity: 20.0,
                jump_shorten_extra_gravity: 40.0,
                jump_peak_prevention_at_upward_velocity: 0.0,
                jump_peak_prevention_extra_gravity: 20.0,
                free_fall_behavior: TnuaFreeFallBehavior::LikeJumpShorten,
                tilt_offset_angvel: 5.0,
                tilt_offset_angacl: 500.0,
                turning_angvel: 10.0,
                height_change_impulse_for_duration: 0.02,
                height_change_impulse_limit: 40.0,
            },
            controls: Default::default(),
            motor: Default::default(),
            rigid_body_tracker: Default::default(),
            proximity_sensor: Default::default(),
            state: Default::default(),
        }
    }
}

/// Movement settings for a platformer-like character controlled by Tnua.
#[derive(Component, Reflect)]
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
    /// `Vec3::X` for 2D sprites (character turning left) and `-Vec3::Z` for 3D (character model
    /// faced in camera's forward direction)
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
    ///
    /// Note that as this approaches 2.0, the character starts to shake violently and eventually
    /// get launched upward at great speed.
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

    /// The time, in seconds, the character can still jump after losing their footing.
    pub coyote_time: f32,

    /// A duration, in seconds, where a player can press a jump button before a jump becomes
    /// possible (typically when a character is still in the air and about the land) and the jump
    /// command would still get registered and be executed once the jump is possible.
    pub jump_input_buffer_time: f32,

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
    /// [`jump_input_buffer_time`](Self::jump_input_buffer_time)), this setting will have no effect
    /// because the character will simply jump immediately.
    pub held_jump_cooldown: Option<f32>,

    /// Extra gravity for breaking too fast jump from running up a slope.
    ///
    /// When running up a slope, the character gets more jump strength to avoid slamming into the
    /// slope. This may cause the jump to be too high, so this value is used to brake it.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub jump_start_extra_gravity: f32,

    /// Extra gravity for fast takeoff.
    ///
    /// Without this, jumps feel painfully slow. Adding this will apply extra gravity until the
    /// vertical velocity reaches below
    /// [`jump_takeoff_above_velocity`](Self::jump_takeoff_above_velocity), and increase the
    /// initial jump boost in order to compensate. This will make the jump feel more snappy.
    pub jump_takeoff_extra_gravity: f32,

    /// The range of upward velocity during
    /// [`jump_takeoff_extra_gravity`](Self::jump_takeoff_extra_gravity) is applied.
    ///
    /// To disable, set this to [`f32::INFINITY`] rather than zero.
    pub jump_takeoff_above_velocity: f32,

    /// Extra gravity for falling down after reaching the top of the jump.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub jump_fall_extra_gravity: f32,

    /// Extra gravity for shortening a jump when the player releases the jump button.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub jump_shorten_extra_gravity: f32,

    /// Used to decrease the time the character spends "floating" at the peak of the jump.
    ///
    /// When the character's upward velocity is above this value,
    /// [`jump_peak_prevention_extra_gravity`](Self::jump_peak_prevention_extra_gravity) will be
    /// added to the gravity in order to shorten the float time.
    ///
    /// This extra gravity is taken into account when calculating the initial jump speed, so the
    /// character is still supposed to reach its [`full_jump_height`](Self::full_jump_height).
    pub jump_peak_prevention_at_upward_velocity: f32,

    /// Extra gravity for decreasing the time the character spends at the peak of the jump.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub jump_peak_prevention_extra_gravity: f32,

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

    /// A duration, in seconds, that it should take for the character to change its floating height
    /// when the [`float_height_offset`](TnuaPlatformerControls::float_height_offset) control
    /// field is changed.
    ///
    /// Set this to more than the expected duration of a single frame, so that the character will
    /// some distance for the [`spring_dampening`](Self::spring_dampening) force to reduce its
    /// vertical velocity.
    pub height_change_impulse_for_duration: f32,

    /// The maximum impulse to apply when
    /// [`float_height_offset`](TnuaPlatformerControls::float_height_offset) control field is
    /// changed.
    pub height_change_impulse_limit: f32,
}

#[derive(Debug, Reflect)]
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

/// Edit this component in a system to control the character.
///
/// Tnua does not write to `TnuaPlatformerControls` - only reads from it - so it should be updated
/// every frame.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_tnua::{TnuaPlatformerControls};
/// fn player_control_system(mut query: Query<&mut TnuaPlatformerControls>) {
///     for mut controls in query.iter_mut() {
///         *controls = TnuaPlatformerControls {
///             desired_velocity: Vec3::X, // always go right for some reason
///             desired_forward: -Vec3::X, // face backwards from walking direction
///             jump: None, // no jumping
///             float_height_offset: 0.0, // not crouching,
///         };
///     }
/// }
/// ```
#[derive(Component)]
pub struct TnuaPlatformerControls {
    /// The direction to go in, in the world space, as a fraction of the
    /// [`full_speed`](TnuaPlatformerConfig::full_speed) (so a lenght of 1 is full speed)
    ///
    /// Tnua assumes that this vector is orthogonal to the [`up`](TnuaPlatformerConfig::up) vector.
    pub desired_velocity: Vec3,

    /// If non-zero, Tnua will rotate the character to face in that direction.
    ///
    /// Tnua assumes that this vector is orthogonal to the [`up`](TnuaPlatformerConfig::up) vector.
    pub desired_forward: Vec3,

    /// Instructs the character to jump. The number is a fraction of the
    /// [`full_jump_height`](TnuaPlatformerConfig::full_jump_height) (so a height of 1 is full
    /// height)
    ///
    /// For variable height jumping based on button press length, don't bother calculating the
    /// number - just set this to `Some(1.0)` and let Tnua handle the variable height with the
    /// [`jump_shorten_extra_gravity`](TnuaPlatformerConfig::jump_shorten_extra_gravity) setting
    /// (which should be hight than 0 to support this). Only set it to a number lower or higher
    /// than 1 if the height is calculated on something like an analog button press strenght or an
    /// AI that needs to decide exactly how high to jump.
    pub jump: Option<f32>,

    /// An offset from the regular float height. Setting this to a negative number will make the
    /// character crouch.
    ///
    /// To prevent the character from standing up while under a low ceiling, use
    /// [`TnuaKeepCrouchingBelowObstacles`].
    ///
    /// Prefer this over manipulating [`float_height`](TnuaPlatformerConfig::float_height) during
    /// gameplay, because:
    /// * Changing `float_height_offset` will make the transition between float heights faster by
    ///   applying a one shot boost impulse (can be configured with the
    ///   [`height_change_impulse_for_duration`](TnuaPlatformerConfig::height_change_impulse_for_duration)
    ///   and [`height_change_impulse_limit`](TnuaPlatformerConfig::height_change_impulse_limit)
    ///   settings) when `float_height_offset` changes.
    /// * When `float_height_offset` is negative, the raycast will still reach the same lenght as
    ///   it would for the base float height. This means that
    ///   [`cling_distance`](TnuaPlatformerConfig::cling_distance) does not need to be big enough
    ///   to cover the crouch offset.
    pub float_height_offset: f32,
}

impl Default for TnuaPlatformerControls {
    fn default() -> Self {
        Self {
            desired_velocity: Vec3::ZERO,
            desired_forward: Vec3::ZERO,
            jump: None,
            float_height_offset: 0.0,
        }
    }
}

#[doc(hidden)]
#[derive(Component, Default, Debug)]
pub struct TnuaPlatformerState {
    jump_command_state: JumpCommandState,
    jump_state: JumpState,
    standing_on: Option<StandingOnState>,
    prev_float_height_offset: f32,
}

#[derive(Default, Debug)]
enum JumpCommandState {
    #[default]
    Unissued,
    Consumed,
    Buffered(Timer),
    Cooldown(Timer),
}

#[derive(Default, Debug)]
enum JumpState {
    #[default]
    NoJump,
    FreeFall {
        coyote_time: Timer,
    },
    StartingJump {
        /// The potential energy at the top of the jump, when:
        /// * The potential energy at the bottom of the jump is defined as 0
        /// * The mass is 1
        /// Calculating the desired velocity based on energy is easier than using the ballistic
        /// formulas.
        desired_energy: f32,
        coyote_time: Timer,
    },
    SlowDownTooFastSlopeJump {
        desired_energy: f32,
        zero_potential_energy_at: Vec3,
    },
    MaintainingJump,
    StoppedMaintainingJump {
        coyote_time: Timer,
    },
    FallSection {
        coyote_time: Timer,
    },
}

#[derive(Debug)]
struct StandingOnState {
    entity: Entity,
    entity_linvel: Vec3,
}

/// If added as component, Tnua will update its `forward` field instead of rotating the rigid body.
///
/// This is useful for controlling the rotation via a system - e.g. when working with 2D and the
/// physics engine cannot handle the rotation, so it should be done with a sprite animation
/// instead.
#[derive(Component, Default)]
pub struct TnuaManualTurningOutput {
    pub forward: Vec3,
}

/// If added as component, Tnua will update its fields so that they can used to decide which
/// animation to play and at which speed.
///
/// See [`TnuaAnimatingState`](crate::TnuaAnimatingState) for usage example.
#[derive(Component, Default)]
pub struct TnuaPlatformerAnimatingOutput {
    /// The current running velocity on a plane orthogonal to the [`up`](TnuaPlatformerConfig::up)
    /// vector.
    pub running_velocity: Vec3,

    /// The current jumping velocity on the [`up`](TnuaPlatformerConfig::up), or `None` if the
    /// character is not currently jumping.
    pub jumping_velocity: Option<f32>,

    /// When the character is standing, this is the offset from the configured
    /// [`float_height`](TnuaPlatformerConfig::float_height).
    ///
    /// Note that this value does not take the
    /// [`float_height_offset`](TnuaPlatformerControls::float_height_offset) control field into
    /// account. This means that the value of `standing_offset` should be close to that of
    /// `float_height_offset` (after the transition time, of course), and can be used to determine
    /// if the character is standing or crouching.
    pub standing_offset: f32,
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
        Option<&TnuaKeepCrouchingBelowObstacles>,
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
        keep_crouching,
        mut motor,
        manual_turning_output,
        mut animating_output,
    ) in query.iter_mut()
    {
        match &mut platformer_state.jump_state {
            JumpState::NoJump
            | JumpState::MaintainingJump
            | JumpState::SlowDownTooFastSlopeJump {
                desired_energy: _,
                zero_potential_energy_at: _,
            } => {}

            JumpState::FreeFall { coyote_time }
            | JumpState::StartingJump {
                desired_energy: _,
                coyote_time,
            }
            | JumpState::StoppedMaintainingJump { coyote_time }
            | JumpState::FallSection { coyote_time } => {
                coyote_time.tick(time.delta());
            }
        }

        let (_, rotation, translation) = transform.to_scale_rotation_translation();

        let float_height_offset = if let Some(keep_crouching) = keep_crouching {
            controls
                .float_height_offset
                .min(keep_crouching.force_crouching_to_height)
        } else {
            controls.float_height_offset
        };

        sensor.cast_range =
            config.float_height + config.cling_distance + float_height_offset.max(0.0);

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
        let impulse_to_offset: Vec3;

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
                JumpState::FreeFall { .. } => true,
                JumpState::StartingJump { .. } => false,
                JumpState::SlowDownTooFastSlopeJump { .. } => true,
                JumpState::MaintainingJump => true,
                JumpState::StoppedMaintainingJump { .. } => true,
                JumpState::FallSection { .. } => true,
            };
            if considered_in_air {
                impulse_to_offset = Vec3::ZERO;
            } else if let Some(standing_on_state) = &platformer_state.standing_on {
                if standing_on_state.entity != sensor_output.entity {
                    impulse_to_offset = Vec3::ZERO;
                } else {
                    impulse_to_offset =
                        sensor_output.entity_linvel - standing_on_state.entity_linvel;
                }
            } else {
                impulse_to_offset = Vec3::ZERO;
            }
        } else {
            effective_velocity = tracker.velocity;
            climb_vectors = None;
            considered_in_air = true;
            impulse_to_offset = Vec3::ZERO;
        }
        let effective_velocity = effective_velocity + impulse_to_offset;

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

        fn make_finished_timer() -> Timer {
            let mut result = Timer::new(Duration::ZERO, TimerMode::Once);
            result.tick(Duration::ZERO);
            result
        }

        let jump_command_can_be_fired = match &mut platformer_state.jump_command_state {
            JumpCommandState::Unissued => true,
            JumpCommandState::Consumed => false,
            JumpCommandState::Buffered(timer) => !timer.tick(time.delta()).finished(),
            JumpCommandState::Cooldown(timer) => timer.tick(time.delta()).finished(),
        };

        let should_jump_calc_energy = |can_jump: bool| {
            if can_jump && jump_command_can_be_fired {
                if let Some(jump_multiplier) = controls.jump {
                    let jump_height = jump_multiplier * config.full_jump_height;
                    let mut calculator = SegmentedJumpInitialVelocityCalculator::new(jump_height);

                    let gravity = tracker.gravity.dot(-config.up);

                    let kinetic_energy = calculator
                        // Jump peak prevention segment
                        .add_segment(
                            gravity + config.jump_peak_prevention_extra_gravity,
                            config.jump_peak_prevention_at_upward_velocity,
                        )
                        // Regular gravity segment
                        .add_segment(gravity, config.jump_takeoff_above_velocity)
                        // Jump takeoff segment
                        .add_segment(gravity + config.jump_takeoff_extra_gravity, f32::INFINITY)
                        .kinetic_energy();
                    Some(kinetic_energy)
                } else {
                    None
                }
            } else {
                None
            }
        };

        let mut standing_offset = 0.0;
        let upward_impulse: TnuaVelChange = 'upward_impulse: {
            // TODO: Once `std::mem::variant_count` gets stabilized, use that instead. The idea is
            // to allow jumping through multiple states but failing if we get into loop.
            for _ in 0..7 {
                match &mut platformer_state.jump_state {
                    JumpState::NoJump => {
                        if let Some(sensor_output) = &sensor.output {
                            if let Some(desired_energy) = should_jump_calc_energy(true) {
                                platformer_state.jump_state = JumpState::StartingJump {
                                    desired_energy,
                                    coyote_time: Timer::new(
                                        Duration::from_secs_f32(config.coyote_time),
                                        TimerMode::Once,
                                    ),
                                };
                                continue;
                            } else {
                                let spring_offset = config.float_height - sensor_output.proximity;
                                standing_offset = -spring_offset;
                                let spring_offset = spring_offset + float_height_offset;
                                let spring_force: f32 = spring_offset * config.spring_strengh;
                                let offset_change_impulse: f32 = if 0.01
                                    <= (float_height_offset
                                        - platformer_state.prev_float_height_offset)
                                        .abs()
                                {
                                    let velocity_to_get_to_new_float_height =
                                        spring_offset / config.height_change_impulse_for_duration;
                                    velocity_to_get_to_new_float_height.clamp(
                                        -config.height_change_impulse_limit,
                                        config.height_change_impulse_limit,
                                    )
                                } else {
                                    0.0
                                };

                                let relative_velocity =
                                    effective_velocity.dot(config.up) - vertical_velocity;

                                let dampening_force =
                                    relative_velocity * config.spring_dampening / frame_duration;
                                let spring_force = spring_force - dampening_force;

                                let gravity_compensation = -tracker.gravity.dot(config.up);

                                let spring_impulse =
                                    frame_duration * (spring_force + gravity_compensation);

                                let impulse_to_use =
                                    if spring_impulse.abs() < offset_change_impulse.abs() {
                                        offset_change_impulse
                                    } else {
                                        spring_impulse
                                    };

                                // TODO: maybe this needs to be an acceleration rather than an
                                // impulse? The problem is the comparison between `spring_impulse`
                                // and `offset_change_impulse`...
                                break 'upward_impulse TnuaVelChange::boost(
                                    impulse_to_use * config.up,
                                );
                            }
                        } else {
                            platformer_state.jump_state = JumpState::FreeFall {
                                coyote_time: Timer::new(
                                    Duration::from_secs_f32(config.coyote_time),
                                    TimerMode::Once,
                                ),
                            };
                            continue;
                        }
                    }
                    JumpState::FreeFall { coyote_time } => match config.free_fall_behavior {
                        TnuaFreeFallBehavior::ExtraGravity(extra_gravity) => {
                            if sensor.output.is_some() {
                                platformer_state.jump_state = JumpState::NoJump;
                                continue;
                            }
                            if let Some(desired_energy) = should_jump_calc_energy(true) {
                                platformer_state.jump_state = JumpState::StartingJump {
                                    desired_energy,
                                    coyote_time: coyote_time.clone(),
                                };
                                continue;
                            }
                            break 'upward_impulse TnuaVelChange::acceleration(
                                -extra_gravity * config.up,
                            );
                        }
                        TnuaFreeFallBehavior::LikeJumpShorten => {
                            platformer_state.jump_state = JumpState::StoppedMaintainingJump {
                                coyote_time: coyote_time.clone(),
                            };
                            continue;
                        }
                        TnuaFreeFallBehavior::LikeJumpFall => {
                            platformer_state.jump_state = JumpState::FallSection {
                                coyote_time: coyote_time.clone(),
                            };
                            continue;
                        }
                    },
                    JumpState::StartingJump {
                        desired_energy,
                        coyote_time,
                    } => {
                        if let Some(sensor_output) = &sensor.output {
                            let relative_velocity =
                                effective_velocity.dot(config.up) - vertical_velocity.max(0.0);
                            let extra_height = sensor_output.proximity - config.float_height;
                            let gravity = tracker.gravity.dot(-config.up);
                            let energy_from_extra_height = extra_height * gravity;
                            let desired_kinetic_energy = *desired_energy - energy_from_extra_height;
                            let desired_upward_velocity = (2.0 * desired_kinetic_energy).sqrt();

                            if config.float_height < sensor_output.proximity {
                                platformer_state.jump_state = JumpState::SlowDownTooFastSlopeJump {
                                    desired_energy: *desired_energy,
                                    zero_potential_energy_at: translation
                                        - extra_height * config.up,
                                };
                            }

                            break 'upward_impulse TnuaVelChange::boost(
                                (desired_upward_velocity - relative_velocity) * config.up,
                            );
                        } else if !coyote_time.finished() {
                            let relative_velocity =
                                effective_velocity.dot(config.up) - vertical_velocity.max(0.0);
                            let desired_upward_velocity = (2.0 * *desired_energy).sqrt();
                            platformer_state.jump_state = JumpState::SlowDownTooFastSlopeJump {
                                desired_energy: *desired_energy,
                                zero_potential_energy_at: translation,
                            };
                            break 'upward_impulse TnuaVelChange::boost(
                                (desired_upward_velocity - relative_velocity) * config.up,
                            );
                        } else {
                            platformer_state.jump_state = JumpState::SlowDownTooFastSlopeJump {
                                desired_energy: *desired_energy,
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
                            platformer_state.jump_state = JumpState::FallSection {
                                coyote_time: make_finished_timer(),
                            };
                            continue;
                        } else if controls.jump.is_none() {
                            platformer_state.jump_state = JumpState::StoppedMaintainingJump {
                                coyote_time: make_finished_timer(),
                            };
                            continue;
                        }
                        let relative_velocity = effective_velocity.dot(config.up);
                        let extra_height = (translation - *zero_potential_energy_at).dot(config.up);
                        let gravity = tracker.gravity.dot(-config.up);
                        let energy_from_extra_height = extra_height * gravity;
                        let desired_kinetic_energy = *desired_energy - energy_from_extra_height;
                        let desired_upward_velocity = (2.0 * desired_kinetic_energy).sqrt();
                        if relative_velocity <= desired_upward_velocity {
                            platformer_state.jump_state = JumpState::MaintainingJump;
                            continue;
                        } else {
                            let mut extra_gravity = config.jump_start_extra_gravity;
                            if config.jump_takeoff_above_velocity <= relative_velocity {
                                extra_gravity += config.jump_takeoff_extra_gravity;
                            }
                            break 'upward_impulse TnuaVelChange::acceleration(
                                -extra_gravity * config.up,
                            );
                        }
                    }
                    JumpState::MaintainingJump => {
                        let relevant_upwrad_velocity = upward_velocity - vertical_velocity;
                        if relevant_upwrad_velocity <= 0.0 {
                            platformer_state.jump_state = JumpState::FallSection {
                                coyote_time: make_finished_timer(),
                            };
                            continue;
                        } else if config.jump_takeoff_above_velocity <= relevant_upwrad_velocity {
                            break 'upward_impulse TnuaVelChange::acceleration(
                                -config.jump_takeoff_extra_gravity * config.up,
                            );
                        } else if relevant_upwrad_velocity
                            < config.jump_peak_prevention_at_upward_velocity
                        {
                            break 'upward_impulse TnuaVelChange::acceleration(
                                -config.jump_peak_prevention_extra_gravity * config.up,
                            );
                        } else if controls.jump.is_none() {
                            platformer_state.jump_state = JumpState::StoppedMaintainingJump {
                                coyote_time: make_finished_timer(),
                            };
                            continue;
                        }
                        break 'upward_impulse TnuaVelChange::ZERO;
                    }
                    JumpState::StoppedMaintainingJump { coyote_time } => {
                        if upward_velocity <= 0.0 {
                            platformer_state.jump_state = JumpState::FallSection {
                                coyote_time: coyote_time.clone(),
                            };
                            continue;
                        }
                        if let Some(desired_energy) =
                            should_jump_calc_energy(!coyote_time.finished())
                        {
                            platformer_state.jump_state = JumpState::StartingJump {
                                desired_energy,
                                coyote_time: coyote_time.clone(),
                            };
                            continue;
                        }
                        break 'upward_impulse TnuaVelChange::acceleration(
                            -config.jump_shorten_extra_gravity * config.up,
                        );
                    }
                    JumpState::FallSection { coyote_time } => {
                        if let Some(sensor_output) = &sensor.output {
                            if sensor_output.proximity <= config.float_height {
                                platformer_state.jump_state = JumpState::NoJump;
                                continue;
                            }
                        }
                        if let Some(desired_energy) =
                            should_jump_calc_energy(!coyote_time.finished())
                        {
                            platformer_state.jump_state = JumpState::StartingJump {
                                desired_energy,
                                coyote_time: coyote_time.clone(),
                            };
                            continue;
                        }
                        break 'upward_impulse TnuaVelChange::acceleration(
                            -config.jump_fall_extra_gravity * config.up,
                        );
                    }
                }
            }
            error!("Tnua could not decide on jump state");
            TnuaVelChange::ZERO
        };
        platformer_state.prev_float_height_offset = float_height_offset;

        motor.lin = TnuaVelChange::boost(walk_acceleration + impulse_to_offset) + upward_impulse;

        if controls.jump.is_some() {
            if jump_command_can_be_fired {
                match platformer_state.jump_state {
                    JumpState::StartingJump { .. } | JumpState::MaintainingJump => {
                        platformer_state.jump_command_state = JumpCommandState::Consumed
                    }
                    JumpState::NoJump
                    | JumpState::FreeFall { .. }
                    | JumpState::SlowDownTooFastSlopeJump { .. }
                    | JumpState::StoppedMaintainingJump { .. }
                    | JumpState::FallSection { .. } => {
                        if !matches!(
                            platformer_state.jump_command_state,
                            JumpCommandState::Buffered(_)
                        ) {
                            platformer_state.jump_command_state = JumpCommandState::Buffered(
                                Timer::from_seconds(config.jump_input_buffer_time, TimerMode::Once),
                            );
                        }
                    }
                };
            } else if matches!(
                platformer_state.jump_command_state,
                JumpCommandState::Consumed
            ) {
                let make_cooldown = || {
                    if let Some(cooldown) = config.held_jump_cooldown {
                        JumpCommandState::Cooldown(Timer::from_seconds(cooldown, TimerMode::Once))
                    } else {
                        JumpCommandState::Consumed
                    }
                };
                platformer_state.jump_command_state = match &platformer_state.jump_state {
                    JumpState::NoJump => make_cooldown(),
                    JumpState::FreeFall { coyote_time }
                    | JumpState::StoppedMaintainingJump { coyote_time }
                    | JumpState::FallSection { coyote_time } => {
                        if coyote_time.finished() {
                            JumpCommandState::Consumed
                        } else {
                            make_cooldown()
                        }
                    }
                    JumpState::StartingJump { .. }
                    | JumpState::SlowDownTooFastSlopeJump { .. }
                    | JumpState::MaintainingJump => JumpCommandState::Consumed,
                };
            }
        } else {
            platformer_state.jump_command_state = JumpCommandState::Unissued;
        }

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

        let turn_torque_to_offset = if let Some(mut manual_turning_output) = manual_turning_output {
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
            0.0
        } else {
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
            torque_to_turn - existing_turn_torque
        };

        motor.ang = TnuaVelChange::boost(torque_to_fix_tilt + turn_torque_to_offset * config.up);

        let is_airborne = match &platformer_state.jump_state {
            JumpState::NoJump => false,
            JumpState::SlowDownTooFastSlopeJump { .. } => true,
            JumpState::MaintainingJump => true,
            JumpState::FreeFall { coyote_time }
            | JumpState::StartingJump {
                desired_energy: _,
                coyote_time,
            }
            | JumpState::StoppedMaintainingJump { coyote_time }
            | JumpState::FallSection { coyote_time } => coyote_time.finished(),
        };

        if is_airborne {
            platformer_state.standing_on = None;
        } else if let Some(sensor_output) = &sensor.output {
            platformer_state.standing_on = Some(StandingOnState {
                entity: sensor_output.entity,
                entity_linvel: sensor_output.entity_linvel,
            });
        }
        // NOTE: In cases like Coyote time the `standing_on` will not change.

        if let Some(animating_output) = animating_output.as_mut() {
            let new_velocity = effective_velocity + motor.lin.boost - impulse_to_offset;
            let new_upward_velocity = config.up.dot(new_velocity);
            animating_output.running_velocity = new_velocity - config.up * new_upward_velocity;
            animating_output.jumping_velocity = is_airborne.then_some(new_upward_velocity);
            animating_output.standing_offset = standing_offset;
        }
    }
}

/// Prevent the character from standing up if the player releases the crouch button while under an
/// obstacle.
///
/// This will create a child entity with a proximity sensor pointed upward. When that sensor senses
/// a ceiling, it will prevent the height offset from increasing - even if the
/// [`float_height_offset`](TnuaPlatformerControls::float_height_offset) control field raises.
#[derive(Component)]
pub struct TnuaKeepCrouchingBelowObstacles {
    sensor_entity: Option<Entity>,
    detection_height: f32,
    modify_sensor: Box<dyn Send + Sync + Fn(&mut EntityCommands)>,
    /// The current crouch state of the character. Read it to determine if the character is
    /// crawling and thus its speed needs to be reduced.
    pub force_crouching_to_height: f32,
}

impl TnuaKeepCrouchingBelowObstacles {
    /// Create a new [`TnuaKeepCrouchingBelowObstacles`].
    ///
    /// # Arguments
    ///
    /// * `detection_height`: The distance, from the character's origin, to cast a ray that looks
    ///   for for a ceiling. Set this to be exactly enough to detect a ceiling that'd prevent
    ///   standing up when the player is crouched.
    /// * `modify_sensor`: A closure that operates on the sensor entity when created. Use it to add
    ///   a sensor shape, so that the character will not stand up under the edge of the ceiling and
    ///   may still get stuck trying to stand up.
    pub fn new(
        detection_height: f32,
        modify_sensor: impl 'static + Send + Sync + Fn(&mut EntityCommands),
    ) -> Self {
        Self {
            sensor_entity: None,
            detection_height,
            modify_sensor: Box::new(modify_sensor),
            force_crouching_to_height: f32::INFINITY,
        }
    }
}

fn handle_keep_crouching_below_obstacles(
    mut query: Query<(
        Entity,
        &mut TnuaKeepCrouchingBelowObstacles,
        &TnuaPlatformerControls,
    )>,
    sensors_query: Query<&TnuaProximitySensor, With<TnuaSubservientSensor>>,
    mut commands: Commands,
) {
    for (owner_entity, mut keep_crouching, controls) in query.iter_mut() {
        if let Some(subservient_sensor) = keep_crouching
            .sensor_entity
            .and_then(|entity| sensors_query.get(entity).ok())
        {
            if subservient_sensor.output.is_some() {
                keep_crouching.force_crouching_to_height = keep_crouching
                    .force_crouching_to_height
                    .min(controls.float_height_offset);
            } else {
                keep_crouching.force_crouching_to_height = f32::INFINITY;
            }
        } else {
            let mut cmd = commands.spawn((
                TransformBundle {
                    ..Default::default()
                },
                TnuaSubservientSensor { owner_entity },
                TnuaProximitySensor {
                    cast_direction: Vec3::Y,
                    cast_range: keep_crouching.detection_height,
                    ..Default::default()
                },
            ));
            cmd.set_parent(owner_entity);
            (keep_crouching.modify_sensor)(&mut cmd);
            let sensor_entity = cmd.id();
            keep_crouching.sensor_entity = Some(sensor_entity);
            keep_crouching.force_crouching_to_height = f32::INFINITY;
        }
    }
}
