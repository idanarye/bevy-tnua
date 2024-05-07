use std::time::Duration;

use crate::math::{AdjustPrecision, Float, Quaternion, Vector3};
use bevy::prelude::*;

use crate::util::rotation_arc_around_axis;
use crate::TnuaBasisContext;
use crate::{TnuaBasis, TnuaVelChange};

/// The most common [basis](TnuaBasis) - walk around as a floating capsule.
///
/// This basis implements the floating capsule character controller explained in
/// <https://youtu.be/qdskE8PJy6Q>. It controls both the floating and the movement. Most of its
/// fields have sane defaults, except:
///
/// * [`float_height`](Self::float_height) - this field defaults to 0.0, which means the character
///   will not float. Set it to be higher than the distance from the center of the entity to the
///   bottom of the collider.
/// * [`desired_velocity`](Self::desired_velocity) - while leaving this as as the default
///   `Vector3::ZERO`, doing so would mean that the character will not move.
/// * [`desired_forward`](Self::desired_forward) - leaving this is the default `Vector3::ZERO` will
///   mean that Tnua will not attempt to fix the character's rotation along the [up](Self::up)
///   axis.
///
///   This is fine if rotation along the up axis is locked (Rapier only supports locking cardinal
///   axes, but [`up`](Self::up) defaults to `Vector3::Y` which fits the bill).
///
///   This is also fine for 2D games (or games with 3D graphics and 2D physics) played from side
///   view where the physics engine cannot rotate the character along the up axis.
///
///   But if the physics engine is free to rotate the character's rigid body along the up axis,
///   leaving `desired_forward` as the default `Vector3::ZERO` may cause the character to spin
///   uncontrollably when it contacts other colliders. Unless, of course, some other mechanism
///   prevents that.
#[derive(Clone)]
pub struct TnuaBuiltinWalk {
    /// The direction (in the world space) and speed to accelerate to.
    ///
    /// Tnua assumes that this vector is orthogonal to the [`up`](Self::up) vector.
    pub desired_velocity: Vector3,

    /// If non-zero, Tnua will rotate the character so that its negative Z will face in that
    /// direction.
    ///
    /// Tnua assumes that this vector is orthogonal to the [`up`](Self::up) vector.
    pub desired_forward: Vector3,

    /// The height at which the character will float above ground at rest.
    ///
    /// Note that this is the height of the character's center of mass - not the distance from its
    /// collision mesh.
    ///
    /// To make a character crouch, instead of altering this field, prefer to use the
    /// [`TnuaBuiltinCrouch`](crate::builtins::TnuaBuiltinCrouch) action.
    pub float_height: Float,

    /// Extra distance above the `float_height` where the spring is still in effect.
    ///
    /// When the character is at at most this distance above the
    /// [`float_height`](Self::float_height), the spring force will kick in and move it to the
    /// float height - even if that means pushing it down. If the character is above that distance
    /// above the `float_height`, Tnua will consider it to be in the air.
    pub cling_distance: Float,

    /// The direction considered as upward.
    ///
    /// Typically `Vector3::Y`.
    pub up: Direction3d,

    /// The force that pushes the character to the float height.
    ///
    /// The actual force applied is in direct linear relationship to the displacement from the
    /// `float_height`.
    pub spring_strengh: Float,

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
}

impl Default for TnuaBuiltinWalk {
    fn default() -> Self {
        Self {
            desired_velocity: Vector3::ZERO,
            desired_forward: Vector3::ZERO,
            float_height: 0.0,
            cling_distance: 1.0,
            up: Direction3d::Y,
            spring_strengh: 400.0,
            spring_dampening: 1.2,
            acceleration: 60.0,
            air_acceleration: 20.0,
            coyote_time: 0.15,
            free_fall_extra_gravity: 60.0,
            tilt_offset_angvel: 5.0,
            tilt_offset_angacl: 500.0,
            turning_angvel: 10.0,
        }
    }
}

impl TnuaBasis for TnuaBuiltinWalk {
    const NAME: &'static str = "TnuaBuiltinWalk";
    type State = TnuaBuiltinWalkState;

    fn apply(&self, state: &mut Self::State, ctx: TnuaBasisContext, motor: &mut crate::TnuaMotor) {
        if let Some(stopwatch) = &mut state.airborne_timer {
            #[allow(clippy::unnecessary_cast)]
            stopwatch.tick(Duration::from_secs_f64(ctx.frame_duration as f64));
        }

        let climb_vectors: Option<ClimbVectors>;
        let considered_in_air: bool;
        let impulse_to_offset: Vector3;

        if let Some(sensor_output) = &ctx.proximity_sensor.output {
            state.effective_velocity = ctx.tracker.velocity - sensor_output.entity_linvel;
            let sideways_unnormalized = sensor_output.normal.cross(*self.up).adjust_precision();
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
            considered_in_air = state.airborne_timer.is_some();
            if considered_in_air {
                impulse_to_offset = Vector3::ZERO;
                state.standing_on = None;
            } else {
                if let Some(standing_on_state) = &state.standing_on {
                    if standing_on_state.entity != sensor_output.entity {
                        impulse_to_offset = Vector3::ZERO;
                    } else {
                        impulse_to_offset =
                            sensor_output.entity_linvel - standing_on_state.entity_linvel;
                    }
                } else {
                    impulse_to_offset = Vector3::ZERO;
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
            impulse_to_offset = Vector3::ZERO;
            state.standing_on = None;
        }
        state.effective_velocity += impulse_to_offset;

        let velocity_on_plane = state
            .effective_velocity
            .reject_from(self.up.adjust_precision());

        let desired_boost = self.desired_velocity - velocity_on_plane;

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
        let max_acceleration = direction_change_factor * relevant_acceleration_limit;

        let walk_vel_change = if self.desired_velocity == Vector3::ZERO {
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
            let walk_acceleration = if let Some(climb_vectors) = &climb_vectors {
                climb_vectors.project(walk_acceleration)
            } else {
                walk_acceleration
            };
            TnuaVelChange::acceleration(walk_acceleration)
        };

        state.vertical_velocity = if let Some(climb_vectors) = &climb_vectors {
            state.effective_velocity.dot(climb_vectors.direction)
                * climb_vectors.direction.dot(self.up.adjust_precision())
        } else {
            0.0
        };

        let upward_impulse: TnuaVelChange = 'upward_impulse: {
            for _ in 0..2 {
                #[allow(clippy::unnecessary_cast)]
                match &mut state.airborne_timer {
                    None => {
                        if let Some(sensor_output) = &ctx.proximity_sensor.output {
                            // not doing the jump calculation here
                            let spring_offset =
                                self.float_height - sensor_output.proximity.adjust_precision();
                            state.standing_offset = -spring_offset;
                            let boost = self.spring_force_boost(state, &ctx, spring_offset);
                            break 'upward_impulse TnuaVelChange::boost(
                                boost * self.up.adjust_precision(),
                            );
                        } else {
                            state.airborne_timer = Some(Timer::from_seconds(
                                self.coyote_time as f32,
                                TimerMode::Once,
                            ));
                            continue;
                        }
                    }
                    Some(_) => {
                        if let Some(sensor_output) = &ctx.proximity_sensor.output {
                            if sensor_output.proximity.adjust_precision() <= self.float_height {
                                state.airborne_timer = None;
                                continue;
                            }
                        }
                        if state.vertical_velocity <= 0.0 {
                            break 'upward_impulse TnuaVelChange::acceleration(
                                -self.free_fall_extra_gravity * self.up.adjust_precision(),
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
        let new_velocity = state.effective_velocity
            + motor.lin.boost
            + ctx.frame_duration * motor.lin.acceleration
            - impulse_to_offset;
        state.running_velocity = new_velocity.reject_from(self.up.adjust_precision());

        // Tilt

        let torque_to_fix_tilt = {
            let tilted_up = ctx.tracker.rotation.mul_vec3(self.up.adjust_precision());

            let rotation_required_to_fix_tilt =
                Quaternion::from_rotation_arc(tilted_up, self.up.adjust_precision());

            let desired_angvel = (rotation_required_to_fix_tilt.xyz() / ctx.frame_duration)
                .clamp_length_max(self.tilt_offset_angvel);
            let angular_velocity_diff = desired_angvel - ctx.tracker.angvel;
            angular_velocity_diff.clamp_length_max(ctx.frame_duration * self.tilt_offset_angacl)
        };

        // Turning

        let desired_angvel = if 0.0 < self.desired_forward.length_squared() {
            let current_forward = ctx.tracker.rotation.mul_vec3(Vector3::NEG_Z);
            let rotation_along_up_axis =
                rotation_arc_around_axis(Direction3d::Y, current_forward, self.desired_forward)
                    .unwrap_or(0.0);
            (rotation_along_up_axis / ctx.frame_duration)
                .clamp(-self.turning_angvel, self.turning_angvel)
        } else {
            0.0
        };

        // NOTE: This is the regular axis system so we used the configured up.
        let existing_angvel = ctx.tracker.angvel.dot(self.up.adjust_precision());

        // This is the torque. Should it be clamped by an acceleration? From experimenting with
        // this I think it's meaningless and only causes bugs.
        let torque_to_turn = desired_angvel - existing_angvel;

        let existing_turn_torque = torque_to_fix_tilt.dot(self.up.adjust_precision());
        let torque_to_turn = torque_to_turn - existing_turn_torque;

        motor.ang =
            TnuaVelChange::boost(torque_to_fix_tilt + torque_to_turn * self.up.adjust_precision());
    }

    fn proximity_sensor_cast_range(&self, _state: &Self::State) -> Float {
        self.float_height + self.cling_distance
    }

    fn up_direction(&self, _state: &Self::State) -> Direction3d {
        self.up
    }

    fn displacement(&self, state: &Self::State) -> Option<Vector3> {
        match state.airborne_timer {
            None => Some(self.up.adjust_precision() * state.standing_offset),
            Some(_) => None,
        }
    }

    fn effective_velocity(&self, state: &Self::State) -> Vector3 {
        state.effective_velocity
    }

    fn vertical_velocity(&self, state: &Self::State) -> Float {
        state.vertical_velocity
    }

    fn neutralize(&mut self) {
        self.desired_velocity = Vector3::ZERO;
        self.desired_forward = Vector3::ZERO;
    }

    fn is_airborne(&self, state: &Self::State) -> bool {
        state
            .airborne_timer
            .as_ref()
            .is_some_and(|timer| timer.finished())
    }

    fn violate_coyote_time(&self, state: &mut Self::State) {
        if let Some(timer) = &mut state.airborne_timer {
            timer.set_duration(Duration::ZERO);
        }
    }
}

impl TnuaBuiltinWalk {
    // TODO: maybe this needs to be an acceleration rather than an
    // impulse? The problem is the comparison between `spring_impulse`
    // and `offset_change_impulse`...

    /// Calculate the vertical spring force that this basis would need to apply assuming its
    /// vertical distance from the vertical distance it needs to be at equals the `spring_offset`
    /// argument.
    ///
    /// Note: this is exposed so that actions like
    /// [`TnuaBuiltinCrouch`](crate::builtins::TnuaBuiltinCrouch) may rely on it.
    pub fn spring_force_boost(
        &self,
        state: &TnuaBuiltinWalkState,
        ctx: &TnuaBasisContext,
        spring_offset: Float,
    ) -> Float {
        let spring_force: Float = spring_offset * self.spring_strengh;

        let relative_velocity =
            state.effective_velocity.dot(self.up.adjust_precision()) - state.vertical_velocity;

        let dampening_force = relative_velocity * self.spring_dampening / ctx.frame_duration;
        let spring_force = spring_force - dampening_force;

        let gravity_compensation = -ctx.tracker.gravity.dot(self.up.adjust_precision());

        ctx.frame_duration * (spring_force + gravity_compensation)
    }
}

#[derive(Debug)]
struct StandingOnState {
    entity: Entity,
    entity_linvel: Vector3,
}

#[derive(Default)]
pub struct TnuaBuiltinWalkState {
    airborne_timer: Option<Timer>,
    /// The current vertical distance of the character from the distance its supposed to float at.
    pub standing_offset: Float,
    standing_on: Option<StandingOnState>,
    effective_velocity: Vector3,
    vertical_velocity: Float,
    /// The velocity, perpendicular to the [up](TnuaBuiltinWalk::up) axis, that the character is
    /// supposed to move at.
    ///
    /// If the character is standing on something else
    /// ([`standing_on_entity`](Self::standing_on_entity) returns `Some`) then the
    /// `running_velocity` will be relative to the velocity of that entity.
    pub running_velocity: Vector3,
}

impl TnuaBuiltinWalkState {
    /// Returns the entity that the character currently stands on.
    pub fn standing_on_entity(&self) -> Option<Entity> {
        Some(self.standing_on.as_ref()?.entity)
    }
}

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
