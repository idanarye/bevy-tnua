use std::time::Duration;

use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::data_for_backends::{
    TnuaGhostSensor, TnuaProximitySensor,
};
use serde::{Deserialize, Serialize};

use crate::TnuaBasis;
use crate::basis_action_traits::TnuaBasisAccess;
use crate::basis_capabilities::{
    TnuaBasisWithDisplacement, TnuaBasisWithFrameOfReferenceSurface, TnuaBasisWithFloating,
    TnuaBasisWithGround, TnuaBasisWithHeadroom, TnuaBasisWithSpring,
};
use crate::ghost_overrides::TnuaGhostOverwrite;
use crate::math::*;
use crate::sensor_sets::{ProximitySensorPreparationHelper, TnuaSensors};
use crate::util::rotation_arc_around_axis;
use crate::{TnuaBasisContext, TnuaMotor, TnuaVelChange};

use super::walk_sensors::TnuaBuiltinWalkSensors;

#[derive(Default)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct TnuaBuiltinWalk {
    /// The direction (in the world space) and speed to accelerate to.
    ///
    /// Tnua assumes that this vector is orthogonal to the up dierction.
    pub desired_motion: Vector3,

    /// If non-zero, Tnua will rotate the character so that its negative Z will face in that
    /// direction.
    ///
    /// Tnua assumes that this vector is orthogonal to the up direction.
    pub desired_forward: Option<Dir3>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TnuaBuiltinWalkConfig {
    // How fast the character will go.
    //
    // Note that this will be the speed when [`desired_motion`](TnuaBuiltinWalk::desired_motion)
    // is a unit vector - meaning that its length is 1.0. If its not 1.0, the speed will be a
    // multiply of that length.
    //
    // Also note that this is the full speed - the character will gradually accelerate to this
    // speed based on the acceleration configuration.
    pub speed: Float,

    /// The height at which the character will float above ground at rest.
    ///
    /// Note that this is the height of the character's center of mass - not the distance from its
    /// collision mesh.
    ///
    /// To make a character crouch, instead of altering this field, prefer to use the
    /// [`TnuaBuiltinCrouch`](crate::builtins::TnuaBuiltinCrouch) action.
    pub float_height: Float,

    /// Add an upward-facing proximity sensor that can check if the character has room above it.
    ///
    /// This is not (currently) used by `TnuaBuiltinWalk` itself, but
    /// [`TnuaBuiltinCrouch`](crate::builtins::TnuaBuiltinCrouch) uses it to determine
    pub headroom: Option<TnuaBuiltinWalkHeadroom>,

    /// Extra distance above the `float_height` where the spring is still in effect.
    ///
    /// When the character is at at most this distance above the
    /// [`float_height`](Self::float_height), the spring force will kick in and move it to the
    /// float height - even if that means pushing it down. If the character is above that distance
    /// above the `float_height`, Tnua will consider it to be in the air.
    pub cling_distance: Float,

    /// The force that pushes the character to the float height.
    ///
    /// The actual force applied is in direct linear relationship to the displacement from the
    /// `float_height`.
    pub spring_strength: Float,

    /// A force that slows down the characters vertical spring motion.
    ///
    /// The actual dampening is in direct linear relationship to the vertical velocity it tries to
    /// dampen.
    ///
    /// Note that as this approaches 2.0, the character starts to shake violently and eventually
    /// get launched upward at great speed.
    pub spring_dampening: Float,

    /// The acceleration for horizontal movement.
    ///
    /// Note that this is the acceleration for starting the horizontal motion and for reaching the
    /// top speed. When braking or changing direction the acceleration is greater, up to 2 times
    /// `acceleration` when doing a 180 turn.
    pub acceleration: Float,

    /// The acceleration for horizontal movement while in the air.
    ///
    /// Set to 0.0 to completely disable air movement.
    pub air_acceleration: Float,

    /// The time, in seconds, the character can still jump after losing their footing.
    pub coyote_time: Float,

    /// Extra gravity for free fall (fall that's not initiated by a jump or some other action that
    /// provides its own fall gravity)
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    ///
    /// **NOTE**: If the parameter set to this option is too low, the character may be able to run
    /// up a slope and "jump" potentially even higher than a regular jump, even without pressing
    /// the jump button.
    pub free_fall_extra_gravity: Float,

    /// The maximum angular velocity used for keeping the character standing upright.
    ///
    /// NOTE: The character's rotation can also be locked to prevent it from being tilted, in which
    /// case this paramter is redundant and can be set to 0.0.
    pub tilt_offset_angvel: Float,

    /// The maximum angular acceleration used for reaching `tilt_offset_angvel`.
    ///
    /// NOTE: The character's rotation can also be locked to prevent it from being tilted, in which
    /// case this paramter is redundant and can be set to 0.0.
    pub tilt_offset_angacl: Float,

    /// The maximum angular velocity used for turning the character when the direction changes.
    pub turning_angvel: Float,

    /// The maximum slope, in radians, that the character can stand on without slipping.
    pub max_slope: Float,
}

/// Definition for an upward-facing proximity sensor that checks for obstacles above the
/// character's "head".
#[derive(Clone, Serialize, Deserialize)]
pub struct TnuaBuiltinWalkHeadroom {
    /// Disnce from the collider's center to its top.
    pub distance_to_collider_top: Float,

    /// Extra distance, from the top of the collider, for the sensor to cover.
    ///
    /// Set this slightly higher than zero. Actions that rely on the headroom sensor will want
    /// to add their own extra distance anyway by using
    /// [`set_extra_headroom`](TnuaBasisWithHeadroom::set_extra_headroom).
    pub sensor_extra_distance: Float,
}

impl Default for TnuaBuiltinWalkHeadroom {
    fn default() -> Self {
        Self {
            distance_to_collider_top: 0.0,
            sensor_extra_distance: 0.1,
        }
    }
}

impl Default for TnuaBuiltinWalkConfig {
    fn default() -> Self {
        Self {
            speed: 10.0,
            float_height: 0.0,
            headroom: None,
            cling_distance: 1.0,
            spring_strength: 400.0,
            spring_dampening: 1.2,
            acceleration: 60.0,
            air_acceleration: 20.0,
            coyote_time: 30.15,
            free_fall_extra_gravity: 60.0,
            tilt_offset_angvel: 5.0,
            tilt_offset_angacl: 500.0,
            turning_angvel: 10.0,
            max_slope: float_consts::FRAC_PI_2,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
struct StandingOnState {
    entity: Entity,
    entity_linvel: Vector3,
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct TnuaBuiltinWalkMemory {
    airborne_timer: Option<Timer>,
    /// The current distance of the character from the distance its supposed to float at.
    pub standing_offset: Vector3,
    standing_on: Option<StandingOnState>,
    effective_velocity: Vector3,
    vertical_velocity: Float,
    /// The velocity, perpendicular to the up direction, that the character is supposed to move at.
    ///
    /// If the character is standing on something else
    /// ([`standing_on_entity`](Self::standing_on_entity) returns `Some`) then the
    /// `running_velocity` will be relative to the velocity of that entity.
    pub running_velocity: Vector3,
    extra_headroom: Float,
}

// TODO: move these to a trait?
impl TnuaBuiltinWalkMemory {
    /// Returns the entity that the character currently stands on.
    pub fn standing_on_entity(&self) -> Option<Entity> {
        Some(self.standing_on.as_ref()?.entity)
    }
}

impl TnuaBasis for TnuaBuiltinWalk {
    type Config = TnuaBuiltinWalkConfig;

    type Memory = TnuaBuiltinWalkMemory;

    type Sensors<'a> = TnuaBuiltinWalkSensors<'a>;

    fn apply(
        &self,
        config: &Self::Config,
        memory: &mut Self::Memory,
        sensors: &Self::Sensors<'_>,
        ctx: TnuaBasisContext,
        motor: &mut TnuaMotor,
    ) {
        if let Some(stopwatch) = &mut memory.airborne_timer {
            #[allow(clippy::unnecessary_cast)]
            stopwatch.tick(Duration::from_secs_f64(ctx.frame_duration as f64));
        }

        // Reset this every frame - if there is an action that changes it, it will use
        // `influence_basis` to set it back immediately after.
        memory.extra_headroom = 0.0;

        let climb_vectors: Option<ClimbVectors>;
        let considered_in_air: bool;
        let impulse_to_offset: Vector3;
        let slipping_vector: Option<Vector3>;

        if let Some(sensor_output) = &sensors.ground.output {
            memory.effective_velocity = ctx.tracker.velocity - sensor_output.entity_linvel;
            let sideways_unnormalized = sensor_output
                .normal
                .cross(*ctx.up_direction)
                .adjust_precision();
            if sideways_unnormalized == Vector3::ZERO {
                climb_vectors = None;
            } else {
                climb_vectors = Some(ClimbVectors {
                    direction: sideways_unnormalized
                        .cross(sensor_output.normal.adjust_precision())
                        .normalize_or_zero()
                        .adjust_precision(),
                    sideways: sideways_unnormalized.normalize_or_zero().adjust_precision(),
                });
            }

            slipping_vector = {
                let angle_with_floor = sensor_output
                    .normal
                    .angle_between(*ctx.up_direction)
                    .adjust_precision();
                if angle_with_floor <= config.max_slope {
                    None
                } else {
                    Some(
                        sensor_output
                            .normal
                            .reject_from(*ctx.up_direction)
                            .adjust_precision(),
                    )
                }
            };

            if memory.airborne_timer.is_some() {
                considered_in_air = true;
                impulse_to_offset = Vector3::ZERO;
                memory.standing_on = None;
            } else {
                if let Some(standing_on_state) = &memory.standing_on {
                    if standing_on_state.entity != sensor_output.entity {
                        impulse_to_offset = Vector3::ZERO;
                    } else {
                        impulse_to_offset =
                            sensor_output.entity_linvel - standing_on_state.entity_linvel;
                    }
                } else {
                    impulse_to_offset = Vector3::ZERO;
                }

                if slipping_vector.is_none() {
                    considered_in_air = false;
                    memory.standing_on = Some(StandingOnState {
                        entity: sensor_output.entity,
                        entity_linvel: sensor_output.entity_linvel,
                    });
                } else {
                    considered_in_air = true;
                    memory.standing_on = None;
                }
            }
        } else {
            memory.effective_velocity = ctx.tracker.velocity;
            climb_vectors = None;
            considered_in_air = true;
            impulse_to_offset = Vector3::ZERO;
            slipping_vector = None;
            memory.standing_on = None;
        }
        memory.effective_velocity += impulse_to_offset;

        let velocity_on_plane = memory
            .effective_velocity
            .reject_from(ctx.up_direction.adjust_precision());

        let desired_velocity = self.desired_motion * config.speed;

        let desired_boost = desired_velocity - velocity_on_plane;

        let safe_direction_coefficient = desired_velocity
            .normalize_or_zero()
            .dot(velocity_on_plane.normalize_or_zero());
        let direction_change_factor = 1.5 - 0.5 * safe_direction_coefficient;

        let relevant_acceleration_limit = if considered_in_air {
            config.air_acceleration
        } else {
            config.acceleration
        };
        let max_acceleration = direction_change_factor * relevant_acceleration_limit;

        memory.vertical_velocity = if let Some(climb_vectors) = &climb_vectors {
            memory.effective_velocity.dot(climb_vectors.direction)
                * climb_vectors
                    .direction
                    .dot(ctx.up_direction.adjust_precision())
        } else {
            0.0
        };

        let walk_vel_change = if desired_velocity == Vector3::ZERO && slipping_vector.is_none() {
            // When stopping, prefer a boost to be able to reach a precise stop (see issue #39)
            let walk_boost = desired_boost.clamp_length_max(ctx.frame_duration * max_acceleration);
            let walk_boost = if let Some(climb_vectors) = &climb_vectors {
                climb_vectors.project(walk_boost)
            } else {
                walk_boost
            };
            TnuaVelChange::boost(walk_boost)
        } else {
            // When accelerating, prefer an acceleration because the physics backends treat it
            // better (see issue #34)
            let walk_acceleration =
                (desired_boost / ctx.frame_duration).clamp_length_max(max_acceleration);
            let walk_acceleration =
                if let (Some(climb_vectors), None) = (&climb_vectors, slipping_vector) {
                    climb_vectors.project(walk_acceleration)
                } else {
                    walk_acceleration
                };

            let slipping_boost = 'slipping_boost: {
                let Some(slipping_vector) = slipping_vector else {
                    break 'slipping_boost Vector3::ZERO;
                };
                let vertical_velocity = if 0.0 <= memory.vertical_velocity {
                    ctx.tracker.gravity.dot(ctx.up_direction.adjust_precision())
                        * ctx.frame_duration
                } else {
                    memory.vertical_velocity
                };

                let Ok((slipping_direction, slipping_per_vertical_unit)) =
                    Dir3::new_and_length(slipping_vector.f32())
                else {
                    break 'slipping_boost Vector3::ZERO;
                };

                let required_veloicty_in_slipping_direction =
                    slipping_per_vertical_unit.adjust_precision() * -vertical_velocity;
                let expected_velocity = velocity_on_plane + walk_acceleration * ctx.frame_duration;
                let expected_velocity_in_slipping_direction =
                    expected_velocity.dot(slipping_direction.adjust_precision());

                let diff = required_veloicty_in_slipping_direction
                    - expected_velocity_in_slipping_direction;

                if diff <= 0.0 {
                    break 'slipping_boost Vector3::ZERO;
                }

                slipping_direction.adjust_precision() * diff
            };
            TnuaVelChange {
                acceleration: walk_acceleration,
                boost: slipping_boost,
            }
        };

        let upward_impulse: TnuaVelChange = 'upward_impulse: {
            let should_disable_due_to_slipping =
                slipping_vector.is_some() && memory.vertical_velocity <= 0.0;
            for _ in 0..2 {
                #[allow(clippy::unnecessary_cast)]
                match &mut memory.airborne_timer {
                    None => {
                        if let (false, Some(sensor_output)) =
                            (should_disable_due_to_slipping, &sensors.ground.output)
                        {
                            // not doing the jump calculation here
                            let spring_offset =
                                config.float_height - sensor_output.proximity.adjust_precision();
                            memory.standing_offset =
                                -spring_offset * ctx.up_direction.adjust_precision();
                            break 'upward_impulse Self::spring_force(
                                &TnuaBasisAccess {
                                    input: self,
                                    config,
                                    memory,
                                },
                                &ctx,
                                spring_offset,
                            );
                        } else {
                            memory.airborne_timer = Some(Timer::from_seconds(
                                config.coyote_time as f32,
                                TimerMode::Once,
                            ));
                            continue;
                        }
                    }
                    Some(_) => {
                        if let (false, Some(sensor_output)) =
                            (should_disable_due_to_slipping, &sensors.ground.output)
                            && sensor_output.proximity.adjust_precision() <= config.float_height
                        {
                            memory.airborne_timer = None;
                            continue;
                        }
                        if memory.vertical_velocity <= 0.0 {
                            break 'upward_impulse TnuaVelChange::acceleration(
                                -config.free_fall_extra_gravity
                                    * ctx.up_direction.adjust_precision(),
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

        motor.lin = walk_vel_change + TnuaVelChange::boost(impulse_to_offset) + upward_impulse;
        let new_velocity = memory.effective_velocity
            + motor.lin.boost
            + ctx.frame_duration * motor.lin.acceleration
            - impulse_to_offset;
        memory.running_velocity = new_velocity.reject_from(ctx.up_direction.adjust_precision());

        // Tilt

        let torque_to_fix_tilt = {
            let tilted_up = ctx.tracker.rotation.mul_vec3(Vector3::Y);

            let rotation_required_to_fix_tilt =
                Quaternion::from_rotation_arc(tilted_up, ctx.up_direction.adjust_precision());

            let desired_angvel = (rotation_required_to_fix_tilt.xyz() / ctx.frame_duration)
                .clamp_length_max(config.tilt_offset_angvel);
            let angular_velocity_diff = desired_angvel - ctx.tracker.angvel;
            angular_velocity_diff.clamp_length_max(ctx.frame_duration * config.tilt_offset_angacl)
        };

        // Turning

        let desired_angvel = if let Some(desired_forward) = self.desired_forward {
            let current_forward = ctx.tracker.rotation.mul_vec3(Vector3::NEG_Z);
            let rotation_along_up_axis = rotation_arc_around_axis(
                ctx.up_direction,
                current_forward,
                desired_forward.adjust_precision(),
            )
            .unwrap_or(0.0);
            (rotation_along_up_axis / ctx.frame_duration)
                .clamp(-config.turning_angvel, config.turning_angvel)
        } else {
            0.0
        };

        // NOTE: This is the regular axis system so we used the configured up.
        let existing_angvel = ctx.tracker.angvel.dot(ctx.up_direction.adjust_precision());

        // This is the torque. Should it be clamped by an acceleration? From experimenting with
        // this I think it's meaningless and only causes bugs.
        let torque_to_turn = desired_angvel - existing_angvel;

        let existing_turn_torque = torque_to_fix_tilt.dot(ctx.up_direction.adjust_precision());
        let torque_to_turn = torque_to_turn - existing_turn_torque;

        motor.ang = TnuaVelChange::boost(
            torque_to_fix_tilt + torque_to_turn * ctx.up_direction.adjust_precision(),
        );
    }

    fn get_or_create_sensors<'a: 'b, 'b>(
        up_direction: Dir3,
        config: &'a Self::Config,
        memory: &Self::Memory,
        entities: &'a mut <Self::Sensors<'static> as TnuaSensors<'static>>::Entities,
        proximity_sensors_query: &'b Query<(&TnuaProximitySensor, Has<TnuaGhostSensor>)>,
        controller_entity: Entity,
        commands: &mut Commands,
        has_ghost_overwrites: bool,
    ) -> Option<Self::Sensors<'b>> {
        let ground = ProximitySensorPreparationHelper {
            cast_direction: -up_direction,
            cast_range: config.float_height + config.cling_distance,
            ghost_sensor: has_ghost_overwrites,
            ..Default::default()
        }
        .prepare_for(
            &mut entities.ground,
            proximity_sensors_query,
            controller_entity,
            commands,
        );

        let headroom = if let Some(headroom) = config.headroom.as_ref() {
            ProximitySensorPreparationHelper {
                cast_direction: up_direction,
                cast_range: headroom.distance_to_collider_top
                    + headroom.sensor_extra_distance
                    + memory.extra_headroom,
                ..Default::default()
            }
            .prepare_for(
                &mut entities.headroom,
                proximity_sensors_query,
                controller_entity,
                commands,
            )
        } else {
            ProximitySensorPreparationHelper::ensure_not_existing(
                &mut entities.headroom,
                proximity_sensors_query,
                commands,
            )
        };
        // .prepare_for(

        Some(Self::Sensors {
            ground: ground?,
            headroom,
        })
    }

    fn ghost_sensor_overwrites<'a>(
        ghost_overwrites: &'a mut <Self::Sensors<'static> as TnuaSensors<'static>>::GhostOverwrites,
        entities: &<Self::Sensors<'static> as TnuaSensors<'static>>::Entities,
    ) -> impl Iterator<Item = (&'a mut TnuaGhostOverwrite, Entity)> {
        [(&mut ghost_overwrites.ground, entities.ground)]
            .into_iter()
            .flat_map(|(o, e)| Some((o, e?)))
    }
}

impl TnuaBasisWithFrameOfReferenceSurface for TnuaBuiltinWalk {
    fn effective_velocity(access: &TnuaBasisAccess<Self>) -> Vector3 {
        access.memory.effective_velocity
    }

    fn vertical_velocity(access: &TnuaBasisAccess<Self>) -> Float {
        access.memory.vertical_velocity
    }
}
impl TnuaBasisWithDisplacement for TnuaBuiltinWalk {
    fn displacement(access: &TnuaBasisAccess<Self>) -> Option<Vector3> {
        match access.memory.airborne_timer {
            None => Some(access.memory.standing_offset),
            Some(_) => None,
        }
    }
}
impl TnuaBasisWithGround for TnuaBuiltinWalk {
    fn is_airborne(access: &TnuaBasisAccess<Self>) -> bool {
        access
            .memory
            .airborne_timer
            .as_ref()
            .is_some_and(|timer| timer.is_finished())
    }

    fn violate_coyote_time(memory: &mut Self::Memory) {
        if let Some(timer) = &mut memory.airborne_timer {
            timer.set_duration(Duration::ZERO);
        }
    }

    fn ground_sensor<'a>(sensors: &Self::Sensors<'a>) -> &'a TnuaProximitySensor {
        sensors.ground
    }
}
impl TnuaBasisWithHeadroom for TnuaBuiltinWalk {
    fn headroom_intrusion<'a>(
        access: &TnuaBasisAccess<Self>,
        sensors: &Self::Sensors<'a>,
    ) -> Option<std::ops::Range<Float>> {
        let headroom_config = access.config.headroom.as_ref()?;
        let headroom_sensor_output = sensors.headroom?.output.as_ref()?;
        Some(headroom_config.distance_to_collider_top..headroom_sensor_output.proximity)
    }

    fn set_extra_headroom(memory: &mut Self::Memory, extra_headroom: Float) {
        memory.extra_headroom = extra_headroom.max(0.0);
    }
}
impl TnuaBasisWithFloating for TnuaBuiltinWalk {
    fn float_height(access: &TnuaBasisAccess<Self>) -> Float {
        access.config.float_height
    }
}
impl TnuaBasisWithSpring for TnuaBuiltinWalk {
    fn spring_force(
        access: &TnuaBasisAccess<Self>,
        ctx: &TnuaBasisContext,
        spring_offset: Float,
    ) -> TnuaVelChange {
        let spring_force: Float = spring_offset * access.config.spring_strength;

        let relative_velocity = access
            .memory
            .effective_velocity
            .dot(ctx.up_direction.adjust_precision())
            - access.memory.vertical_velocity;

        let gravity_compensation = -ctx.tracker.gravity;

        let dampening_boost = relative_velocity * access.config.spring_dampening;

        TnuaVelChange {
            acceleration: ctx.up_direction.adjust_precision() * spring_force + gravity_compensation,
            boost: ctx.up_direction.adjust_precision() * -dampening_boost,
        }
    }
}

#[derive(Debug, Clone)]
struct ClimbVectors {
    direction: Vector3,
    sideways: Vector3,
}

impl ClimbVectors {
    fn project(&self, vector: Vector3) -> Vector3 {
        let axis_direction = vector.dot(self.direction) * self.direction;
        let axis_sideways = vector.dot(self.sideways) * self.sideways;
        axis_direction + axis_sideways
    }
}
